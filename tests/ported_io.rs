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
