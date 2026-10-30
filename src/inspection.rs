//! Bounded, transient samples captured during actual evaluation, only when requested.
use crate::model::{Bank, DebugFocus};
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;
pub const MAX_TARGETS: usize = 256;
const SAMPLES: usize = 8;
#[derive(Clone, Debug, Default, Serialize)]
pub struct Attributes {
    pub attributes: BTreeMap<String, Value>,
    pub counts: BTreeMap<String, usize>,
}
impl Attributes {
    pub fn capture(bank: &Bank) -> Self {
        let mut result = Self::default();
        macro_rules! scalar {($($field:ident),*)=>{$(if let Some(value)=&bank.$field{result.attributes.insert(stringify!($field).into(),json!(value));})*};}
        scalar!(rotation, speed, duration, progress, pick, position, value);
        if let Some(reference) = &bank.fixture_reference {
            result.attributes.insert(
                "fixture_reference".into(),
                json!(reference.chars().take(200).collect::<String>()),
            );
        }
        if let Some(colors) = &bank.colors {
            result.counts.insert("colors".into(), colors.len());
            result
                .attributes
                .insert("colors".into(), json!(&colors[..colors.len().min(SAMPLES)]));
        }
        if let Some(positions) = &bank.pixel_positions {
            result
                .counts
                .insert("pixel_positions".into(), positions.len());
            result.attributes.insert(
                "pixel_positions".into(),
                json!(&positions[..positions.len().min(SAMPLES)]),
            );
        }
        if let Some(audio) = &bank.audio {
            result.attributes.insert("audio".into(),json!({"level":audio.level,"peak":audio.peak,"power":audio.power,"overall_power":audio.overall_power,"pitch":audio.pitch,"bpm":audio.bpm,"sample_rate":audio.sample_rate,"samples":audio.samples.len(),"bands":&audio.bands[..audio.bands.len().min(SAMPLES)]}));
        }
        if let Some(texture) = &bank.texture {
            result.attributes.insert("texture".into(),json!({"width":texture.width,"height":texture.height,"pixels":texture.pixels.len(),"sample":&texture.pixels[..texture.pixels.len().min(SAMPLES)]}));
        }
        for (name, values) in [
            (
                "actions",
                bank.actions
                    .keys()
                    .take(SAMPLES)
                    .cloned()
                    .collect::<Vec<_>>(),
            ),
            (
                "channels",
                bank.channels.keys().take(SAMPLES).cloned().collect(),
            ),
        ] {
            let count = if name == "actions" {
                bank.actions.len()
            } else {
                bank.channels.len()
            };
            if count > 0 {
                result.counts.insert(name.into(), count);
                result.attributes.insert(
                    name.into(),
                    json!(values
                        .into_iter()
                        .map(|v| v.chars().take(160).collect::<String>())
                        .collect::<Vec<_>>()),
                );
            }
        }
        if !bank.emitters.is_empty() {
            result.counts.insert("emitters".into(), bank.emitters.len());
            result.attributes.insert(
                "emitters".into(),
                json!(bank
                    .emitters
                    .iter()
                    .take(SAMPLES)
                    .map(|e| json!({"color":e.color,"intensity":e.intensity}))
                    .collect::<Vec<_>>()),
            );
        }
        if !bank.channels.is_empty() {
            result.attributes.insert(
                "channels".into(),
                Value::Object(
                    bank.channels
                        .iter()
                        .take(SAMPLES)
                        .map(|(k, v)| (k.chars().take(160).collect(), bounded(v, 2)))
                        .collect(),
                ),
            );
        }
        result
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct Block {
    pub target: DebugFocus,
    pub frame: u64,
    pub revision: u64,
    pub incoming: Attributes,
    pub outgoing: Attributes,
    pub bypassed: bool,
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct Frame {
    pub frame: u64,
    pub revision: u64,
    pub blocks: Vec<Block>,
}
#[derive(Default)]
pub(crate) struct Capture {
    wanted: BTreeMap<String, std::collections::BTreeSet<(usize, u64)>>,
    pub targets: Vec<DebugFocus>,
    pub blocks: Vec<Block>,
}
impl Capture {
    pub fn new(targets: Vec<DebugFocus>) -> Self {
        let mut wanted: BTreeMap<String, std::collections::BTreeSet<(usize, u64)>> =
            BTreeMap::new();
        for target in &targets {
            wanted
                .entry(target.fixture.clone())
                .or_default()
                .insert((target.layer, target.node));
        }
        Self {
            wanted,
            targets,
            blocks: Vec::new(),
        }
    }
    pub fn wants(&self, fixture: &str, layer: usize, node: u64) -> bool {
        self.wanted
            .get(fixture)
            .is_some_and(|nodes| nodes.contains(&(layer, node)))
    }
    pub fn record(
        &mut self,
        target: DebugFocus,
        incoming: Attributes,
        outgoing: &Bank,
        bypassed: bool,
    ) {
        self.blocks.push(Block {
            target,
            frame: 0,
            revision: 0,
            incoming,
            outgoing: Attributes::capture(outgoing),
            bypassed,
        });
    }
}

fn bounded(value: &Value, depth: usize) -> Value {
    match value {
        Value::String(s) => json!(s.chars().take(160).collect::<String>()),
        Value::Array(a) if depth > 0 => json!(a
            .iter()
            .take(SAMPLES)
            .map(|v| bounded(v, depth - 1))
            .collect::<Vec<_>>()),
        Value::Object(o) if depth > 0 => Value::Object(
            o.iter()
                .take(SAMPLES)
                .map(|(k, v)| (k.chars().take(160).collect(), bounded(v, depth - 1)))
                .collect(),
        ),
        Value::Array(_) | Value::Object(_) => json!("…"),
        _ => value.clone(),
    }
}
