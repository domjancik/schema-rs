//! Native output mapping and cyclic daily master curve; no scripting on the frame path.
use crate::model::{Bank, Fixture, Project, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Point {
    pub minute: u16,
    pub level: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct DimmingTimeline {
    pub enabled: bool,
    pub master: f64,
    pub utc_offset_minutes: i32,
    pub points: Vec<Point>,
}
impl Default for DimmingTimeline {
    fn default() -> Self {
        Self {
            enabled: false,
            master: 1.,
            utc_offset_minutes: 0,
            points: vec![
                Point {
                    minute: 480,
                    level: 1.,
                },
                Point {
                    minute: 1200,
                    level: 0.2,
                },
            ],
        }
    }
}
impl DimmingTimeline {
    pub fn validate(&self) -> Result<()> {
        if !self.master.is_finite()
            || !(0.0..=1.).contains(&self.master)
            || !(-840..=840).contains(&self.utc_offset_minutes)
            || self.points.is_empty()
            || self.points.len() > 96
        {
            return Err("Invalid dimming timeline".into());
        }
        let mut seen = std::collections::HashSet::new();
        for p in &self.points {
            if p.minute >= 1440
                || !p.level.is_finite()
                || !(0.0..=1.).contains(&p.level)
                || !seen.insert(p.minute)
            {
                return Err("Timeline needs unique times and levels 0..1".into());
            }
        }
        Ok(())
    }
    pub fn level(&self, utc_seconds: u32) -> f64 {
        if !self.enabled {
            return self.master;
        }
        let minute = (utc_seconds as f64 / 60. + self.utc_offset_minutes as f64).rem_euclid(1440.);
        let before = self.points.iter().min_by(|a, b| {
            (minute - a.minute as f64)
                .rem_euclid(1440.)
                .total_cmp(&(minute - b.minute as f64).rem_euclid(1440.))
        });
        let after = self.points.iter().min_by(|a, b| {
            (a.minute as f64 - minute)
                .rem_euclid(1440.)
                .total_cmp(&(b.minute as f64 - minute).rem_euclid(1440.))
        });
        match (before, after) {
            (Some(a), Some(b)) => {
                let span = (b.minute as f64 - a.minute as f64).rem_euclid(1440.);
                let t = if span == 0. {
                    0.
                } else {
                    (minute - a.minute as f64).rem_euclid(1440.) / span
                };
                self.master * (a.level + (b.level - a.level) * t)
            }
            _ => self.master,
        }
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    Value,
    Red,
    Green,
    Blue,
    White,
    Pan,
    Tilt,
    Speed,
    Duration,
    Channel,
    Constant,
}
fn maximum() -> u8 {
    255
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Slot {
    pub name: String,
    pub source: Source,
    #[serde(default)]
    pub constant: u8,
    #[serde(default)]
    pub channel: String,
    #[serde(default)]
    pub minimum: u8,
    #[serde(default = "maximum")]
    pub maximum: u8,
    #[serde(default)]
    pub fine: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ModelDefinition {
    pub name: String,
    pub slots: Vec<Slot>,
}
impl ModelDefinition {
    pub fn validate(&self) -> Result<()> {
        if self.name.is_empty()
            || self.name.len() > 100
            || self.name.chars().any(char::is_control)
            || self.slots.is_empty()
            || self.slots.len() > 512
        {
            return Err("Invalid DMX model name/slot count".into());
        }
        for s in &self.slots {
            if s.name.len() > 100
                || s.channel.len() > 100
                || s.name
                    .chars()
                    .chain(s.channel.chars())
                    .any(char::is_control)
                || s.source == Source::Channel && s.channel.is_empty()
            {
                return Err("Invalid DMX slot".into());
            }
        }
        Ok(())
    }
}
pub fn default_models() -> BTreeMap<String, ModelDefinition> {
    BTreeMap::from([(
        "uv-cob".into(),
        ModelDefinition {
            name: "UV COB · 2 channel".into(),
            slots: vec![
                Slot {
                    name: "Dimmer".into(),
                    source: Source::Value,
                    constant: 0,
                    channel: String::new(),
                    minimum: 0,
                    maximum: 255,
                    fine: false,
                },
                Slot {
                    name: "Program".into(),
                    source: Source::Constant,
                    constant: 0,
                    channel: String::new(),
                    minimum: 0,
                    maximum: 255,
                    fine: false,
                },
            ],
        },
    )])
}
pub fn encode(
    project: &Project,
    fixture: &Fixture,
    bank: &Bank,
    blackout: bool,
    global: f64,
) -> Vec<u8> {
    let Some(model) = fixture
        .dmx_model
        .as_ref()
        .and_then(|id| project.dmx_models.get(id))
    else {
        let mut f = fixture.clone();
        f.dimmer *= global;
        return crate::engine::encode_fixture(&f, bank, blackout);
    };
    let mut out = Vec::with_capacity(model.slots.len() * fixture.pixel_positions.len());
    for pixel in 0..fixture.pixel_positions.len() {
        let color = bank
            .colors
            .as_ref()
            .filter(|c| !c.is_empty())
            .map(|c| c[pixel % c.len()])
            .unwrap_or_else(|| {
                let v = bank.value.unwrap_or(0.);
                [v, v, v, 1.]
            });
        let gain = if blackout {
            0.
        } else {
            fixture.dimmer * global * color[3].clamp(0., 1.)
        };
        for slot in &model.slots {
            let raw = match slot.source {
                Source::Constant => {
                    out.push(slot.constant);
                    continue;
                }
                Source::Value => bank.value.unwrap_or((color[0] + color[1] + color[2]) / 3.),
                Source::Red => color[0],
                Source::Green => color[1],
                Source::Blue => color[2],
                Source::White => color[0].min(color[1]).min(color[2]),
                Source::Pan => bank.rotation.map_or(0., |r| r[0]),
                Source::Tilt => bank.rotation.map_or(0., |r| r[1]),
                Source::Speed => bank.speed.unwrap_or(0.),
                Source::Duration => bank.duration.unwrap_or(0.),
                Source::Channel => bank
                    .channels
                    .get(&slot.channel)
                    .and_then(serde_json::Value::as_f64)
                    .unwrap_or(0.),
            };
            let mapped = slot.minimum as f64
                + (slot.maximum as f64 - slot.minimum as f64)
                    * if raw.is_finite() {
                        raw.clamp(0., 1.)
                    } else {
                        0.
                    };
            let mapped = if matches!(
                slot.source,
                Source::Value | Source::Red | Source::Green | Source::Blue | Source::White
            ) {
                mapped * gain
            } else {
                mapped
            };
            out.push(if slot.fine {
                ((mapped / 255. * 65535.).round() as u16 & 255) as u8
            } else {
                mapped.round() as u8
            });
        }
    }
    out
}
