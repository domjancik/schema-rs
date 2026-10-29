use super::{flag, hsv, num, rgb};
use crate::model::{Bank, Fixture};
use serde_json::Value;
use std::collections::BTreeMap;
pub(crate) fn supported(key: &str) -> bool {
    matches!(
        key,
        "Create.Color.Pixel.Segment"
            | "Create.Color.Pixel.Rainbow"
            | "Create.Color.Pixel.Pulse.Manual"
            | "Modify.Color.RangeSlice"
            | "Modify.Color.Shift"
            | "Modify.Color.Blur"
            | "Modify.Color.Scramble"
            | "Modify.Color.EvenOdd"
            | "Modify.Color.Complement"
            | "Combine.Color.Max"
            | "Combine.Color.Mix"
            | "Pick.Channel"
            | "Tool.Deprecated.Pick.Index"
            | "Setup.Pixels.Circle"
    )
}
pub(crate) fn count(bank: &Bank, fixture: &Fixture) -> usize {
    bank.pixel_positions
        .as_ref()
        .map_or(fixture.pixel_positions.len(), Vec::len)
        .clamp(1, 512)
}
fn sample(colors: &[[f64; 4]], position: f64, wrap: bool) -> [f64; 4] {
    if colors.is_empty() {
        return [0.; 4];
    }
    let p = if wrap {
        position.rem_euclid(colors.len() as f64)
    } else {
        position.clamp(0., colors.len().saturating_sub(1) as f64)
    };
    let a = p.floor() as usize;
    let b = if wrap {
        (a + 1) % colors.len()
    } else {
        (a + 1).min(colors.len() - 1)
    };
    let t = p.fract();
    std::array::from_fn(|i| colors[a][i] + (colors[b][i] - colors[a][i]) * t)
}
pub(crate) fn evaluate(
    key: &str,
    p: &BTreeMap<String, Value>,
    inputs: &[Bank],
    bank: &mut Bank,
    f: &Fixture,
) -> bool {
    let n = |key, default| num(p, key, default);
    let count = count(bank, f);
    match key {
        "Pick.Channel" | "Tool.Deprecated.Pick.Index" => {
            let index = if key == "Pick.Channel" {
                f.setup_channel
            } else {
                bank.channels
                    .get("Index")
                    .and_then(Value::as_u64)
                    .unwrap_or(0) as usize
            };
            *bank = if inputs.is_empty() {
                Bank::default()
            } else {
                inputs[index % inputs.len()].clone()
            };
            bank.pick = Some(index);
        }
        "Create.Color.Pixel.Segment" => {
            let colors: Vec<_> = inputs
                .iter()
                .map(|b| {
                    b.colors
                        .as_ref()
                        .and_then(|c| c.first())
                        .copied()
                        .unwrap_or([0., 0., 0., 1.])
                })
                .collect();
            let repeat = n("Repeat", 1.).max(1.) as usize;
            let mut out = if colors.is_empty() {
                vec![[0., 0., 0., 1.]; count]
            } else {
                (0..count)
                    .map(|i| colors[(i * repeat * colors.len() / count) % colors.len()])
                    .collect()
            };
            if f.pixel_reverse {
                out.reverse();
            }
            *bank = Bank {
                colors: Some(out),
                ..Bank::default()
            };
        }
        "Create.Color.Pixel.Rainbow" => {
            bank.colors = Some(
                (0..count)
                    .map(|i| {
                        let i = if f.pixel_reverse { count - 1 - i } else { i };
                        rgb([
                            n("Position", 0.5) + i as f64 / count as f64 * n("Width", 0.6),
                            1.,
                            1.,
                            1.,
                        ])
                    })
                    .collect(),
            );
        }
        "Create.Color.Pixel.Pulse.Manual" => {
            let width = n("Width", 0.6).abs().max(1e-9);
            let pos = n("Position", 0.5);
            bank.colors = Some(
                (0..count)
                    .map(|i| {
                        let i = if f.pixel_reverse { count - 1 - i } else { i };
                        let x = (i as f64 + 0.5) / count as f64;
                        let level = (1. - (x - pos).abs() / width).clamp(0., 1.);
                        [level, level, level, 1.]
                    })
                    .collect(),
            );
        }
        "Setup.Pixels.Circle" => {
            let count = n("Count", 3.).round().clamp(1., 512.) as usize;
            let radius = n("Radius", 1.) / 2.;
            let positions = (0..count)
                .map(|i| {
                    let a = i as f64 / count as f64 * std::f64::consts::TAU;
                    [
                        f.position[0] + radius * a.cos(),
                        0.,
                        f.position[2] + radius * a.sin(),
                    ]
                })
                .collect();
            bank.actions.insert(
                format!("7/pixels/{}", f.id),
                super::actions::Action::Pixels {
                    fixture: f.id.clone(),
                    positions,
                },
            );
        }
        "Combine.Color.Max" | "Combine.Color.Mix" => {
            let count = inputs
                .iter()
                .filter_map(|b| b.colors.as_ref().map(Vec::len))
                .max()
                .unwrap_or(0)
                .min(512);
            let mut out: Vec<[f64; 4]> = vec![[0.; 4]; count];
            for (i, result) in out.iter_mut().enumerate() {
                let mut seen = 0;
                for colors in inputs
                    .iter()
                    .filter_map(|b| b.colors.as_ref())
                    .filter(|c| !c.is_empty())
                {
                    for (ch, value) in result.iter_mut().enumerate() {
                        if key.ends_with("Max") {
                            *value = value.max(colors[i % colors.len()][ch]);
                        } else {
                            *value += colors[i % colors.len()][ch];
                        }
                    }
                    seen += 1;
                }
                if key.ends_with("Mix") && seen > 0 {
                    for ch in result {
                        *ch /= seen as f64;
                    }
                }
            }
            bank.colors = Some(out);
        }
        "Modify.Color.RangeSlice"
        | "Modify.Color.Shift"
        | "Modify.Color.Blur"
        | "Modify.Color.Scramble"
        | "Modify.Color.EvenOdd"
        | "Modify.Color.Complement" => {
            if let Some(colors) = bank.colors.as_ref() {
                let mut out = colors.clone();
                let len = colors.len();
                if len > 0 {
                    match key {
                        "Modify.Color.RangeSlice" => {
                            for (i, color) in out.iter_mut().enumerate() {
                                let t = if len <= 1 {
                                    0.
                                } else {
                                    i as f64 / (len - 1) as f64
                                };
                                let position = (n("From", 0.) + (n("To", 1.) - n("From", 0.)) * t)
                                    * len.saturating_sub(1) as f64;
                                *color = sample(colors, position, false);
                            }
                        }

                        "Modify.Color.Shift" => {
                            for (i, color) in out.iter_mut().enumerate() {
                                *color =
                                    sample(colors, i as f64 - n("Offset", 0.) * len as f64, true);
                            }
                        }
                        "Modify.Color.Blur" => {
                            let radius = (n("Value", 0.).abs() * len as f64).round().min(len as f64)
                                as isize;
                            let width = (radius * 2).max(1);
                            for (i, color) in out.iter_mut().enumerate() {
                                let mut total = [0.; 4];
                                for offset in -radius..(-radius + width) {
                                    let c = colors
                                        [(i as isize + offset).rem_euclid(len as isize) as usize];
                                    for ch in 0..4 {
                                        total[ch] += c[ch] / width as f64;
                                    }
                                }
                                *color = total;
                            }
                        }
                        "Modify.Color.Scramble" => {
                            let mut state = super::timed::State {
                                rng: n("Seed", 0.) as i64 as u64,
                                ..Default::default()
                            };
                            for i in 0..(n("Value", 0.).clamp(0., 1.) * len as f64).round() as usize
                            {
                                let j = (state.random() * len as f64) as usize;
                                out.swap(i, j.min(len - 1));
                            }
                        }
                        "Modify.Color.EvenOdd" => {
                            let even = f.setup_channel.is_multiple_of(2);
                            if even && flag(p, "Flip Even", true) {
                                out.reverse();
                            }
                            if !(if even {
                                flag(p, "Show Even", true)
                            } else {
                                flag(p, "Show Odd", true)
                            }) {
                                out.fill([0.; 4]);
                            }
                        }
                        _ => {
                            for color in &mut out {
                                let mut h = hsv(*color);
                                h[0] += 0.5;
                                *color = rgb(h);
                            }
                        }
                    }
                }
                bank.colors = Some(out);
            }
        }
        _ => return false,
    }
    true
}
