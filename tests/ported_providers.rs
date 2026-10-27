use schema_rs::{
    engine::{Command, Engine},
    inputs::{Audio, Event},
    model::Project,
    scenic,
};
fn engine(body: &str) -> Engine {
    let mut p = Project::demo();
    for layers in p.content.values_mut() {
        *layers = vec![None, None, None];
    }
    p.content.get_mut("fixture-1").unwrap()[0] =
        Some(scenic::parse(&format!("name: Input\nstack:\n{body}")).unwrap());
    Engine::new(p).unwrap()
}
fn event(e: &mut Engine, mut event: Event) {
    event.prepare().unwrap();
    e.apply(Command::Input { event }).unwrap();
}
#[test]
fn osc_numeric_and_received_nodes_consume_real_events_once() {
    let mut e = engine("  Create.Value.OSC: {Name: /fader, Index: 1, Default: 0.2}");
    e.tick(0.);
    assert_eq!(e.snapshot().fixtures[0].result.value, Some(0.2));
    event(
        &mut e,
        Event::Osc {
            address: "/fader".into(),
            arguments: vec![serde_json::json!(0), serde_json::json!(0.75)],
        },
    );
    e.tick(0.);
    assert_eq!(e.snapshot().fixtures[0].result.value, Some(0.75));
    let mut received = engine("  Create.Value.OSCReceived: {Name: /trigger}");
    event(
        &mut received,
        Event::Osc {
            address: "/trigger".into(),
            arguments: vec![],
        },
    );
    received.tick(0.);
    assert_eq!(received.snapshot().fixtures[0].result.value, Some(1.));
    received.tick(0.);
    assert_eq!(received.snapshot().fixtures[0].result.value, Some(0.));
}
#[test]
fn controller_keyboard_and_sequence_sources_use_host_context() {
    let mut e = engine("  Create.Value.Control: {Index: 7, Default: 0.1}");
    event(
        &mut e,
        Event::Controller {
            index: 7,
            value: 0.8,
        },
    );
    e.tick(0.);
    assert_eq!(e.snapshot().fixtures[0].result.value, Some(0.8));
    assert_eq!(e.revision(), 0);
    let mut keyboard = engine("  Create.Value.Keyboard: {Key Name: Space}");
    event(
        &mut keyboard,
        Event::Keyboard {
            keys: vec!["Space".into()],
        },
    );
    keyboard.tick(0.);
    assert_eq!(keyboard.snapshot().fixtures[0].result.value, Some(1.));
    let mut sequence = engine("  Create.Value.SequenceIndex: {Sequence: chase, Group: ''}");
    event(
        &mut sequence,
        Event::Sequence {
            name: "chase".into(),
            fixtures: vec!["fixture-2".into(), "fixture-1".into()],
        },
    );
    sequence.tick(0.);
    assert_eq!(sequence.snapshot().fixtures[0].result.value, Some(0.5));
}
#[test]
fn audio_signal_multiplication_and_meter_are_not_scalar_arithmetic() {
    let mut e=engine("  Convert.Audio.Meter:\n    in:\n      - Combine.Audio.Multiply:\n          in:\n            - Create.Audio.Input: {Index: 0, Gain: 1}\n            - Create.Audio.Input: {Index: 1, Gain: 1}");
    event(
        &mut e,
        Event::Audio {
            slot: 0,
            audio: Audio {
                samples: vec![0.5; 64],
                sample_rate: 48000,
                ..Audio::default()
            },
        },
    );
    event(
        &mut e,
        Event::Audio {
            slot: 1,
            audio: Audio {
                samples: vec![0.4; 64],
                sample_rate: 48000,
                ..Audio::default()
            },
        },
    );
    e.tick(0.);
    let bank = &e.snapshot().fixtures[0].result;
    assert!((bank.audio.as_ref().unwrap().samples[0] - 0.2).abs() < 1e-9);
    assert!((bank.value.unwrap() - 0.2).abs() < 1e-9);
}
#[test]
fn texture_color_and_multiply_produce_distinct_pixel_payloads() {
    let mut e=engine("  Combine.Texture.Multiply:\n    in:\n      - Create.Texture.Color:\n          in:\n            - Create.Color.Color: {Color: '#ff0000'}\n      - Create.Texture.Color:\n          in:\n            - Create.Color.Color: {Color: '#ffffff'}");
    e.tick(0.);
    let bank = &e.snapshot().fixtures[0].result;
    let texture = bank.texture.as_ref().unwrap();
    assert_eq!(texture.width, 1);
    assert_eq!(texture.pixels[0], [1., 0., 0., 1.]);
}
#[test]
fn invalid_pixels_and_unbounded_input_registries_are_rejected_atomically() {
    let mut e = engine("  Create.Value.Global: {Name: test}");
    assert!(e
        .apply(Command::Input {
            event: Event::Pixels {
                fixture: "fixture-1".into(),
                positions: vec![]
            }
        })
        .is_err());
    for index in 0..256 {
        event(
            &mut e,
            Event::Value {
                name: format!("value-{index}"),
                value: index as f64,
            },
        );
    }
    assert!(e
        .apply(Command::Input {
            event: Event::Value {
                name: "overflow".into(),
                value: 1.
            }
        })
        .is_err());
    assert_eq!(e.revision(), 0);
    e.checkpoint().validate().unwrap();
}
