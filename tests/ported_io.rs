use schema_rs::io::{decode_osc, decode_wav, resource_path};
#[test]
fn osc_parses_typed_messages_and_rejects_truncation() {
    let packet = b"/fader\0\0,if\0\0\0\0\x07\x3f\x40\0\0";
    let event = decode_osc(packet).unwrap();
    assert_eq!(
        serde_json::to_value(event).unwrap(),
        serde_json::json!({"type":"osc","address":"/fader","arguments":[7,0.75]})
    );
    assert!(decode_osc(&packet[..packet.len() - 1]).is_err());
}
#[test]
fn wav_decodes_pcm_and_resource_paths_stay_in_project() {
    let mut bytes = b"RIFF".to_vec();
    bytes.extend(40u32.to_le_bytes());
    bytes.extend(b"WAVEfmt ");
    bytes.extend(16u32.to_le_bytes());
    bytes.extend(1u16.to_le_bytes());
    bytes.extend(1u16.to_le_bytes());
    bytes.extend(48000u32.to_le_bytes());
    bytes.extend(96000u32.to_le_bytes());
    bytes.extend(2u16.to_le_bytes());
    bytes.extend(16u16.to_le_bytes());
    bytes.extend(b"data");
    bytes.extend(4u32.to_le_bytes());
    bytes.extend(16384i16.to_le_bytes());
    bytes.extend((-16384i16).to_le_bytes());
    let audio = decode_wav(&bytes).unwrap();
    assert_eq!(audio.samples, vec![0.5, -0.5]);
    assert_eq!(audio.sample_rate, 48000);
    assert!(resource_path(std::path::Path::new("."), "../secret").is_err());
    assert!(resource_path(std::path::Path::new("."), "C:/secret").is_err());
}
#[tokio::test]
async fn osc_udp_worker_delivers_messages_to_the_running_engine() {
    use schema_rs::{engine::Engine, io::Services, model::Project, scenic, server::Runtime};
    let mut p = Project::demo();
    p.content.get_mut("fixture-1").unwrap()[0] = Some(
        scenic::parse("name: OSC\nstack:\n  Create.Value.OSC: {Name: /fader, Index: 1}").unwrap(),
    );
    let runtime = Runtime::start_engine(Engine::new(p).unwrap(), None).unwrap();
    let socket = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
    let address = socket.local_addr().unwrap();
    drop(socket);
    let services = Services::start(
        runtime.controller.clone(),
        std::env::temp_dir(),
        Some(address),
    )
    .unwrap();
    let sender = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
    sender
        .send_to(b"/fader\0\0,if\0\0\0\0\x07\x3f\x40\0\0", address)
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    loop {
        if runtime.controller.current().snapshot.fixtures[0]
            .result
            .value
            == Some(0.75)
        {
            break;
        }
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert_eq!(runtime.controller.current().snapshot.revision, 0);
    drop(services);
}

#[tokio::test]
async fn saved_versioned_content_refreshes_provider_cache_and_survives_restart() {
    use schema_rs::{
        engine::Engine,
        io::Services,
        model::{ContentEntry, Project},
        project,
        server::Runtime,
    };
    let root = std::env::temp_dir().join(format!("schema-content-watch-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    project::initialize(&root).unwrap();
    let mut p = Project::demo();
    p.output.enabled = false;
    let runtime = Runtime::start_engine(Engine::new(p.clone()).unwrap(), None).unwrap();
    let entries = vec![ContentEntry {
        fixture: p.fixtures[0].id.clone(),
        layer: 0,
        stack: p.content[&p.fixtures[0].id][0].clone(),
    }];
    project::save_content(&root, &entries, "saved").unwrap();
    for pass in 0..2 {
        let services = Services::start(runtime.controller.clone(), root.clone(), None).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(4);
        loop {
            let session = runtime.controller.checkpoint().await.unwrap();
            if session
                .inputs
                .content
                .get("saved")
                .is_some_and(|v| v[0].layer == pass)
            {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "saved v2 content was not registered/updated"
            );
            tokio::time::sleep(std::time::Duration::from_millis(30)).await;
        }
        if pass == 0 {
            let mut changed = entries.clone();
            changed[0].layer = 1;
            project::save_content(&root, &changed, "saved").unwrap();
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(4);
            loop {
                if runtime
                    .controller
                    .checkpoint()
                    .await
                    .unwrap()
                    .inputs
                    .content["saved"][0]
                    .layer
                    == 1
                {
                    break;
                }
                assert!(
                    std::time::Instant::now() < deadline,
                    "v2 cache did not refresh"
                );
                tokio::time::sleep(std::time::Duration::from_millis(30)).await;
            }
        }
        drop(services);
    }
    drop(runtime);
    std::fs::remove_dir_all(root).unwrap();
}
