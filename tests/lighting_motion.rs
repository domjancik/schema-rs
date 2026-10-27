use schema_rs::{
    engine::{Command, Engine},
    model::Project,
    scenic,
};
fn project(programs: &[(&str, &str)]) -> Project {
    let mut p = Project::demo();
    for layers in p.content.values_mut() {
        *layers = vec![None, None, None];
    }
    for (fixture, yaml) in programs {
        p.content.get_mut(*fixture).unwrap()[0] = Some(scenic::parse(yaml).unwrap());
    }
    p
}
fn value(e: &Engine, id: &str) -> f64 {
    e.snapshot()
        .fixtures
        .iter()
        .find(|f| f.fixture.id == id)
        .unwrap()
        .result
        .value
        .unwrap()
}
fn position(e: &Engine) -> [f64; 3] {
    e.snapshot().fixtures[0].result.position.unwrap()
}
#[test]
fn point_light_is_three_dimensional_order_independent_and_uses_same_frame_position() {
    let mut p=project(&[("fixture-1","name: Receive\nstack:\n  Light.Receiver: {}"),("fixture-2","name: Emit\nstack:\n  Light.Emitter:\n    Type: 0\n    Range: 10\n    Falloff: 1\n    Color: '#ff0000'\n    in:\n      - Create.Position.XYZ: {X: 0, Y: 0, Z: 0}")]);
    p.fixtures[0].position = [0., 3., 4.];
    let mut e = Engine::new(p.clone()).unwrap();
    e.tick(0.);
    assert!((value(&e, "fixture-1") - 0.5).abs() < 1e-9);
    assert_eq!(
        e.snapshot().fixtures[0].result.colors.as_ref().unwrap()[0],
        [0.5, 0., 0., 1.]
    );
    p.fixtures.reverse();
    let mut reversed = Engine::new(p).unwrap();
    reversed.tick(0.);
    assert_eq!(value(&reversed, "fixture-1"), 0.5);
    e.apply(Command::SetProperty {
        fixture: "fixture-2".into(),
        layer: 0,
        node: 2,
        key: "Y".into(),
        value: serde_json::json!(3),
    })
    .unwrap();
    e.tick(0.);
    assert!((value(&e, "fixture-1") - 0.6).abs() < 1e-9);
}
#[test]
fn spot_and_plane_volumes_have_direction_boundaries_channels_and_additive_color() {
    for (kind, point, expected) in [
        (2, [0., 0., 5.], 0.5),
        (2, [5., 0., 0.], 0.),
        (2, [0., 0., -5.], 0.),
        (1, [0., 0., 5.], 0.5),
        (1, [0., 0., -5.], 0.),
        (1, [20., 0., 0.], 0.),
    ] {
        let mut p=project(&[("fixture-1","name: R\nstack:\n  Light.Receiver: {}"),("fixture-2",&format!("name: E\nstack:\n  Light.Emitter: {{Type: {kind}, Direction: [0,0,1], Range: 10, Falloff: 1, Width: 2, Height: 2}}"))]);
        p.fixtures[0].position = point;
        p.fixtures[1].position = [0.; 3];
        let mut e = Engine::new(p).unwrap();
        e.tick(0.);
        assert!(
            (value(&e, "fixture-1") - expected).abs() < 1e-9,
            "kind {kind} point {point:?}"
        );
    }
    let mut p = project(&[
        (
            "fixture-1",
            "name: R\nstack:\n  Light.Receiver: {Channel: blue}",
        ),
        (
            "fixture-2",
            "name: E\nstack:\n  Light.Emitter: {Channel: red, Range: 10}",
        ),
    ]);
    p.fixtures[0].position = [0.; 3];
    p.fixtures[1].position = [0.; 3];
    let mut e = Engine::new(p).unwrap();
    e.tick(0.);
    assert_eq!(value(&e, "fixture-1"), 0.);
}
#[test]
fn bezier_has_exact_endpoints_curved_midpoint_and_clock_pause() {
    let yaml="name: Curve\nstack:\n  Create.Position.Bezier: {P0: [0,0,0], P1: [0,4,0], P2: [4,4,0], P3: [4,0,0], Progress: 0.5}";
    let mut e = Engine::new(project(&[("fixture-1", yaml)])).unwrap();
    e.tick(0.);
    assert_eq!(position(&e), [2., 3., 0.]);
    for (progress, expected) in [(0., [0.; 3]), (1., [4., 0., 0.])] {
        e.apply(Command::SetProperty {
            fixture: "fixture-1".into(),
            layer: 0,
            node: 1,
            key: "Progress".into(),
            value: serde_json::json!(progress),
        })
        .unwrap();
        e.tick(0.);
        assert_eq!(position(&e), expected);
    }
    let mut e = Engine::new(project(&[(
        "fixture-1",
        &yaml.replace("Progress: 0.5", "Duration: 1, Loop: false"),
    )]))
    .unwrap();
    for _ in 0..2 {
        e.tick(0.25)
    }
    assert_eq!(position(&e), [2., 3., 0.]);
    e.apply(Command::Pause { paused: true }).unwrap();
    e.tick(0.25);
    assert_eq!(position(&e), [2., 3., 0.]);
    let scalar="name: Easing\nstack:\n  Create.Value.Bezier: {Start: 0, Control1: 0, Control2: 0, End: 1, Progress: 0.5}";
    let mut e = Engine::new(project(&[("fixture-1", scalar)])).unwrap();
    e.tick(0.);
    assert_eq!(value(&e, "fixture-1"), 0.125);
}
#[test]
fn native_position_group_sources_and_random_are_bounded_reproducible_and_pauseable() {
    let mut p = project(&[(
        "fixture-1",
        "name: Group\nstack:\n  Create.Position.Group.XYZ: {Group: front, X: 0, Y: 0.5, Z: 1}",
    )]);
    for (i, f) in p.fixtures.iter_mut().take(4).enumerate() {
        f.position = [i as f64, 2., i as f64 * 2.];
    }
    let mut e = Engine::new(p).unwrap();
    e.tick(0.);
    assert_eq!(position(&e), [0., 2., 6.]);
    let yaml =
        "name: Random\nstack:\n  Create.Value.Random: {Interval: 0.2, Filter Time: 0, Seed: 42}";
    let p = project(&[("fixture-1", yaml)]);
    let mut a = Engine::new(p.clone()).unwrap();
    let mut b = Engine::new(p).unwrap();
    let mut values = vec![];
    for _ in 0..8 {
        a.tick(0.1);
        b.tick(0.1);
        let v = value(&a, "fixture-1");
        assert!((0. ..=1.).contains(&v));
        assert_eq!(v, value(&b, "fixture-1"));
        values.push(v);
    }
    assert!(values.windows(2).any(|v| v[0] != v[1]));
    a.apply(Command::SetProperty {
        fixture: "fixture-1".into(),
        layer: 0,
        node: 1,
        key: "Pause".into(),
        value: serde_json::json!(true),
    })
    .unwrap();
    a.tick(0.1);
    let held = value(&a, "fixture-1");
    for _ in 0..10 {
        a.tick(0.25);
        assert_eq!(value(&a, "fixture-1"), held);
    }
}
#[test]
fn invalid_light_and_animation_settings_are_rejected_before_commit() {
    for body in [
        "Light.Emitter: {Range: 0}",
        "Light.Emitter: {Direction: [0,0,0]}",
        "Light.Emitter: {Angle: 200}",
        "Create.Position.Bezier: {Duration: 0}",
        "Create.Value.Random: {Interval: -1}",
        "Create.Position.Group.Slide: {Speed: 2}",
    ] {
        assert!(
            scenic::parse(&format!("name: Invalid\nstack:\n  {body}")).is_err(),
            "{body}"
        );
    }
}
#[test]
fn closed_gate_suppresses_carried_emitters() {
    let mut p=project(&[("fixture-1","name: R\nstack:\n  Light.Receiver: {}"),("fixture-2","name: Disabled\nstack:\n  Filter.Manual:\n    Open: false\n    in:\n      - Light.Emitter: {Range: 10}")]);
    p.fixtures[0].position = [0.; 3];
    p.fixtures[1].position = [0.; 3];
    let mut e = Engine::new(p).unwrap();
    e.tick(0.);
    assert_eq!(value(&e, "fixture-1"), 0.);
}
#[test]
fn lighting_probe_does_not_advance_random_state_twice() {
    let random =
        "name: Random\nstack:\n  Create.Value.Random: {Interval: 0.1, Filter Time: 0, Seed: 42}";
    let mut a = Engine::new(project(&[("fixture-1", random)])).unwrap();
    let mut b = Engine::new(project(&[
        ("fixture-1", random),
        ("fixture-2", "name: R\nstack:\n  Light.Receiver: {}"),
    ]))
    .unwrap();
    for _ in 0..20 {
        a.tick(0.1);
        b.tick(0.1);
        assert_eq!(value(&a, "fixture-1"), value(&b, "fixture-1"));
    }
}
#[test]
fn lights_add_color_and_animated_range_and_intensity_use_world_units() {
    let mut p=project(&[("fixture-1","name: R\nstack:\n  Light.Receiver: {}"),("fixture-2","name: E\nstack:\n  Light.Emitter:\n    Color: '#ff0000'\n    Falloff: 1\n    Range:\n      Create.Value.Constant: {Value: 5}\n    Intensity:\n      Create.Value.Constant: {Value: 2}"),("fixture-3","name: Blue\nstack:\n  Light.Emitter: {Color: '#0000ff', Range: 5, Falloff: 1}")]);
    p.fixtures[0].position = [0., 2.5, 0.];
    p.fixtures[1].position = [0.; 3];
    p.fixtures[2].position = [0.; 3];
    let mut e = Engine::new(p).unwrap();
    e.tick(0.);
    e.tick(0.);
    let color = e.snapshot().fixtures[0].result.colors.as_ref().unwrap()[0];
    assert_eq!(color, [1., 0., 0.5, 1.]);
    assert_eq!(value(&e, "fixture-1"), 1.5);
}
#[test]
fn source_position_random_trigger_slide_and_group_motion_are_real_behaviors() {
    let mut current = Engine::new(project(&[(
        "fixture-1",
        "name: Current\nstack:\n  Create.Position.Current: {}",
    )]))
    .unwrap();
    current.tick(0.);
    assert_eq!(position(&current), [-3., 0., -1.5]);
    let mut center = Engine::new(project(&[(
        "fixture-1",
        "name: Center\nstack:\n  Create.Position.Group.Center: {Group: front}",
    )]))
    .unwrap();
    center.tick(0.);
    assert_eq!(position(&center), [0., 0., -1.5]);
    let mut slide = Engine::new(project(&[(
        "fixture-1",
        "name: Slide\nstack:\n  Create.Position.Group.Slide: {Group: front, Speed: 1, Axis: 0}",
    )]))
    .unwrap();
    slide.tick(0.);
    assert_eq!(position(&slide)[0], -3.);
    slide.tick(0.25);
    assert_eq!(position(&slide)[0], 0.);
    for behavior in ["Create.Position.Group.Random: {Group: front, Speed: 1, Padding: 0, Seed: 8}","Create.Position.Random: {Minimum: [-3,0,-1.5], Maximum: [3,0,-1.5], Interval: 0.2, Filter Time: 0, Seed: 8}"]{let mut random=Engine::new(project(&[("fixture-1",&format!("name: Random position\nstack:\n  {behavior}"))])).unwrap();let mut xs=vec![];for _ in 0..20{random.tick(0.1);let p=position(&random);assert!((-3. ..=3.).contains(&p[0]));assert_eq!(p[1],0.);assert_eq!(p[2],-1.5);xs.push(p[0]);}assert!(xs.windows(2).any(|p|p[0]!=p[1]));}
    for probability in [0, 1] {
        let mut trigger=Engine::new(project(&[("fixture-1",&format!("name: Trigger\nstack:\n  Modify.Value.RandomizeTrigger:\n    Probability: {probability}\n    in:\n      - Create.Value.Constant: {{Value: 1}}"))])).unwrap();
        trigger.tick(0.);
        assert_eq!(value(&trigger, "fixture-1"), f64::from(probability));
    }
}
#[test]
fn nonfinite_light_energy_is_diagnosed_and_not_published() {
    let mut e=Engine::new(project(&[("fixture-1","name: Overflow\nstack:\n  Light.Emitter:\n    Intensity: 100\n    in:\n      - Script.Value: {Code: '1.0e308'}\n      - Create.Color.Color: {Color: '#ffffff'}")])).unwrap();
    e.tick(0.);
    assert!(e.snapshot().emitters.is_empty());
    assert!(!e.snapshot().diagnostics.is_empty());
}
