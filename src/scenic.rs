use crate::model::*;
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashSet};
#[derive(Clone, Debug, serde::Serialize)]
pub struct Behavior {
    #[serde(flatten)]
    pub info: serde_json::Map<String, Value>,
    pub key: &'static str,
    pub defaults: BTreeMap<String, Value>,
    pub extension: bool,
}
pub fn catalog() -> &'static [Behavior] {
    static CATALOG: std::sync::OnceLock<Vec<Behavior>> = std::sync::OnceLock::new();
    CATALOG.get_or_init(build_catalog).as_slice()
}
fn build_catalog() -> Vec<Behavior> {
    let metadata: BTreeMap<String, serde_json::Map<String, Value>> =
        serde_json::from_str(include_str!("../web/block-metadata.json"))
            .expect("valid built-in block metadata");
    let entries: Vec<(&str, Value)> = vec![
        ("Create.Value.Constant", json!({"Value":1.0})),
        ("Create.Value.LFO", json!({"Period":1.0,"Type":0})),
        ("Create.Color.Constant", json!({"Value":1.0})),
        ("Create.Color.Color", json!({"Color":"#ffffff"})),
        ("Create.Layer.Above", json!({"Layer":0})),
        ("Modify.Value.Invert", json!({})),
        ("Modify.Value.Clamp", json!({"Minimum":0.,"Maximum":1.})),
        ("Modify.Value.MapTo", json!({"Minimum":0.,"Maximum":1.})),
        (
            "Modify.Value.MapFromTo",
            json!({"Source Min":0.,"Source Max":1.,"Minimum":0.,"Maximum":1.}),
        ),
        ("Modify.Value.Steps", json!({"Count":4})),
        (
            "Modify.Value.Integrate",
            json!({"Reset":false,"Wrap/Clamp":false}),
        ),
        ("Modify.SampleAndHold", json!({"Sample":false})),
        ("Combine.Add", json!({})),
        ("Combine.Multiply", json!({})),
        ("Combine.Crossfade", json!({"Fader":0.,"Shape":0})),
        ("Filter.Manual", json!({"Open":true})),
        ("Convert.Value.ToColor", json!({})),
        ("Tool.Merge", json!({})),
        ("Modify.Color.Invert", json!({"Value":0.0})),
        ("Modify.Color.Dim", json!({"Value":1.,"Amount":1.})),
        ("Create.Position.Constant", json!({"Position":[0,0,0]})),
        ("Create.Reference.Fixture", json!({"Fixture":"Fixture"})),
        (
            "Create.Value.Distance",
            json!({"Auto/Setup":false,"Fixture":"Fixture"}),
        ),
        (
            "Spatial.Distance",
            json!({"Position":[0,0,0],"Fixture":"","Auto/Setup":false}),
        ),
        ("Spatial.Axis", json!({"Axis":0,"Minimum":-5.,"Maximum":5.})),
        (
            "Light.Emitter",
            json!({"Type": 0, "Intensity": 1.0, "Range": 10.0, "Falloff": 2.0, "Color": "#ffffff", "Direction": [0, -1, 0], "Angle": 60.0, "Softness": 0.2, "Width": 2.0, "Height": 2.0, "Two Sided": false, "Channel": "default", "Enabled": true}),
        ),
        (
            "Light.Receiver",
            json!({"Channel": "default", "Fixture": "", "Gain": 1.0, "Include Self": false}),
        ),
        ("Create.Position.Current", json!({})),
        ("Create.Position.XYZ", json!({"X": 0.0, "Y": 0.0, "Z": 0.0})),
        (
            "Create.Position.Group.Center",
            json!({"Group": "Dancefloor"}),
        ),
        (
            "Create.Position.Group.XYZ",
            json!({"Group": "Group", "X": 0.5, "Y": 0.5, "Z": 0.5, "Padding": 0.0, "Reverse": false}),
        ),
        (
            "Create.Position.Group.Slide",
            json!({"Speed": 0.5, "Speed Audio": 0.0, "Group": "Group", "Axis": 0, "Reverse": false}),
        ),
        (
            "Create.Position.Group.Random",
            json!({"Group": "Dancefloor", "Speed": 0.5, "Speed Audio": 0.0, "Padding": 1.0, "Continuous": true, "Seed": 0}),
        ),
        (
            "Create.Value.Random",
            json!({"Interval": 1.0, "Filter Time": 1.0, "Pause": false, "Randomness": 0.0, "Seed": 0}),
        ),
        (
            "Modify.Value.RandomizeTrigger",
            json!({"Probability": 0.5, "Seed": 0}),
        ),
        (
            "Create.Position.Random",
            json!({"Minimum": [-1, -1, -1], "Maximum": [1, 1, 1], "Interval": 1.0, "Filter Time": 1.0, "Pause": false, "Randomness": 0.0, "Seed": 0}),
        ),
        (
            "Create.Position.Bezier",
            json!({"P0": [-3, 0, 0], "P1": [-1, 2, -3], "P2": [1, 2, 3], "P3": [3, 0, 0], "Duration": 4.0, "Progress": -1.0, "Loop": true, "Ping Pong": false}),
        ),
        (
            "Create.Value.Bezier",
            json!({"Start": 0.0, "Control1": 0.0, "Control2": 1.0, "End": 1.0, "Duration": 4.0, "Progress": -1.0, "Loop": true, "Ping Pong": false}),
        ),
        ("Script.Value", json!({"Code":"input"})),
    ];
    let mut catalog: Vec<Behavior> = entries
        .into_iter()
        .map(|(key, v)| Behavior {
            info: metadata
                .get(key)
                .expect("metadata for every behavior")
                .clone(),
            key,
            defaults: v
                .as_object()
                .unwrap()
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            extension: matches!(
                key,
                "Modify.Color.Dim"
                    | "Spatial.Distance"
                    | "Spatial.Axis"
                    | "Script.Value"
                    | "Light.Emitter"
                    | "Light.Receiver"
                    | "Create.Position.Random"
                    | "Create.Position.Bezier"
                    | "Create.Value.Bezier"
            ),
        })
        .collect();
    for (key, spec) in crate::native::specs() {
        if crate::native::supported(key) && !catalog.iter().any(|b| b.key == key) {
            catalog.push(Behavior {
                key: key.as_str(),
                defaults: spec.defaults.clone(),
                extension: false,
                info: metadata[key].clone(),
            });
        }
    }
    catalog
}
thread_local! {static PLUGINS: std::cell::RefCell<std::sync::Arc<crate::plugins::Registry>> = std::cell::RefCell::new(std::sync::Arc::new(crate::plugins::Registry::default()));}
pub fn active_plugins() -> std::sync::Arc<crate::plugins::Registry> {
    PLUGINS.with(|p| p.borrow().clone())
}
pub fn with_plugins<T>(
    registry: std::sync::Arc<crate::plugins::Registry>,
    f: impl FnOnce() -> T,
) -> T {
    struct Restore(std::sync::Arc<crate::plugins::Registry>);
    impl Drop for Restore {
        fn drop(&mut self) {
            PLUGINS.with(|p| *p.borrow_mut() = self.0.clone());
        }
    }
    let previous = PLUGINS.with(|p| p.replace(registry));
    let _restore = Restore(previous);
    f()
}
pub fn defaults(key: &str) -> Result<BTreeMap<String, Value>> {
    if key == "Plugin.Input" {
        return Ok(BTreeMap::new());
    }
    if let Some(defaults) = active_plugins().defaults(key) {
        return Ok(defaults);
    }
    catalog()
        .iter()
        .find(|b| b.key == key)
        .map(|b| b.defaults.clone())
        .ok_or_else(|| format!("Unsupported behavior: {key}"))
}
pub fn new_node(key: &str, id: u64) -> Result<Node> {
    Ok(Node {
        bypassed: false,
        id,
        behavior: key.into(),
        properties: defaults(key)?,
        children: vec![],
        drivers: BTreeMap::new(),
    })
}
pub fn parse(text: &str) -> Result<Stack> {
    if text.len() > 256 * 1024 {
        return Err("ScenicScript exceeds 256 KiB".into());
    }
    let value: Value = serde_yaml::from_str(text).map_err(|e| format!("YAML: {e}"))?;
    let object = value.as_object().ok_or("Expected a stack envelope")?;
    let name = object
        .get("name")
        .and_then(Value::as_str)
        .ok_or("Missing stack name")?
        .to_owned();
    let mut next = 1;
    let root = parse_node(
        object.get("stack").ok_or("Missing stack declaration")?,
        &mut next,
        0,
        None,
    )?;
    let enabled = object
        .get("enabled")
        .map(|v| v.as_bool().ok_or("enabled must be boolean"))
        .transpose()?
        .unwrap_or(true);
    let stack = Stack {
        name,
        root,
        enabled,
    };
    validate(&stack)?;
    Ok(stack)
}
fn parse_node(v: &Value, next: &mut u64, depth: usize, parent: Option<&str>) -> Result<Node> {
    if depth > 64 || *next > 2048 {
        return Err("Program exceeds 64 levels or 2048 nodes".into());
    }
    if let Some(key) = v.as_str() {
        let node = new_node(key, *next)?;
        *next += 1;
        return Ok(node);
    }
    let map = v.as_object().ok_or("Block declaration must be a map")?;
    if map.len() != 1 {
        return Err("Block declaration must have exactly one behavior key".into());
    }
    let (key, fields) = map.iter().next().unwrap();
    let mut node = new_node(key, *next)?;
    *next += 1;
    if fields.is_null() {
        return Ok(node);
    }
    let fields = fields.as_object().ok_or("Block fields must be a map")?;
    for (k, v) in fields {
        if k == "bypassed" {
            node.bypassed = v.as_bool().ok_or("bypassed must be boolean")?;
            continue;
        }
        if k == "in" {
            for child in v.as_array().ok_or("in must be an ordered sequence")? {
                node.children
                    .push(parse_node(child, next, depth + 1, Some(key))?)
            }
            continue;
        }
        if !node.properties.contains_key(k)
            && !matches!(
                (parent, k.as_str()),
                (Some("Pick.Group"), "Group")
                    | (Some("Pick.Audio.Power"), "Power")
                    | (Some("Pick.BPM"), "BPM")
                    | (Some("Pick.Time"), "Hour" | "Minutes")
            )
        {
            return Err(format!("Unknown property {key}.{k}"));
        }
        if v.is_object() {
            node.drivers
                .insert(k.clone(), parse_node(v, next, depth + 1, None)?);
        } else {
            let value = if k == "Type" && key == "Light.Emitter" {
                match v.as_str() {
                    Some("Point") => json!(0),
                    Some("Plane") => json!(1),
                    Some("Spot") => json!(2),
                    _ => v.clone(),
                }
            } else if k == "Type" && key == "Create.Value.LFO" {
                match v.as_str() {
                    Some("Sawtooth") => json!(0),
                    Some("Triangle") => json!(1),
                    Some("Sine") => json!(2),
                    _ => v.clone(),
                }
            } else if let Some(index) = crate::native::specs()
                .get(key)
                .and_then(|spec| spec.options.get(k))
                .and_then(|options| {
                    v.as_str().and_then(|label| {
                        options
                            .iter()
                            .position(|option| option.eq_ignore_ascii_case(label))
                    })
                })
            {
                json!(index)
            } else {
                v.clone()
            };
            node.properties.insert(k.clone(), value);
        }
    }
    Ok(node)
}
pub fn validate(stack: &Stack) -> Result<()> {
    if stack.name.is_empty() || stack.name.len() > 200 || stack.name.chars().any(char::is_control) {
        return Err("Invalid stack name".into());
    }
    fn walk(n: &Node, depth: usize, ids: &mut HashSet<u64>, parent: Option<&str>) -> Result<()> {
        if depth > 64 || ids.len() >= 2048 || n.id == 0 || n.id > 1_000_000_000 || !ids.insert(n.id)
        {
            return Err("Invalid depth, node count or duplicate node identity".into());
        }
        let mut defs = defaults(&n.behavior)?;
        for (owner, key, value) in [
            ("Pick.Group", "Group", json!("")),
            ("Pick.Audio.Power", "Power", json!(0.5)),
            ("Pick.BPM", "BPM", json!(100.)),
            ("Pick.Time", "Hour", json!(0.)),
            ("Pick.Time", "Minutes", json!(0.)),
        ] {
            if parent == Some(owner) {
                defs.entry(key.to_owned()).or_insert(value);
            }
        }
        for (k, v) in &n.properties {
            let default = defs
                .get(k)
                .ok_or_else(|| format!("Unknown property {}.{k}", n.behavior))?;
            if default.is_number() {
                let parsed = number(v)
                    .ok_or_else(|| format!("{}.{k} must be a finite number", n.behavior))?;
                if matches!(k.as_str(), "Layer" | "Axis" | "Type" | "Count" | "Shape")
                    && parsed.fract() != 0.
                {
                    return Err(format!("{k} must be an integer"));
                }
                if matches!(k.as_str(), "Layer" | "Axis" | "Type")
                    && crate::native::specs().get(&n.behavior).is_none()
                    && !(0.0..=2.0).contains(&parsed)
                {
                    return Err(format!("{k} must be 0..2"));
                }

                if let Some(spec) = crate::native::specs().get(&n.behavior) {
                    if spec
                        .types
                        .get(k)
                        .is_some_and(|kind| kind == "Int32" || kind == "EnumIndex")
                        && parsed.fract() != 0.
                    {
                        return Err(format!("{k} must be an integer"));
                    }
                    if let Some(options) = spec.options.get(k) {
                        if parsed < 0. || parsed >= options.len() as f64 {
                            return Err(format!("Invalid enum value for {k}"));
                        }
                    }
                }
                if !parsed.is_finite() || parsed.abs() > 1e9 {
                    return Err("Number out of range".into());
                }
            } else if default.is_boolean() && !v.is_boolean() {
                return Err(format!("{k} must be boolean"));
            } else if default.is_string() && !v.is_string() {
                return Err(format!("{k} must be text"));
            } else if default.is_array() {
                vector(v)?;
            }
            if k == "Color" {
                color(v.as_str().ok_or("Expected color")?)?;
            }
            if k == "Code" && v.as_str().unwrap_or("").len() > 4096 {
                return Err("Script exceeds 4096 bytes".into());
            }
            if k == "Period"
                && n.behavior != "Tool.Deprecated.Index.Counter"
                && number(v).unwrap_or(0.) <= 0.
            {
                return Err("Period must be positive".into());
            }
            if k == "Count" && number(v).unwrap_or(0.) < 1. {
                return Err("Count must be positive".into());
            }
        }
        let property = |key: &str| n.properties.get(key).or_else(|| defs.get(key));
        let scalar = |key: &str| property(key).and_then(number);
        for key in ["Range", "Width", "Height", "Duration"] {
            if (n.behavior == "Light.Emitter" || n.behavior.ends_with("Bezier"))
                && scalar(key).is_some_and(|v| v <= 0.)
            {
                return Err(format!("{key} must be positive"));
            }
        }
        if scalar("Interval").is_some_and(|v| v < 0.001) {
            return Err("Interval must be at least 0.001 seconds".into());
        }
        for key in ["Intensity", "Gain", "Falloff", "Filter Time", "Padding"] {
            if scalar(key).is_some_and(|v| v < 0.) {
                return Err(format!("{key} must be nonnegative"));
            }
        }
        for key in ["Probability", "Randomness", "Softness", "Speed"] {
            if (key != "Speed" || n.behavior != "Create.Audio.Sample")
                && scalar(key).is_some_and(|v| !(0. ..=1.).contains(&v))
            {
                return Err(format!("{key} must be 0..1"));
            }
        }
        if scalar("Angle").is_some_and(|v| v <= 0. || v >= 180.) {
            return Err("Angle must be between 0 and 180 degrees".into());
        }
        if let Some(direction) = property("Direction").filter(|_| n.behavior == "Light.Emitter") {
            if vector(direction)?.iter().map(|v| v * v).sum::<f64>() < 1e-12 {
                return Err("Direction must be nonzero".into());
            }
        }
        if scalar("Seed").is_some_and(|v| v.fract() != 0.) {
            return Err("Seed must be an integer".into());
        }
        if n.drivers.contains_key("Seed") {
            return Err("Seed is a static generator setting, not a driven property".into());
        }
        for (k, d) in &n.drivers {
            if !defs.get(k).is_some_and(|v| v.is_number() || v.is_boolean()) {
                return Err("Drivers currently require numeric or boolean properties".into());
            }
            walk(d, depth + 1, ids, Some(&n.behavior))?
        }
        for child in &n.children {
            walk(child, depth + 1, ids, Some(&n.behavior))?
        }
        Ok(())
    }
    walk(&stack.root, 0, &mut HashSet::new(), None)
}
pub fn number(v: &Value) -> Option<f64> {
    v.as_f64().or_else(|| v.as_str()?.parse().ok())
}
pub fn vector(v: &Value) -> Result<[f64; 3]> {
    let a = v.as_array().ok_or("Expected XYZ array")?;
    if a.len() != 3 {
        return Err("Expected three coordinates".into());
    }
    let mut result = [0.; 3];
    for i in 0..3 {
        result[i] = number(&a[i])
            .filter(|v| v.is_finite() && v.abs() <= 1e9)
            .ok_or("Invalid coordinate")?;
    }
    Ok(result)
}
pub fn color(text: &str) -> Result<[f64; 4]> {
    let s = text
        .strip_prefix('#')
        .ok_or("Color must be #RRGGBB or #RRGGBBAA")?;
    if !matches!(s.len(), 6 | 8) || !s.is_ascii() {
        return Err("Invalid color length".into());
    }
    let mut c = [0., 0., 0., 1.];
    for i in 0..s.len() / 2 {
        c[i] = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).map_err(|_| "Invalid color")? as f64
            / 255.;
    }
    Ok(c)
}
pub fn encode(stack: &Stack) -> Result<String> {
    fn node(n: &Node) -> Value {
        let mut fields = serde_json::Map::new();
        if n.bypassed {
            fields.insert("bypassed".into(), json!(true));
        }
        for (k, v) in &n.properties {
            fields.insert(k.clone(), v.clone());
        }
        for (k, d) in &n.drivers {
            fields.insert(k.clone(), node(d));
        }
        if !n.children.is_empty() {
            fields.insert(
                "in".into(),
                Value::Array(n.children.iter().map(node).collect()),
            );
        }
        json!({n.behavior.clone():fields})
    }
    let mut envelope = json!({"name":stack.name,"stack":node(&stack.root)});
    if !stack.enabled {
        envelope["enabled"] = json!(false);
    }
    serde_yaml::to_string(&envelope).map_err(|e| e.to_string())
}
