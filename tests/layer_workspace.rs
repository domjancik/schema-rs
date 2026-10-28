use schema_rs::{
    engine::{Command, Engine},
    model::Project,
    project, scenic,
};
use serde_json::json;
fn edit(engine: &mut Engine, value: serde_json::Value) {
    engine
        .apply(serde_json::from_value::<Command>(value).unwrap())
        .unwrap();
}
#[test]
fn extra_layers_and_mutes_are_independent_undoable_and_persisted() {
    let mut engine = Engine::new(Project::demo()).unwrap();
    edit(
        &mut engine,
        json!({"type":"add_layer","fixture":"fixture-1"}),
    );
    assert_eq!(engine.project().content["fixture-1"].len(), 4);
    assert_eq!(engine.project().content["fixture-2"].len(), 3);
    let value =
        scenic::parse("name: Extra\nstack:\n  Create.Value.Constant: {Value: 0.6}\n").unwrap();
    engine
        .apply(Command::ReplaceLayer {
            fixture: "fixture-1".into(),
            layer: 3,
            stack: Some(value),
        })
        .unwrap();
    engine.tick(0.1);
    assert_eq!(engine.snapshot().fixtures[0].layers[3].value, Some(0.6));
    edit(
        &mut engine,
        json!({"type":"set_layer_enabled","fixture":"fixture-1","layer":3,"enabled":false}),
    );
    engine.tick(0.1);
    assert_eq!(engine.snapshot().fixtures[0].layers[3].value, None);
    assert!(engine.project().content["fixture-1"][3].is_some());
    let yaml = scenic::encode(engine.project().content["fixture-1"][3].as_ref().unwrap()).unwrap();
    assert!(!scenic::parse(&yaml).unwrap().enabled);
    let saved = engine.checkpoint();
    let resumed = Engine::from_session(saved).unwrap();
    assert_eq!(engine.project(), resumed.project());
    let root = std::env::temp_dir().join(format!("schema-extra-layers-{}", std::process::id()));
    project::save(&root, engine.project(), "layers").unwrap();
    let loaded = project::load(&root, "layers").unwrap();
    assert_eq!(loaded.content, engine.project().content);
    std::fs::remove_dir_all(root).unwrap();
    engine.apply(Command::Undo).unwrap();
    engine.tick(0.1);
    assert_eq!(engine.snapshot().fixtures[0].layers[3].value, Some(0.6));
}
#[test]
fn director_layers_are_dynamic_and_invalid_indices_do_not_panic() {
    let mut engine = Engine::new(Project::demo()).unwrap();
    edit(
        &mut engine,
        json!({"type":"add_layer","fixture":"__director"}),
    );
    engine.tick(0.1);
    assert_eq!(engine.snapshot().director.len(), 4);
    let result = engine.apply(Command::ApplyContent {
        entries: vec![schema_rs::model::ContentEntry {
            fixture: "fixture-1".into(),
            layer: 99,
            stack: None,
        }],
    });
    assert!(result.is_err());
    for _ in 4..32 {
        edit(
            &mut engine,
            json!({"type":"add_layer","fixture":"__director"}),
        );
    }
    let command =
        serde_json::from_value::<Command>(json!({"type":"add_layer","fixture":"__director"}))
            .unwrap();
    assert!(engine.apply(command).is_err());
}
#[test]
fn serialized_session_preserves_float_bits_in_runtime_inputs() {
    let mut checkpoint = Engine::new(Project::demo()).unwrap().checkpoint();
    let values = (0..16)
        .map(|i| (i as f64 + 0.17) / 17.3)
        .collect::<Vec<_>>();
    std::sync::Arc::make_mut(&mut checkpoint.inputs)
        .audio
        .insert(
            0,
            schema_rs::inputs::Audio {
                sample_rate: 48000,
                bands: values.clone(),
                ..Default::default()
            },
        );
    let bytes = serde_json::to_vec(&checkpoint).unwrap();
    let decoded: schema_rs::session::Session = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(decoded.inputs.audio[&0].bands, values);
}
