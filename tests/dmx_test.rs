use schema_rs::{
    dmx_test::{apply_at, update_at, Lease},
    engine::{Command, Engine},
    inputs::Event,
    model::Project,
};
use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};
#[test]
fn lease_is_bounded_exclusive_and_expires() {
    let now = Instant::now();
    let mut lease: Option<Lease> = None;
    update_at(
        &mut lease,
        "panel-a".into(),
        7,
        [(1, 128), (512, 255)].into(),
        now,
    )
    .unwrap();
    let mut frames = BTreeMap::new();
    assert!(apply_at(&lease, &mut frames, false, now).is_some());
    assert_eq!(frames[&7][0], 128);
    assert_eq!(frames[&7][511], 255);
    assert!(update_at(&mut lease, "panel-b".into(), 7, [(2, 3)].into(), now).is_err());
    let mut blackout = BTreeMap::new();
    apply_at(&lease, &mut blackout, true, now);
    assert_eq!(blackout[&7][0], 0);
    assert!(apply_at(&lease, &mut frames, false, now + Duration::from_secs(6)).is_none());
    update_at(
        &mut lease,
        "panel-b".into(),
        0,
        [(1, 10)].into(),
        now + Duration::from_secs(6),
    )
    .unwrap();
    update_at(
        &mut lease,
        "panel-b".into(),
        0,
        BTreeMap::new(),
        now + Duration::from_secs(6),
    )
    .unwrap();
    assert!(lease.is_none());
}
#[test]
fn transient_override_does_not_edit_project_history_or_persist() {
    let mut e = Engine::new(Project::demo()).unwrap();
    e.tick(0.01);
    let before = e.checkpoint();
    e.apply(Command::Input {
        event: Event::ChannelTest {
            owner: "test-panel".into(),
            universe: 9,
            channels: [(2, 120)].into(),
        },
    })
    .unwrap();
    e.tick(0.01);
    assert_eq!(e.snapshot().universes[&9][1], 120);
    let after = e.checkpoint();
    assert_eq!(after.project, before.project);
    assert_eq!(after.undo, before.undo);
    assert_eq!(after.redo, before.redo);
    assert_eq!(after.snapshot.revision, before.snapshot.revision);
    let serialized = serde_json::to_value(&after.inputs).unwrap();
    assert!(serialized.get("channel_test").is_none());
    let recovered: schema_rs::session::Session =
        serde_json::from_slice(&serde_json::to_vec(&after).unwrap()).unwrap();
    let mut restored = Engine::from_session(recovered).unwrap();
    restored.tick(0.0);
    assert!(restored.snapshot().channel_test.is_none());
    assert!(!restored.snapshot().universes.contains_key(&9));
}
#[test]
fn malformed_channels_are_rejected() {
    let now = Instant::now();
    for channels in [
        vec![(0, 5)],
        vec![(513, 5)],
        (1..=33).map(|n| (n, 0)).collect(),
    ] {
        assert!(update_at(
            &mut None,
            "a".into(),
            0,
            channels.into_iter().collect(),
            now
        )
        .is_err());
    }
    assert!(update_at(&mut None, "a".into(), 32768, [(1, 0)].into(), now).is_err());
}
#[tokio::test]
async fn output_worker_sends_test_levels_to_loopback_and_release_restores_scene() {
    use schema_rs::server::Runtime;
    let receiver = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
    receiver
        .set_read_timeout(Some(Duration::from_millis(500)))
        .unwrap();
    let mut project = Project::demo();
    project.output.enabled = true;
    project.output.target = receiver.local_addr().unwrap().to_string();
    let runtime = Runtime::start(project).unwrap();
    runtime
        .controller
        .command(
            Command::Input {
                event: Event::ChannelTest {
                    owner: "loopback-test".into(),
                    universe: 0,
                    channels: [(300, 128)].into(),
                },
            },
            None,
        )
        .await
        .unwrap();
    let wait = |value| {
        let until = Instant::now() + Duration::from_secs(2);
        let mut packet = [0; 1024];
        while Instant::now() < until {
            let (n, _) = receiver.recv_from(&mut packet).unwrap();
            if n > 317 && packet[14..16] == [0, 0] && packet[18 + 299] == value {
                return;
            }
        }
        panic!("Expected channel300={value}");
    };
    wait(128);
    runtime
        .controller
        .command(
            Command::Input {
                event: Event::ChannelTest {
                    owner: "loopback-test".into(),
                    universe: 0,
                    channels: BTreeMap::new(),
                },
            },
            None,
        )
        .await
        .unwrap();
    wait(0);
    assert_eq!(runtime.controller.current().snapshot.revision, 0);
    assert!(!runtime.controller.current().snapshot.can_undo);
}

#[test]
fn input_json_round_trip_decodes_numeric_channel_keys() {
    let json = serde_json::json!({"type":"channel_test","owner":"json-test","universe":0,"channels":{"1":128,"512":255}});
    let mut event: Event = serde_json::from_value(json.clone()).unwrap();
    event.prepare().unwrap();
    assert_eq!(serde_json::to_value(event).unwrap(), json);
}
