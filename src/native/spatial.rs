use super::{flag, num, pixels, providers::Context, text, timed::State};
use crate::model::{Bank, Node};
use serde_json::{json, Value};
use std::collections::BTreeMap;
pub(crate) fn supported(key: &str) -> bool {
    matches!(
        key,
        "Create.Value.Closeness"
            | "Create.Value.Group.AxisPosition"
            | "Create.Color.Pixel.Distance.Axis"
            | "Create.Color.Pixel.Distance.Plane"
            | "Create.Color.Pixel.Distance.Point"
            | "Create.Color.Pixel.Pulse"
            | "Create.Color.Pixel.Pulse.Cannon"
            | "Create.Copycat"
            | "Create.Radius.Constant"
            | "Create.Rotation.Audio.Shake"
            | "Create.Rotation.Follow"
            | "Modify.Color.Rainbow"
            | "Modify.Value.Map.HalfUpDown"
            | "Pick.Group"
            | "Pick.LTP"
            | "Tool.Deprecated.Index.Counter"
            | "Create.Value.Expression"
    )
}
fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    a.iter()
        .zip(b)
        .map(|(a, b)| (a - b).powi(2))
        .sum::<f64>()
        .sqrt()
}
fn rotated(mut point: [f64; 3], rotation: [f64; 3], inverse: bool) -> [f64; 3] {
    let axes = if inverse { [2, 1, 0] } else { [0, 1, 2] };
    for axis in axes {
        let angle = rotation[axis] * std::f64::consts::TAU * if inverse { -1. } else { 1. };
        let (a, b) = match axis {
            0 => (1, 2),
            1 => (2, 0),
            _ => (0, 1),
        };
        let x = point[a];
        let y = point[b];
        point[a] = x * angle.cos() - y * angle.sin();
        point[b] = x * angle.sin() + y * angle.cos();
    }
    point
}
pub(crate) fn evaluate(
    node: &Node,
    p: &BTreeMap<String, Value>,
    inputs: &[Bank],
    bank: &mut Bank,
    state: &mut State,
    c: &mut Context<'_>,
) -> bool {
    let key = node.behavior.as_str();
    if !supported(key) || key == "Create.Value.Expression" {
        return false;
    }
    let n = |key, default| num(p, key, default);
    let dt = state.delta(c.time);
    match key {
        "Create.Value.Closeness"
        | "Create.Color.Pixel.Distance.Axis"
        | "Create.Color.Pixel.Distance.Plane"
        | "Create.Color.Pixel.Distance.Point"
        | "Create.Rotation.Follow" => {
            let reference = bank.fixture_reference.clone().unwrap_or_else(|| {
                text(
                    p,
                    "Fixture",
                    if key == "Create.Rotation.Follow" {
                        "Lantern"
                    } else {
                        "Fixture"
                    },
                )
                .to_owned()
            });
            let target = c
                .fixtures
                .iter()
                .find(|f| f.id == reference || f.name == reference);
            if let Some(target) = target {
                let position = c
                    .positions
                    .get(&target.id)
                    .copied()
                    .unwrap_or(target.position);
                let source = bank.position.unwrap_or(c.fixture.position);
                let radius = n("Radius", 1.).abs().max(1e-9);
                if key == "Create.Value.Closeness" {
                    bank.value = Some(1. - distance(source, position) / radius);
                } else if key == "Create.Rotation.Follow" {
                    let [x, y, z] = std::array::from_fn(|i| position[i] - c.fixture.position[i]);
                    let yaw = 1.
                        - (if x < 0. { (-z).atan2(-x) } else { z.atan2(x) }
                            / std::f64::consts::TAU)
                            .rem_euclid(1.);
                    let tilt = (y.atan2(x) / std::f64::consts::TAU).rem_euclid(1.) + 0.25;
                    bank.rotation = Some([
                        (-c.fixture.rotation[0]).rem_euclid(1.),
                        (yaw - c.fixture.rotation[1]).rem_euclid(1.),
                        (tilt - c.fixture.rotation[2]).rem_euclid(1.),
                    ]);
                } else {
                    let local = flag(p, "Target/Local", false);
                    let center = if local { source } else { position };
                    let positions = bank.pixel_positions.clone().unwrap_or_else(|| {
                        c.fixture
                            .pixel_positions
                            .iter()
                            .map(|p| {
                                let p = rotated(*p, c.fixture.rotation, false);
                                std::array::from_fn(|i| p[i] + c.fixture.position[i])
                            })
                            .collect()
                    });
                    let target_color = if flag(p, "Color From Target", false) {
                        c.prior
                            .fixtures
                            .iter()
                            .find(|f| f.fixture.id == target.id)
                            .and_then(|f| f.result.colors.as_ref())
                            .and_then(|v| v.first())
                            .copied()
                            .unwrap_or([1.; 4])
                    } else {
                        [1.; 4]
                    };
                    bank.colors = Some(
                        positions
                            .iter()
                            .map(|point| {
                                let delta = std::array::from_fn(|i| point[i] - center[i]);
                                let transformed = rotated(delta, target.rotation, true);
                                let axis = (n("Axis", 0.) as usize).min(2);
                                let signed = if key.ends_with("Axis") {
                                    delta[axis]
                                } else {
                                    transformed[1]
                                };
                                let d = if key.ends_with("Point") {
                                    distance(*point, center)
                                } else {
                                    signed.abs()
                                };
                                let direction = n("Direction", 0.) as usize;
                                let level = if direction == 1 && signed > 0.
                                    || direction == 2 && signed < 0.
                                {
                                    1.
                                } else {
                                    (1. - d / radius).clamp(0., 1.)
                                };
                                [
                                    target_color[0] * level,
                                    target_color[1] * level,
                                    target_color[2] * level,
                                    target_color[3],
                                ]
                            })
                            .collect(),
                    );
                }
            } else {
                c.diagnostics.push(format!(
                    "{}: referenced fixture '{}' missing",
                    c.fixture.name, reference
                ));
                if key == "Create.Value.Closeness" {
                    bank.value = None;
                } else if key.starts_with("Create.Color") {
                    bank.colors = None;
                } else {
                    bank.rotation = None;
                }
            }
        }
        "Create.Value.Group.AxisPosition" | "Modify.Color.Rainbow" => {
            if let Some((min, max)) =
                crate::lighting::group_bounds(c.fixtures, text(p, "Group", "Group"))
            {
                let axis = (n("Axis", 0.) as usize).min(2);
                let position = bank.position.unwrap_or(c.fixture.position)[axis];
                let t = if (max[axis] - min[axis]).abs() < 1e-9 {
                    0.
                } else {
                    (position - min[axis]) / (max[axis] - min[axis])
                };
                if key == "Create.Value.Group.AxisPosition" {
                    bank.value = Some(t);
                } else if let Some(colors) = bank.colors.as_mut() {
                    for color in colors {
                        let mut hsv = super::hsv(*color);
                        hsv[0] += t * n("Width", 0.) + n("Offset", 0.);
                        *color = super::rgb(hsv);
                    }
                }
            } else {
                c.diagnostics.push("Fixture group has no bounds".into());
                bank.value = None;
            }
        }
        "Create.Copycat" => {
            let reference = bank
                .fixture_reference
                .clone()
                .unwrap_or_else(|| text(p, "Fixture", "Fixture").to_owned());
            if let Some(target) = c
                .fixtures
                .iter()
                .find(|f| f.id == reference || f.name == reference)
            {
                if let Some(source) = c
                    .live_layers
                    .get(&target.id)
                    .and_then(|layers| layers.get(n("Layer", 1.).max(0.) as usize))
                {
                    let mut result = source.clone();
                    if let Some(colors) = result.colors.as_mut() {
                        if !colors.is_empty() {
                            *colors = (0..pixels::count(bank, c.fixture))
                                .map(|i| colors[i % colors.len()])
                                .collect();
                        }
                    }
                    *bank = result;
                } else {
                    *bank = Bank::default();
                }
            } else {
                c.diagnostics
                    .push(format!("Copycat fixture '{reference}' missing"));
                *bank = Bank::default();
            }
        }
        "Create.Radius.Constant" => {
            bank.channels
                .insert("Radius".into(), json!(n("Radius", 1.)));
        }
        "Create.Rotation.Audio.Shake" => {
            let frequency = n("Frequency", 0.).max(0.) as usize;
            let event = *c.host.audio_frames.get(&0).unwrap_or(&0);
            let beat = c
                .host
                .audio
                .get(&0)
                .and_then(|a| a.beats.get(frequency))
                .copied()
                .unwrap_or(false)
                && state.event != event;
            state.event = event;
            if beat && state.random() < n("Probability", 0.5) {
                state.previous = (state.random() * 2. - 1.) * n("Strength", 0.5);
                state.start = Some(c.time);
            }
            let amount = state.previous
                * (1. - (c.time - state.start.unwrap_or(c.time)) / n("Time", 0.).max(0.001))
                    .clamp(0., 1.);
            let mut rotation = [0.; 3];
            rotation[(n("Axis", 0.) as usize).min(2)] = amount;
            bank.rotation = Some(rotation);
        }
        "Create.Color.Pixel.Pulse" | "Create.Color.Pixel.Pulse.Cannon" => {
            let count = pixels::count(bank, c.fixture);
            let audio = c.host.audio.get(&0).map_or(0., |a| a.level);
            let width = (n("Width", 0.6) + audio * n("Width Audio", 0.)).max(1e-6);
            let speed = (n("Speed", 0.5) + audio * n("Speed Audio", 0.)).clamp(0., 1.);
            let step = dt / (100. - 99.5 * speed);
            let reverse =
                flag(p, "Reverse", false) || flag(p, "Direction", false) ^ c.fixture.pixel_reverse;
            if key.ends_with("Cannon") {
                let trigger = flag(p, "Trigger", false);
                if trigger && !state.high {
                    state.samples.push_back((
                        c.time,
                        Bank {
                            value: Some(if reverse { 1. } else { 0. }),
                            ..Bank::default()
                        },
                    ));
                }
                state.high = trigger;
                for (_, pulse) in &mut state.samples {
                    pulse.value =
                        Some(pulse.value.unwrap_or(0.) + step * if reverse { -1. } else { 1. });
                }
                while state.samples.front().is_some_and(|(_, pulse)| {
                    pulse.value.is_some_and(|v| v < -width || v > 1. + width)
                }) || state.samples.len() > 256
                {
                    state.samples.pop_front();
                }
                bank.colors = Some(
                    (0..count)
                        .map(|i| {
                            let x = (i as f64 + 0.5) / count as f64;
                            let value = state
                                .samples
                                .iter()
                                .map(|(_, pulse)| {
                                    (1. - (x - pulse.value.unwrap_or(0.)).abs() / width)
                                        .clamp(0., 1.)
                                })
                                .fold(0., f64::max);
                            [value, value, value, 1.]
                        })
                        .collect(),
                );
            } else {
                state.phase = (state.phase
                    + step * (1. + n("Speed Random", 0.) * (state.random() * 2. - 1.)))
                    .rem_euclid(1.);
                let position = if reverse {
                    1. - state.phase
                } else {
                    state.phase
                };
                bank.colors = Some(
                    (0..count)
                        .map(|i| {
                            let x = (i as f64 + 0.5) / count as f64;
                            let d = (x - position).abs();
                            let level = (1. - d.min(1. - d) / width).clamp(0., 1.);
                            [level, level, level, 1.]
                        })
                        .collect(),
                );
            }
        }
        "Modify.Value.Map.HalfUpDown" => {
            let value = bank.value.unwrap_or(0.);
            bank.value = Some(if flag(p, "Up/Down", false) {
                (2. - value * 2.).clamp(0., 1.)
            } else {
                (value * 2.).clamp(0., 1.)
            });
        }
        "Pick.Group" => {
            if !inputs.is_empty() {
                let index = node
                    .children
                    .iter()
                    .enumerate()
                    .find(|(i, child)| {
                        let group = child
                            .properties
                            .get("Group")
                            .and_then(Value::as_str)
                            .or_else(|| {
                                p.get("Group")
                                    .and_then(Value::as_array)
                                    .and_then(|v| v.get(*i))
                                    .and_then(Value::as_str)
                            })
                            .unwrap_or(text(p, "Group", "Group"));
                        c.fixture.groups.iter().any(|g| g == group)
                    })
                    .map(|(i, _)| i)
                    .unwrap_or(inputs.len() - 1);
                *bank = inputs[index].clone();
                bank.pick = Some(index);
            }
        }
        "Pick.LTP" => {
            for (i, input) in inputs.iter().enumerate() {
                let changed = state
                    .samples
                    .get(i)
                    .is_none_or(|(_, previous)| previous != input);
                let nonzero = input.value.is_some_and(|v| v != 0.)
                    || input.colors.as_ref().is_some_and(|colors| {
                        colors.iter().any(|c| c[..3].iter().any(|v| *v != 0.))
                    });
                if changed && (!flag(p, "AvoidZero", false) || nonzero) {
                    state.index = i;
                }
            }
            state.samples = inputs.iter().cloned().map(|bank| (c.time, bank)).collect();
            if !inputs.is_empty() {
                *bank = inputs[state.index.min(inputs.len() - 1)].clone();
                bank.pick = Some(state.index.min(inputs.len() - 1));
            }
        }
        "Tool.Deprecated.Index.Counter" => {
            state.phase += if n("Period", 0.) <= 0. {
                1.
            } else {
                dt / n("Period", 1.)
            };
            bank.channels
                .insert("Index".into(), json!(state.phase.floor() as u64));
        }
        _ => return false,
    }
    true
}
