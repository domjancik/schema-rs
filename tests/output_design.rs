use schema_rs::{
    engine::{Command, Engine},
    model::{Bank, Project},
    output_design::{DimmingTimeline, ModelDefinition},
};
use serde_json::json;
#[test]
fn circular_day_curve_interpolates_across_midnight() {
    let t:DimmingTimeline=serde_json::from_value(json!({"enabled":true,"master":0.8,"utc_offset_minutes":120,"points":[{"minute":360,"level":0.2},{"minute":1080,"level":1.0}]})).unwrap();
    t.validate().unwrap();
    assert!((t.level(22 * 3600) - 0.48).abs() < 1e-9);
    assert!((t.level(4 * 3600) - 0.16).abs() < 1e-9);
    let mut invalid = t.clone();
    invalid.points[1].minute = 360;
    assert!(invalid.validate().is_err());
}
#[test]
fn uv_cob_reserves_constant_program_and_next_fixture_starts_after_both() {
    let mut p = Project::demo();
    p.fixtures.truncate(2);
    p.content
        .retain(|id, _| p.fixtures.iter().any(|f| &f.id == id));
    p.fixtures[0].dmx_model = Some("uv-cob".into());
    p.fixtures[0].address = 1;
    p.fixtures[1].model = schema_rs::model::FixtureModel::Dimmer;
    p.fixtures[1].address = 3;
    p.validate().unwrap();
    let b = Bank {
        value: Some(0.5),
        ..Bank::default()
    };
    assert_eq!(
        schema_rs::output_design::encode(&p, &p.fixtures[0], &b, false, 1.),
        vec![128, 0]
    );
    assert_eq!(p.footprint(&p.fixtures[0]).unwrap(), 2);
    p.fixtures[1].address = 2;
    assert!(p.validate().is_err());
}
#[test]
fn custom_model_commands_validate_atomically_and_undo() {
    let mut e = Engine::new(Project::demo()).unwrap();
    let model:ModelDefinition=serde_json::from_value(json!({"name":"Moving UV","slots":[{"name":"Intensity","source":"value","minimum":0,"maximum":255},{"name":"Program","source":"constant","constant":0,"minimum":0,"maximum":255}]})).unwrap();
    e.apply(Command::UpsertDmxModel {
        id: "custom-uv".into(),
        model,
    })
    .unwrap();
    assert!(e.project().dmx_models.contains_key("custom-uv"));
    e.apply(Command::Undo).unwrap();
    assert!(!e.project().dmx_models.contains_key("custom-uv"));
    let mut timeline = e.project().dimming.clone();
    timeline.master = 0.3;
    e.apply(Command::SetDimming { timeline }).unwrap();
    assert_eq!(e.project().dimming.master, 0.3);
    e.apply(Command::Undo).unwrap();
    assert_eq!(e.project().dimming.master, 1.);
}
#[test]
fn model_and_timeline_roundtrip_layout_but_content_save_does_not_change_them() {
    let root = std::env::temp_dir().join(format!("schema-output-design-{}", std::process::id()));
    let mut p = Project::demo();
    p.dimming.master = 0.6;
    schema_rs::project::save(&root, &p, "init").unwrap();
    let restored = schema_rs::project::load(&root, "init").unwrap();
    assert_eq!(restored.dmx_models, p.dmx_models);
    assert_eq!(restored.dimming, p.dimming);
    schema_rs::project::save_content(&root, &[], "partial").unwrap();
    assert_eq!(
        schema_rs::project::load(&root, "init")
            .unwrap()
            .dimming
            .master,
        0.6
    );
}

#[test]
fn global_dimming_applies_after_scene_evaluation_and_reserves_program_slot() {
    let mut p = Project::demo();
    p.fixtures.truncate(1);
    p.content.retain(|id, _| id == "fixture-1");
    p.fixtures[0].dmx_model = Some("uv-cob".into());
    p.content.get_mut("fixture-1").unwrap()[0] = Some(
        schema_rs::scenic::parse("name: Full\nstack:\n  Create.Value.Constant: {Value: 1}\n")
            .unwrap(),
    );
    p.dimming.master = 0.5;
    let mut e = Engine::new(p).unwrap();
    e.tick(0.);
    assert_eq!(&e.snapshot().universes[&0][..2], &[128, 0]);
    assert_eq!(e.snapshot().fixtures[0].result.value, Some(1.));
    let before = e.project().clone();
    let mut model = before.dmx_models["uv-cob"].clone();
    model.slots.extend(model.slots.clone());
    let mut invalid = e.project().dimming.clone();
    invalid.utc_offset_minutes = i32::MIN;
    assert!(e.apply(Command::SetDimming { timeline: invalid }).is_err());
    assert_eq!(e.project(), &before);
}

#[test]
fn increasing_shared_model_footprint_or_removing_used_model_is_atomic() {
    let mut p = Project::demo();
    p.fixtures[0].dmx_model = Some("uv-cob".into());
    let mut e = Engine::new(p).unwrap();
    let before = e.project().clone();
    let mut model = e.project().dmx_models["uv-cob"].clone();
    model.slots.extend(model.slots.clone());
    assert!(e
        .apply(Command::UpsertDmxModel {
            id: "uv-cob".into(),
            model
        })
        .is_err());
    assert_eq!(e.project(), &before);
    assert!(e
        .apply(Command::RemoveDmxModel {
            id: "uv-cob".into()
        })
        .is_err());
    assert_eq!(e.project(), &before);
    let mut f = before.fixtures[0].clone();
    f.dmx_model = Some("unknown".into());
    assert!(e.apply(Command::SetFixture { fixture: f }).is_err());
}

#[test]
fn master_and_blackout_scale_intensity_after_slot_ranges_but_preserve_controls() {
    let mut p = Project::demo();
    let model = p.dmx_models.get_mut("uv-cob").unwrap();
    model.slots[0].minimum = 20;
    model.slots[0].maximum = 200;
    model.slots[1].constant = 7;
    let mut fixture = p.fixtures[0].clone();
    fixture.dmx_model = Some("uv-cob".into());
    let b = Bank {
        value: Some(0.5),
        ..Bank::default()
    };
    assert_eq!(
        schema_rs::output_design::encode(&p, &fixture, &b, false, 0.5),
        vec![55, 7]
    );
    assert_eq!(
        schema_rs::output_design::encode(&p, &fixture, &b, true, 1.),
        vec![0, 7]
    );
}
