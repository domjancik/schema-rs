use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
pub type Result<T> = std::result::Result<T, String>;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Node {
    pub id: u64,
    pub behavior: String,
    #[serde(default)]
    pub properties: BTreeMap<String, Value>,
    #[serde(default)]
    pub children: Vec<Node>,
    #[serde(default)]
    pub drivers: BTreeMap<String, Node>,
}
impl Node {
    pub fn find(&self, id: u64) -> Option<&Node> {
        if self.id == id {
            Some(self)
        } else {
            self.children
                .iter()
                .chain(self.drivers.values())
                .find_map(|c| c.find(id))
        }
    }
    pub fn find_mut(&mut self, id: u64) -> Option<&mut Node> {
        if self.id == id {
            Some(self)
        } else {
            self.children
                .iter_mut()
                .chain(self.drivers.values_mut())
                .find_map(|c| c.find_mut(id))
        }
    }
    pub fn max_id(&self) -> u64 {
        self.children
            .iter()
            .chain(self.drivers.values())
            .map(Node::max_id)
            .max()
            .unwrap_or(0)
            .max(self.id)
    }
    pub fn reidentify(&mut self, next: &mut u64) {
        self.id = *next;
        *next += 1;
        for c in self.children.iter_mut().chain(self.drivers.values_mut()) {
            c.reidentify(next)
        }
    }
}
pub const MAX_LAYERS: usize = 32;
fn enabled_by_default() -> bool {
    true
}
fn is_enabled(value: &bool) -> bool {
    *value
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Stack {
    #[serde(default = "enabled_by_default", skip_serializing_if = "is_enabled")]
    pub enabled: bool,
    pub name: String,
    pub root: Node,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FixtureModel {
    Dimmer,
    Rgb,
    Rgbw,
}
fn default_layers() -> Vec<Option<Stack>> {
    vec![None, None, None]
}
fn default_pixels() -> Vec<[f64; 3]> {
    vec![[0.; 3]]
}
impl FixtureModel {
    pub fn channels(self) -> usize {
        match self {
            Self::Dimmer => 1,
            Self::Rgb => 3,
            Self::Rgbw => 4,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Fixture {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dmx_model: Option<String>,
    #[serde(default)]
    pub setup_channel: usize,
    #[serde(default)]
    pub rotation: [f64; 3],
    #[serde(default = "default_pixels")]
    pub pixel_positions: Vec<[f64; 3]>,
    #[serde(default)]
    pub pixel_reverse: bool,
    pub id: String,
    pub name: String,
    pub position: [f64; 3],
    pub groups: Vec<String>,
    pub model: FixtureModel,
    pub universe: u16,
    pub address: u16,
    pub dimmer: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct OutputConfig {
    pub enabled: bool,
    pub target: String,
    pub fps: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Project {
    #[serde(default = "crate::output_design::default_models")]
    pub dmx_models: BTreeMap<String, crate::output_design::ModelDefinition>,
    #[serde(default)]
    pub dimming: crate::output_design::DimmingTimeline,
    #[serde(default = "default_layers")]
    pub director: Vec<Option<Stack>>,
    pub version: u32,
    pub name: String,
    pub fixtures: Vec<Fixture>,
    pub content: BTreeMap<String, Vec<Option<Stack>>>,
    pub output: OutputConfig,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ContentEntry {
    pub fixture: String,
    pub layer: usize,
    pub stack: Option<Stack>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Bank {
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub actions: BTreeMap<String, crate::native::actions::Action>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio: Option<crate::inputs::Audio>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub texture: Option<crate::inputs::Texture>,
    pub rotation: Option<[f64; 3]>,
    pub speed: Option<f64>,
    pub duration: Option<f64>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub channels: BTreeMap<String, Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pixel_positions: Option<Vec<[f64; 3]>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pick: Option<usize>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub emitters: Vec<crate::lighting::Emitter>,
    pub position: Option<[f64; 3]>,
    pub fixture_reference: Option<String>,
    pub value: Option<f64>,
    pub colors: Option<Vec<[f64; 4]>>,
}
impl Bank {
    pub fn merge(mut self, other: Self) -> Self {
        self.actions.extend(other.actions);
        if other.audio.is_some() {
            self.audio = other.audio;
        }
        if other.texture.is_some() {
            self.texture = other.texture;
        }
        for emitter in other.emitters {
            if self.emitters.len() < 1024
                && !self.emitters.iter().any(|e| {
                    e.fixture == emitter.fixture
                        && e.layer == emitter.layer
                        && e.node == emitter.node
                })
            {
                self.emitters.push(emitter);
            }
        }
        if other.rotation.is_some() {
            self.rotation = other.rotation;
        }
        if other.speed.is_some() {
            self.speed = other.speed;
        }
        if other.duration.is_some() {
            self.duration = other.duration;
        }
        if other.progress.is_some() {
            self.progress = other.progress;
        }
        if other.pick.is_some() {
            self.pick = other.pick;
        }
        if other.pixel_positions.is_some() {
            self.pixel_positions = other.pixel_positions;
        }
        for (key, value) in other.channels {
            if let (Some(Value::Object(existing)), Value::Object(incoming)) =
                (self.channels.get_mut(&key), &value)
            {
                existing.extend(incoming.clone());
            } else {
                self.channels.insert(key, value);
            }
        }
        if other.position.is_some() {
            self.position = other.position;
        }
        if other.fixture_reference.is_some() {
            self.fixture_reference = other.fixture_reference;
        }
        if other.value.is_some() {
            self.value = other.value
        }
        if other.colors.is_some() {
            self.colors = other.colors
        }
        self
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FixtureSnapshot {
    pub fixture: Fixture,
    pub result: Bank,
    pub layers: Vec<Bank>,
    pub nodes: BTreeMap<String, Bank>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Snapshot {
    #[serde(default)]
    pub plugin_generation: u64,
    #[serde(default)]
    pub actions: BTreeMap<String, crate::native::actions::Action>,
    #[serde(default)]
    pub director: Vec<Bank>,
    #[serde(default)]
    pub director_nodes: BTreeMap<String, Bank>,
    #[serde(default)]
    pub emitters: Vec<crate::lighting::Emitter>,
    pub frame: u64,
    pub revision: u64,
    pub time: f64,
    pub paused: bool,
    pub blackout: bool,
    pub fixtures: Vec<FixtureSnapshot>,
    pub universes: BTreeMap<u16, Vec<u8>>,
    pub diagnostics: Vec<String>,
    pub can_undo: bool,
    pub can_redo: bool,
    pub evaluation_us: u64,
    pub output_status: String,
}
impl Project {
    pub fn demo() -> Self {
        let stack=crate::scenic::parse("name: Spatial wash\nstack:\n  Modify.Color.Dim:\n    Amount:\n      Combine.Multiply:\n        in:\n          - Modify.Value.MapTo:\n              Minimum: 0.25\n              Maximum: 1\n              in:\n                - Create.Value.LFO: {Period: 4, Type: 2}\n          - Modify.Value.MapTo:\n              Minimum: 0.2\n              Maximum: 1\n              in:\n                - Spatial.Axis: {Axis: 0, Minimum: -5, Maximum: 5}\n    in:\n      - Create.Color.Color: {Color: '#c7eb8b'}\n").expect("built-in demo");
        let fixtures: Vec<_> = (0..8)
            .map(|i| Fixture {
                dmx_model: None,
                setup_channel: 0,
                rotation: [0.; 3],
                pixel_positions: default_pixels(),
                pixel_reverse: false,
                id: format!("fixture-{}", i + 1),
                name: format!("Wash {:02}", i + 1),
                position: [(i % 4) as f64 * 2. - 3., 0., (i / 4) as f64 * 3. - 1.5],
                groups: vec![if i < 4 { "front" } else { "back" }.into()],
                model: FixtureModel::Rgb,
                universe: 0,
                address: (i * 3 + 1) as u16,
                dimmer: 1.,
            })
            .collect();
        let content = fixtures
            .iter()
            .map(|f| (f.id.clone(), vec![Some(stack.clone()), None, None]))
            .collect();
        Self {
            dmx_models: crate::output_design::default_models(),
            dimming: Default::default(),
            director: default_layers(),
            version: 1,
            name: "Spatial light study".into(),
            fixtures,
            content,
            output: OutputConfig {
                enabled: false,
                target: "127.0.0.1:6454".into(),
                fps: 40,
            },
        }
    }
    pub fn footprint(&self, f: &Fixture) -> Result<usize> {
        let channels = match &f.dmx_model {
            Some(id) => self
                .dmx_models
                .get(id)
                .ok_or_else(|| format!("Unknown DMX model {id}"))?
                .slots
                .len(),
            None => f.model.channels(),
        };
        Ok(channels * f.pixel_positions.len())
    }
    pub fn validate(&self) -> Result<()> {
        self.dimming.validate()?;
        if self.dmx_models.len() > 128 {
            return Err("Too many DMX models".into());
        }
        for (id, model) in &self.dmx_models {
            if !crate::plugins::valid_name(id) {
                return Err("Invalid DMX model ID".into());
            }
            model.validate()?;
        }

        if self.version != 1
            || self.fixtures.len() > 512
            || self.name.len() > 200
            || self.name.chars().any(char::is_control)
        {
            return Err("Invalid project version, name or fixture count".into());
        }
        if !(1..=44).contains(&self.output.fps) {
            return Err("Output FPS must be 1..44".into());
        }
        let target: std::net::SocketAddr = self
            .output
            .target
            .parse()
            .map_err(|_| "Art-Net target must be a numeric IP:port")?;
        if !target.is_ipv4()
            || target.ip().is_unspecified()
            || target.ip().is_multicast()
            || target.ip() == std::net::Ipv4Addr::BROADCAST
        {
            return Err("Art-Net requires an explicit unicast IPv4 target".into());
        }
        if self.director.is_empty() || self.director.len() > MAX_LAYERS {
            return Err("Director requires 1..32 layers".into());
        }
        for stack in self.director.iter().flatten() {
            crate::scenic::validate(stack)?;
        }
        let mut ids = std::collections::HashSet::new();
        let mut occupied = std::collections::HashSet::new();
        for f in &self.fixtures {
            if !f
                .id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
                || f.id.is_empty()
                || f.id == "__director"
                || f.id.len() > 100
                || !ids.insert(&f.id)
                || f.name.len() > 200
                || f.name.chars().any(char::is_control)
                || f.groups.len() > 32
                || f.groups
                    .iter()
                    .any(|g| g.len() > 100 || g.chars().any(char::is_control))
                || f.position
                    .iter()
                    .any(|v| !v.is_finite() || v.abs() > 100000.)
                || !f.dimmer.is_finite()
                || !(0.0..=1.0).contains(&f.dimmer)
            {
                return Err(format!("Invalid fixture {}", f.id));
            }
            if f.pixel_positions.is_empty()
                || f.pixel_positions.len() > 512
                || f.pixel_positions
                    .iter()
                    .flatten()
                    .chain(&f.rotation)
                    .any(|v| !v.is_finite() || v.abs() > 100000.)
                || f.setup_channel > 65535
            {
                return Err("Invalid fixture pixel/setup context".into());
            }
            if f.universe > 32767
                || f.address == 0
                || f.address as usize + self.footprint(f)? - 1 > 512
            {
                return Err(format!("Invalid channel range for {}", f.name));
            }
            for c in f.address..f.address + (self.footprint(f)?) as u16 {
                if !occupied.insert((f.universe, c)) {
                    return Err(format!(
                        "Channel collision at universe {}, channel {}",
                        f.universe, c
                    ));
                }
            }
            let layers = self
                .content
                .get(&f.id)
                .ok_or_else(|| format!("Missing content for {}", f.id))?;
            if layers.is_empty() || layers.len() > MAX_LAYERS {
                return Err("Fixture requires 1..32 independent layers".into());
            }
            for s in layers.iter().flatten() {
                crate::scenic::validate(s)?;
            }
        }
        Ok(())
    }
}

impl Bank {
    pub(crate) fn validate_payload(&self) -> Result<()> {
        if [self.value, self.duration, self.speed, self.progress]
            .into_iter()
            .flatten()
            .any(|v| !v.is_finite())
            || [self.position, self.rotation]
                .into_iter()
                .flatten()
                .flatten()
                .any(|v| !v.is_finite())
            || self
                .colors
                .as_ref()
                .is_some_and(|v| v.len() > 512 || v.iter().flatten().any(|v| !v.is_finite()))
            || self.pixel_positions.as_ref().is_some_and(|v| {
                v.is_empty() || v.len() > 512 || v.iter().flatten().any(|v| !v.is_finite())
            })
            || self.actions.len() > 2048
            || self.channels.len() > 128
        {
            return Err("Invalid result bank payload".into());
        }
        if let Some(audio) = &self.audio {
            let mut audio = audio.clone();
            audio.prepare()?;
        }
        if let Some(texture) = &self.texture {
            texture.validate()?;
        }
        Ok(())
    }
}
