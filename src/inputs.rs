use crate::model::{Bank, ContentEntry, Result, Stack};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Audio {
    pub samples: Vec<f64>,
    pub sample_rate: u32,
    pub bpm: f64,
    pub beats: Vec<bool>,
    pub level: f64,
    pub peak: f64,
    pub power: f64,
    pub overall_power: f64,
    pub pitch: f64,
    pub bands: Vec<f64>,
}
impl Audio {
    pub fn prepare(&mut self) -> Result<()> {
        if self.samples.len() > 4096
            || self.samples.iter().any(|v| !v.is_finite() || v.abs() > 1e6)
            || self.beats.len() > 32
        {
            return Err("Invalid audio buffer".into());
        }
        if self.sample_rate == 0 {
            self.sample_rate = 48000;
        }
        if !(8000..=192000).contains(&self.sample_rate) {
            return Err("Invalid audio sample rate".into());
        }
        if !self.samples.is_empty() {
            let len = self.samples.len() as f64;
            self.power = self.samples.iter().map(|v| v * v).sum::<f64>() / len;
            self.level = self.power.sqrt();
            self.peak = self.samples.iter().map(|v| v.abs()).fold(0., f64::max);
            self.overall_power = self.overall_power.max(self.power);
            let crossings = self
                .samples
                .windows(2)
                .filter(|v| v[0] < 0. && v[1] >= 0.)
                .count();
            self.pitch = crossings as f64 * self.sample_rate as f64 / len;
            self.bands = (0..16)
                .map(|band| {
                    let k = (band + 1) as f64;
                    let (mut re, mut im) = (0., 0.);
                    for (i, s) in self.samples.iter().enumerate() {
                        let angle = std::f64::consts::TAU * k * i as f64 / len;
                        re += s * angle.cos();
                        im -= s * angle.sin();
                    }
                    (re * re + im * im).sqrt() * 2. / len
                })
                .collect();
        }
        if [
            self.bpm,
            self.level,
            self.peak,
            self.power,
            self.overall_power,
            self.pitch,
        ]
        .iter()
        .chain(&self.bands)
        .any(|v| !v.is_finite())
        {
            return Err("Non-finite audio analysis".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Texture {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<[f64; 4]>,
}
impl Texture {
    pub fn validate(&self) -> Result<()> {
        if self.width == 0
            || self.height == 0
            || self.width > 512
            || self.height > 256
            || self.pixels.len() != self.width * self.height
            || self.pixels.iter().flatten().any(|v| !v.is_finite())
        {
            return Err("Invalid texture".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Osc {
    pub arguments: Vec<serde_json::Value>,
    pub serial: u64,
    pub frame: u64,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Pong {
    pub ball: [f64; 3],
    pub paddles: Vec<f64>,
    pub width: f64,
}
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Inputs {
    #[serde(skip)]
    pub channel_test: Option<crate::dmx_test::Lease>,
    pub pixels: BTreeMap<String, Vec<[f64; 3]>>,
    pub serial: u64,
    pub audio: BTreeMap<usize, Audio>,
    pub audio_frames: BTreeMap<usize, u64>,
    pub audio_onsets: BTreeMap<usize, u64>,
    pub osc: BTreeMap<String, Osc>,
    pub controllers: BTreeMap<usize, f64>,
    pub pads: BTreeMap<usize, bool>,
    pub keyboard: BTreeSet<String>,
    pub xbox: Vec<f64>,
    pub selected: BTreeSet<String>,
    pub values: BTreeMap<String, f64>,
    pub colors: BTreeMap<usize, [f64; 4]>,
    pub primary: Option<[f64; 4]>,
    pub external_colors: Vec<[f64; 4]>,
    pub sequences: BTreeMap<String, Vec<String>>,
    pub remote: BTreeMap<String, Vec<Bank>>,
    pub files: BTreeMap<String, bool>,
    pub audio_samples: BTreeMap<String, std::sync::Arc<Audio>>,
    pub textures: BTreeMap<String, std::sync::Arc<Texture>>,
    pub stacks: BTreeMap<String, Stack>,
    pub content: BTreeMap<String, Vec<ContentEntry>>,
    pub director: Vec<Option<Stack>>,
    pub clock_seconds: u32,
    pub last_controller: Option<usize>,
    pub pong: Option<Pong>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Event {
    ChannelTest {
        owner: String,
        universe: u16,
        #[serde(deserialize_with = "crate::dmx_test::deserialize_channels")]
        channels: BTreeMap<u16, u8>,
    },
    Pixels {
        fixture: String,
        positions: Vec<[f64; 3]>,
    },
    Audio {
        slot: usize,
        audio: Audio,
    },
    Osc {
        address: String,
        arguments: Vec<serde_json::Value>,
    },
    Controller {
        index: usize,
        value: f64,
    },
    Pad {
        index: usize,
        pressed: bool,
    },
    Keyboard {
        keys: Vec<String>,
    },
    Xbox {
        values: Vec<f64>,
    },
    Selection {
        fixtures: Vec<String>,
    },
    Value {
        name: String,
        value: f64,
    },
    Color {
        slot: usize,
        color: [f64; 4],
    },
    Primary {
        color: [f64; 4],
    },
    ExternalColors {
        colors: Vec<[f64; 4]>,
    },
    Sequence {
        name: String,
        fixtures: Vec<String>,
    },
    Remote {
        address: String,
        banks: Vec<Bank>,
    },
    File {
        path: String,
        exists: bool,
    },
    Sample {
        name: String,
        audio: Audio,
    },
    Texture {
        name: String,
        texture: Texture,
    },
    Stack {
        name: String,
        stack: Stack,
    },
    Content {
        name: String,
        entries: Vec<ContentEntry>,
    },
    Clock {
        seconds: u32,
    },
    Pong {
        state: Pong,
    },
}
impl Event {
    pub fn prepare(&mut self) -> Result<()> {
        match self {
            Self::ChannelTest {
                owner,
                universe,
                channels,
            } => crate::dmx_test::validate(owner, *universe, channels)?,
            Self::Audio { slot, audio } => {
                if *slot > 31 {
                    return Err("Audio slot must be 0..31".into());
                }
                audio.prepare()?;
            }
            Self::Sample { name, audio } => {
                if name.len() > 200 {
                    return Err("Invalid sample name".into());
                }
                if audio.samples.len() > 2_000_000
                    || audio.samples.iter().any(|v| !v.is_finite())
                    || !(8000..=192000).contains(&audio.sample_rate)
                {
                    return Err("Invalid sample resource".into());
                }
            }
            Self::Pixels { fixture, positions } => {
                if fixture.len() > 200
                    || positions.is_empty()
                    || positions.len() > 512
                    || positions
                        .iter()
                        .flatten()
                        .any(|v| !v.is_finite() || v.abs() > 1e9)
                {
                    return Err("Invalid fixture pixel positions".into());
                }
            }
            Self::Content { entries, .. } => {
                if entries.len() > 1024 {
                    return Err("Content exceeds fixture/layer budget".into());
                }
                let mut targets = std::collections::BTreeSet::new();
                for entry in entries {
                    if entry.layer >= crate::model::MAX_LAYERS
                        || !targets.insert((entry.fixture.clone(), entry.layer))
                    {
                        return Err("Invalid content target".into());
                    }
                    if let Some(stack) = &entry.stack {
                        crate::scenic::validate(stack)?;
                    }
                }
            }
            Self::Remote { banks, .. } => {
                if banks.len() > 32 {
                    return Err("Remote input exceeds 32 banks".into());
                }
                for bank in banks {
                    bank.validate_payload()?;
                }
            }
            Self::Selection { fixtures } | Self::Sequence { fixtures, .. } => {
                if fixtures.len() > 1024 || fixtures.iter().any(|s| s.len() > 200) {
                    return Err("Invalid fixture list".into());
                }
            }
            Self::Value { value, .. } => {
                if !value.is_finite() {
                    return Err("Invalid global value".into());
                }
            }
            Self::Color { color, .. } | Self::Primary { color } => {
                if color.iter().any(|v| !v.is_finite()) {
                    return Err("Invalid color".into());
                }
            }
            Self::ExternalColors { colors } => {
                if colors.len() > 4096 || colors.iter().flatten().any(|v| !v.is_finite()) {
                    return Err("Invalid external colors".into());
                }
            }
            Self::Pong { state } => {
                if state.paddles.len() > 16
                    || state
                        .ball
                        .iter()
                        .chain(&state.paddles)
                        .chain(std::iter::once(&state.width))
                        .any(|v| !v.is_finite())
                {
                    return Err("Invalid Pong state".into());
                }
            }
            Self::Texture { texture, .. } => texture.validate()?,
            Self::Stack { stack, .. } => crate::scenic::validate(stack)?,
            Self::Osc { address, arguments } => {
                if address.len() > 200 || arguments.len() > 64 {
                    return Err("Invalid OSC input".into());
                }
            }
            Self::Controller { index, value } => {
                if *index > 65535 || !value.is_finite() {
                    return Err("Invalid controller input".into());
                }
            }
            Self::Keyboard { keys } => {
                if keys.len() > 128 || keys.iter().any(|s| s.len() > 80) {
                    return Err("Invalid keyboard input".into());
                }
            }
            Self::Xbox { values } => {
                if values.len() > 64 || values.iter().any(|v| !v.is_finite()) {
                    return Err("Invalid Xbox input".into());
                }
            }
            Self::Clock { seconds } => {
                if *seconds >= 86400 {
                    return Err("Time of day is outside 24 hours".into());
                }
            }
            _ => {
                let bytes = serde_json::to_vec(self).map_err(|e| e.to_string())?;
                if bytes.len() > 4 * 1024 * 1024 {
                    return Err("Input event exceeds limit".into());
                }
            }
        }
        Ok(())
    }
}
impl Inputs {
    pub fn validate(&self) -> Result<()> {
        if [
            self.pixels.len(),
            self.audio.len(),
            self.osc.len(),
            self.controllers.len(),
            self.pads.len(),
            self.values.len(),
            self.colors.len(),
            self.sequences.len(),
            self.remote.len(),
            self.files.len(),
            self.audio_samples.len(),
            self.textures.len(),
            self.stacks.len(),
            self.content.len(),
        ]
        .iter()
        .any(|n| *n > 256)
            || self
                .audio_samples
                .values()
                .map(|a| a.samples.len())
                .sum::<usize>()
                > 2_000_000
            || self
                .textures
                .values()
                .map(|t| t.pixels.len())
                .sum::<usize>()
                > 262144
        {
            return Err("Provider state exceeds budget".into());
        }
        let mut events = vec![
            Event::Keyboard {
                keys: self.keyboard.iter().cloned().collect(),
            },
            Event::Xbox {
                values: self.xbox.clone(),
            },
            Event::Clock {
                seconds: self.clock_seconds,
            },
            Event::Selection {
                fixtures: self.selected.iter().cloned().collect(),
            },
            Event::ExternalColors {
                colors: self.external_colors.clone(),
            },
        ];
        for (fixture, positions) in &self.pixels {
            events.push(Event::Pixels {
                fixture: fixture.clone(),
                positions: positions.clone(),
            });
        }
        for (slot, audio) in &self.audio {
            events.push(Event::Audio {
                slot: *slot,
                audio: audio.clone(),
            });
        }
        for (address, osc) in &self.osc {
            events.push(Event::Osc {
                address: address.clone(),
                arguments: osc.arguments.clone(),
            });
        }
        for (index, value) in &self.controllers {
            events.push(Event::Controller {
                index: *index,
                value: *value,
            });
        }
        for (name, value) in &self.values {
            events.push(Event::Value {
                name: name.clone(),
                value: *value,
            });
        }
        for (slot, color) in &self.colors {
            events.push(Event::Color {
                slot: *slot,
                color: *color,
            });
        }
        if let Some(color) = self.primary {
            events.push(Event::Primary { color });
        }
        for (name, audio) in &self.audio_samples {
            events.push(Event::Sample {
                name: name.clone(),
                audio: audio.as_ref().clone(),
            });
        }
        for (name, texture) in &self.textures {
            events.push(Event::Texture {
                name: name.clone(),
                texture: texture.as_ref().clone(),
            });
        }
        for (name, stack) in &self.stacks {
            events.push(Event::Stack {
                name: name.clone(),
                stack: stack.clone(),
            });
        }
        for (name, entries) in &self.content {
            events.push(Event::Content {
                name: name.clone(),
                entries: entries.clone(),
            });
        }
        for (address, banks) in &self.remote {
            events.push(Event::Remote {
                address: address.clone(),
                banks: banks.clone(),
            });
        }
        for event in &mut events {
            event.prepare()?;
        }
        Ok(())
    }
    pub fn apply(&mut self, event: Event, frame: u64) -> Result<()> {
        macro_rules! capacity {
            ($map:expr,$key:expr) => {
                if $map.len() >= 256 && !$map.contains_key($key) {
                    return Err("Input registry exceeds 256 entries".into());
                }
            };
        }
        match &event {
            Event::Pixels { fixture, .. } => capacity!(self.pixels, fixture),
            Event::Audio { slot, .. } => capacity!(self.audio, slot),
            Event::Osc { address, .. } => capacity!(self.osc, address),
            Event::Controller { index, .. } => capacity!(self.controllers, index),
            Event::Pad { index, .. } => capacity!(self.pads, index),
            Event::Value { name, .. } => capacity!(self.values, name),
            Event::Color { slot, .. } => capacity!(self.colors, slot),
            Event::Sequence { name, .. } => capacity!(self.sequences, name),
            Event::Remote { address, .. } => capacity!(self.remote, address),
            Event::File { path, .. } => capacity!(self.files, path),
            Event::Stack { name, .. } => capacity!(self.stacks, name),
            Event::Content { name, .. } => capacity!(self.content, name),
            Event::Sample { name, audio } => {
                capacity!(self.audio_samples, name);
                if self
                    .audio_samples
                    .iter()
                    .filter(|(key, _)| *key != name)
                    .map(|(_, a)| a.samples.len())
                    .sum::<usize>()
                    + audio.samples.len()
                    > 2_000_000
                {
                    return Err("Loaded samples exceed two million samples".into());
                }
            }
            Event::Texture { name, texture } => {
                capacity!(self.textures, name);
                if self
                    .textures
                    .iter()
                    .filter(|(key, _)| *key != name)
                    .map(|(_, t)| t.pixels.len())
                    .sum::<usize>()
                    + texture.pixels.len()
                    > 262144
                {
                    return Err("Loaded textures exceed pixel budget".into());
                }
            }
            _ => {}
        }

        self.serial = self.serial.wrapping_add(1);
        match event {
            Event::ChannelTest {
                owner,
                universe,
                channels,
            } => crate::dmx_test::update_at(
                &mut self.channel_test,
                owner,
                universe,
                channels,
                crate::clock::Instant::now(),
            )?,
            Event::Pixels { fixture, positions } => {
                self.pixels.insert(fixture, positions);
            }
            Event::Audio { slot, mut audio } => {
                if audio.beats.is_empty() {
                    let previous = self.audio.get(&slot);
                    audio.beats = (0..4)
                        .map(|band| {
                            let power = audio.bands.iter().skip(band * 4).take(4).sum::<f64>();
                            let before = previous
                                .map_or(0., |a| a.bands.iter().skip(band * 4).take(4).sum::<f64>());
                            power > 0.02 && power > before * 1.5
                        })
                        .collect();
                    if audio.beats.iter().any(|v| *v) {
                        if let Some(previous) = self.audio_onsets.insert(slot, frame) {
                            let elapsed = frame.saturating_sub(previous);
                            if (15..=180).contains(&elapsed) {
                                audio.bpm = 3600. / elapsed as f64;
                            }
                        }
                    }
                    if audio.bpm == 0. {
                        audio.bpm = previous.map_or(0., |a| a.bpm);
                    }
                }
                self.audio.insert(slot, audio);
                self.audio_frames.insert(slot, frame);
            }
            Event::Osc { address, arguments } => {
                self.osc.insert(
                    address,
                    Osc {
                        arguments,
                        serial: self.serial,
                        frame,
                    },
                );
            }
            Event::Controller { index, value } => {
                self.controllers.insert(index, value);
                self.last_controller = Some(index);
            }
            Event::Pad { index, pressed } => {
                self.pads.insert(index, pressed);
            }
            Event::Keyboard { keys } => self.keyboard = keys.into_iter().collect(),
            Event::Xbox { values } => self.xbox = values,
            Event::Selection { fixtures } => self.selected = fixtures.into_iter().collect(),
            Event::Value { name, value } => {
                self.values.insert(name, value);
            }
            Event::Color { slot, color } => {
                self.colors.insert(slot, color);
            }
            Event::Primary { color } => self.primary = Some(color),
            Event::ExternalColors { colors } => self.external_colors = colors,
            Event::Sequence { name, fixtures } => {
                self.sequences.insert(name, fixtures);
            }
            Event::Remote { address, banks } => {
                self.remote.insert(address, banks);
            }
            Event::File { path, exists } => {
                self.files.insert(path, exists);
            }
            Event::Sample { name, audio } => {
                self.audio_samples.insert(name, std::sync::Arc::new(audio));
            }
            Event::Texture { name, texture } => {
                self.textures.insert(name, std::sync::Arc::new(texture));
            }
            Event::Stack { name, stack } => {
                self.stacks.insert(name, stack);
            }
            Event::Content { name, entries } => {
                self.content.insert(name, entries);
            }
            Event::Clock { seconds } => self.clock_seconds = seconds,
            Event::Pong { state } => self.pong = Some(state),
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct PreparedEvent(pub(crate) Event);
impl Event {
    pub fn prepared(mut self) -> Result<PreparedEvent> {
        self.prepare()?;
        Ok(PreparedEvent(self))
    }
}
