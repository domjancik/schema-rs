use super::{flag, num, pixels, text, timed::State};
use crate::{
    inputs::{Audio, Inputs, Texture},
    model::{Bank, Fixture, Node, Snapshot},
};
use serde_json::{json, Value};
use std::collections::BTreeMap;
pub(crate) struct Context<'a> {
    pub fixture: &'a Fixture,
    pub fixtures: &'a [Fixture],
    pub live_layers: &'a BTreeMap<String, Vec<Bank>>,
    pub positions: &'a BTreeMap<String, [f64; 3]>,
    pub host: &'a Inputs,
    pub prior: &'a Snapshot,
    pub time: f64,
    pub frame: u64,
    pub diagnostics: &'a mut Vec<String>,
}
pub(crate) fn supported(key: &str) -> bool {
    matches!(
        key,
        "Create.Value.OSC"
            | "Create.Value.OSCReceived"
            | "Create.Color.OSC"
            | "Create.Value.Control"
            | "Create.Value.Controller.Value"
            | "Create.Value.Controller.Pad"
            | "Create.Value.Keyboard"
            | "Create.Value.XBox"
            | "Create.Value.Fixture.IsSelected"
            | "Create.Value.Global"
            | "Create.Color.Global"
            | "Create.Color.Primary"
            | "Create.Color.External"
            | "Create.Color.Universe"
            | "Create.Reference.SequenceOffset"
            | "Create.Value.SequenceIndex"
            | "Create.Remote"
            | "Create.Value.System.FileExists"
            | "Create.Audio.Input"
            | "Create.Audio.OSC"
            | "Create.Audio.Sample"
            | "Combine.Audio.Add"
            | "Combine.Audio.Multiply"
            | "Convert.Audio.Meter"
            | "Create.Value.Audio.Level"
            | "Create.Value.Audio.PeakPower"
            | "Create.Value.Audio.Pitch"
            | "Create.Value.Audio.Power"
            | "Create.Value.Audio.FFT16"
            | "Create.Value.Audio.Beat"
            | "Create.Color.Audio.FFTPixels"
            | "Create.Color.Audio.Level"
            | "Create.Color.Flash.Beat"
            | "Filter.Audio.Power"
            | "Pick.Audio.Beat"
            | "Pick.Audio.Power"
            | "Pick.BPM"
            | "Pick.Controller"
            | "Pick.Time"
            | "Create.Texture.Color"
            | "Create.Texture.Noise"
            | "Combine.Texture.Multiply"
            | "S.Pong.Position"
            | "S.Pong.Paddle"
            | "Test.Widgets.FilePicker"
    )
}
fn scalar(v: &Value) -> Option<f64> {
    v.as_f64()
        .or_else(|| v.as_bool().map(|b| if b { 1. } else { 0. }))
}
fn diagnostic(c: &mut Context<'_>, message: impl Into<String>) {
    c.diagnostics
        .push(format!("{}: {}", c.fixture.name, message.into()));
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
    if !supported(key) {
        return false;
    }
    let n = |key, default| num(p, key, default);
    let count = pixels::count(bank, c.fixture);
    let slot = n("Slot", 0.).max(0.) as usize;
    match key {
        "Create.Value.OSC" => {
            bank.value = Some(
                c.host
                    .osc
                    .get(text(p, "Name", "Choose"))
                    .and_then(|m| m.arguments.get(n("Index", 0.).max(0.) as usize))
                    .and_then(scalar)
                    .unwrap_or(n("Default", 0.)),
            );
        }
        "Create.Value.OSCReceived" => {
            let message = c.host.osc.get(text(p, "Name", "Choose"));
            let received = message.is_some_and(|m| m.serial != state.event);
            if let Some(message) = message {
                state.event = message.serial;
            }
            bank.value = Some(if received || flag(p, "Simulate", false) {
                1.
            } else {
                0.
            });
        }
        "Create.Color.OSC" => {
            let message = c.host.osc.get(text(p, "Name", "Choose"));
            let color = message
                .and_then(|m| {
                    if let Some(s) = m.arguments.first().and_then(Value::as_str) {
                        crate::scenic::color(s).ok()
                    } else if m.arguments.len() >= 3 {
                        Some([
                            scalar(&m.arguments[0])?,
                            scalar(&m.arguments[1])?,
                            scalar(&m.arguments[2])?,
                            m.arguments.get(3).and_then(scalar).unwrap_or(1.),
                        ])
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| {
                    crate::scenic::color(text(p, "Default", "#ffffff")).unwrap_or([0., 0., 0., 1.])
                });
            bank.colors = Some(vec![color; count]);
        }
        "Create.Value.Control" | "Create.Value.Controller.Value" => {
            let index = if flag(p, "Learn", false) {
                c.host
                    .last_controller
                    .unwrap_or(n("Index", 0.).max(0.) as usize)
            } else {
                n("Index", 0.).max(0.) as usize
            };
            bank.value = Some(
                c.host
                    .controllers
                    .get(&index)
                    .copied()
                    .unwrap_or(n("Default", 0.)),
            );
            bank.channels.insert(
                "ControllerLabel".into(),
                json!({"index":index,"label":text(p,"Label","Untitled")}),
            );
        }
        "Create.Value.Controller.Pad" => {
            bank.value = Some(
                if c.host
                    .pads
                    .get(&(n("Pad", 0.).max(0.) as usize))
                    .copied()
                    .unwrap_or(false)
                {
                    1.
                } else {
                    0.
                },
            )
        }
        "Create.Value.Keyboard" => {
            let name = text(p, "Key Name", "Space");
            bank.value = Some(
                if c.host
                    .keyboard
                    .iter()
                    .any(|key| key.eq_ignore_ascii_case(name))
                {
                    1.
                } else {
                    0.
                },
            );
        }
        "Create.Value.XBox" => {
            bank.value = Some(
                c.host
                    .xbox
                    .get(n("Input", 0.).max(0.) as usize)
                    .copied()
                    .unwrap_or(0.),
            )
        }
        "Create.Value.Fixture.IsSelected" => {
            bank.value = Some(if c.host.selected.contains(&c.fixture.id) {
                1.
            } else {
                0.
            })
        }
        "Create.Value.Global" => {
            let name = text(p, "Name", "Value");
            bank.value = Some(c.host.values.get(name).copied().unwrap_or(0.));
        }
        "Create.Color.Global" => {
            bank.colors = Some(vec![
                c.host
                    .colors
                    .get(&slot)
                    .copied()
                    .unwrap_or([0., 0., 0., 1.]);
                count
            ])
        }
        "Create.Color.Primary" => {
            bank.colors = Some(vec![c.host.primary.unwrap_or([0., 0., 0., 1.]); count])
        }
        "Create.Color.External" => {
            let source = &c.host.external_colors;
            let offset = slot * count;
            bank.colors = Some(
                (0..count)
                    .map(|i| source.get(offset + i).copied().unwrap_or([0., 0., 0., 1.]))
                    .collect(),
            );
        }
        "Create.Color.Universe" => {
            bank.colors = Some(vec![
                super::rgb([
                    (c.fixture.universe as f64 / 16.).rem_euclid(1.),
                    1.,
                    1.,
                    1.
                ]);
                count
            ])
        }
        "Create.Value.SequenceIndex" | "Create.Reference.SequenceOffset" => {
            let group = text(p, "Group", "");
            let sequence = c.host.sequences.get(text(p, "Sequence", "BuiltIn.Name"));
            if let Some(sequence) = sequence {
                let sequence: Vec<_> = sequence
                    .iter()
                    .filter(|id| {
                        group.is_empty()
                            || c.fixtures
                                .iter()
                                .any(|f| &f.id == *id && f.groups.iter().any(|g| g == group))
                    })
                    .collect();
                if let Some(index) = sequence.iter().position(|id| id.as_str() == c.fixture.id) {
                    if key == "Create.Value.SequenceIndex" {
                        bank.value = Some(index as f64 / sequence.len().max(1) as f64);
                    } else {
                        let index = (index as i64 + n("Offset", 0.) as i64)
                            .rem_euclid(sequence.len() as i64)
                            as usize;
                        bank.fixture_reference = Some(sequence[index].to_string());
                    }
                } else {
                    bank.value = None;
                    bank.fixture_reference = None;
                }
            } else {
                diagnostic(c, "Sequence provider has no matching sequence");
                bank.value = None;
            }
        }
        "Create.Remote" => {
            let address = text(p, "Address", "Untitled");
            let index = n("Index", 0.).max(0.) as usize;
            if let Some(remote) = c.host.remote.get(address).and_then(|v| v.get(index)) {
                *bank = bank.clone().merge(remote.clone());
            } else {
                diagnostic(c, "Remote state address is not available");
            }
        }
        "Create.Value.System.FileExists" => {
            bank.value = Some(
                if c.host
                    .files
                    .get(text(p, "File", "Choose File"))
                    .copied()
                    .unwrap_or(false)
                {
                    1.
                } else {
                    0.
                },
            )
        }
        "Test.Widgets.FilePicker" => {
            bank.channels
                .insert("FilePicker".into(), json!(text(p, "File", "")));
        }
        "Create.Audio.Input" => {
            let index = n("Index", 0.).max(0.) as usize;
            if let Some(audio) = c.host.audio.get(&index).filter(|_| {
                c.frame
                    .saturating_sub(*c.host.audio_frames.get(&index).unwrap_or(&c.frame))
                    <= 120
            }) {
                let mut audio = audio.clone();
                audio.samples.truncate(256);
                for v in &mut audio.samples {
                    *v *= n("Gain", 0.1);
                }
                let _ = audio.prepare();
                bank.audio = Some(audio);
            } else {
                diagnostic(c, "Audio input has no current PCM buffer");
                bank.audio = None;
            }
        }
        "Create.Audio.OSC" => {
            let sample_rate = 48000;
            let frequency = n("Frequency", 440.).clamp(0., 24000.);
            let gain = n("Gain", 0.1);
            let waveform = n("Waveform", 0.) as usize;
            let samples = (0..256)
                .map(|i| {
                    let phase = (c.time * frequency + i as f64 * frequency / sample_rate as f64)
                        .rem_euclid(1.);
                    gain * match waveform {
                        0 => (phase * std::f64::consts::TAU).sin(),
                        1 => 1. - 4. * (phase - 0.5).abs(),
                        2 => phase * 2. - 1.,
                        3 => {
                            if phase < n("Symmetry", 0.5) {
                                1.
                            } else {
                                -1.
                            }
                        }
                        _ => state.random() * 2. - 1.,
                    }
                })
                .collect();
            let mut audio = Audio {
                samples,
                sample_rate,
                ..Audio::default()
            };
            let _ = audio.prepare();
            bank.audio = Some(audio);
        }
        "Create.Audio.Sample" => {
            let name = bank
                .channels
                .get("Resource")
                .and_then(Value::as_str)
                .unwrap_or("");
            if let Some(sample) = c.host.audio_samples.get(name) {
                let play = flag(p, "Play", true);
                if play
                    && (!state.playing && flag(p, "Play Restarts", true) || state.start.is_none())
                {
                    state.start = Some(c.time);
                }
                let offset = (c.time - state.start.unwrap_or(c.time))
                    * sample.sample_rate as f64
                    * n("Speed", 1.);
                let repeat = flag(p, "Loop", false);
                let samples = (0..256)
                    .map(|i| {
                        let at = (offset + i as f64 * n("Speed", 1.)).floor() as i64;
                        if !play || sample.samples.is_empty() {
                            0.
                        } else if repeat {
                            sample.samples[at.rem_euclid(sample.samples.len() as i64) as usize]
                                * n("Gain", 0.1)
                        } else {
                            sample.samples.get(at as usize).copied().unwrap_or(0.) * n("Gain", 0.1)
                        }
                    })
                    .collect();
                state.playing = play;
                let mut audio = Audio {
                    samples,
                    sample_rate: sample.sample_rate,
                    ..Audio::default()
                };
                let _ = audio.prepare();
                bank.audio = Some(audio);
            } else {
                diagnostic(c, "Sample resource is not loaded");
                bank.audio = None;
            }
        }
        "Combine.Audio.Add" | "Combine.Audio.Multiply" => {
            let streams: Vec<_> = inputs.iter().filter_map(|b| b.audio.as_ref()).collect();
            if let Some(first) = streams.first() {
                let len = streams
                    .iter()
                    .map(|a| a.samples.len())
                    .min()
                    .unwrap_or(0)
                    .min(4096);
                let mut audio = Audio {
                    sample_rate: first.sample_rate,
                    samples: vec![if key.ends_with("Add") { 0. } else { 1. }; len],
                    ..Audio::default()
                };
                for stream in streams {
                    if stream.sample_rate != audio.sample_rate {
                        diagnostic(c, "Audio sample rates must match before combining");
                        bank.audio = None;
                        return true;
                    }
                    for (out, input) in audio.samples.iter_mut().zip(&stream.samples) {
                        if key.ends_with("Add") {
                            *out += input;
                        } else {
                            *out *= input;
                        }
                    }
                }
                let _ = audio.prepare();
                bank.audio = Some(audio);
            } else {
                bank.audio = None;
            }
        }
        "Convert.Audio.Meter" => {
            if let Some(audio) = &bank.audio {
                let level = if audio.samples.is_empty() {
                    0.
                } else {
                    (audio.samples.iter().map(|v| v * v).sum::<f64>() / audio.samples.len() as f64)
                        .sqrt()
                };
                bank.value = Some(level);
                bank.channels.insert("AudioMeter".into(),json!({"rms":level,"peak":audio.samples.iter().map(|v|v.abs()).fold(0.,f64::max)}));
            } else {
                bank.value = None;
            }
        }
        "Create.Value.Audio.Level"
        | "Create.Value.Audio.PeakPower"
        | "Create.Value.Audio.Pitch"
        | "Create.Value.Audio.Power"
        | "Create.Value.Audio.FFT16"
        | "Create.Value.Audio.Beat"
        | "Create.Color.Audio.FFTPixels"
        | "Create.Color.Audio.Level"
        | "Create.Color.Flash.Beat"
        | "Filter.Audio.Power"
        | "Pick.Audio.Beat"
        | "Pick.Audio.Power"
        | "Pick.BPM" => {
            let audio = c.host.audio.get(&slot).filter(|_| {
                c.frame
                    .saturating_sub(*c.host.audio_frames.get(&slot).unwrap_or(&c.frame))
                    <= 120
            });
            if let Some(audio) = audio {
                let event = *c.host.audio_frames.get(&slot).unwrap_or(&0);
                let beat = audio
                    .beats
                    .get(n("Frequency", 0.).max(0.) as usize)
                    .copied()
                    .unwrap_or(false)
                    && state.event != event;
                let input_changed = state.event != event;
                state.event = event;
                match key {
                    "Create.Value.Audio.Level" => {
                        let a = n("Low", 0.);
                        let b = n("High", 1.);
                        bank.value = Some(if (b - a).abs() < 1e-9 {
                            0.
                        } else {
                            ((audio.level - a) / (b - a)).clamp(0., 1.)
                        });
                    }
                    "Create.Value.Audio.PeakPower" => bank.value = Some(audio.peak),
                    "Create.Value.Audio.Pitch" => bank.value = Some(audio.pitch),
                    "Create.Value.Audio.Power" => {
                        bank.value = Some(if flag(p, "Current/Overall", false) {
                            audio.overall_power
                        } else {
                            audio.power
                        })
                    }
                    "Create.Value.Audio.FFT16" => {
                        let index = n("Frequency Band", 0.).max(0.) as usize;
                        bank.value = Some(
                            audio.bands.get(index).copied().unwrap_or_else(|| {
                                if flag(p, "Average/Max", false) {
                                    audio.bands.iter().sum::<f64>()
                                        / audio.bands.len().max(1) as f64
                                } else {
                                    audio.bands.iter().copied().fold(0., f64::max)
                                }
                            }) * n("Gain", 1.),
                        );
                    }
                    "Create.Value.Audio.Beat" => {
                        bank.value = Some(if beat && state.random() < n("Probability", 0.5) {
                            1.
                        } else {
                            0.
                        })
                    }
                    "Create.Color.Audio.FFTPixels" => {
                        bank.colors = Some(
                            (0..count)
                                .map(|i| {
                                    let level = audio.bands.get(i % 16).copied().unwrap_or(0.)
                                        * n("Gain", 1.);
                                    [level, level, level, 1.]
                                })
                                .collect(),
                        )
                    }
                    "Create.Color.Audio.Level" => {
                        let level = if flag(p, "Invert Level", false) {
                            1. - audio.level
                        } else {
                            audio.level
                        } * n("Value", 0.);
                        bank.colors = Some(vec![[level, level, level, 1.]; count]);
                    }
                    "Create.Color.Flash.Beat" => {
                        if beat
                            && audio.level >= n("Threshold", 0.5)
                            && state.random() < n("Probability", 0.5)
                        {
                            state.start = Some(c.time);
                        }
                        let elapsed = c.time - state.start.unwrap_or(c.time);
                        let attack = n("Attack", 0.);
                        let level = if state.start.is_none() {
                            0.
                        } else if elapsed < attack {
                            elapsed / attack
                        } else {
                            (1. - (elapsed - attack) / n("Decay", 1.).max(0.001)).clamp(0., 1.)
                        };
                        bank.colors = Some(vec![[level, level, level, 1.]; count]);
                    }
                    "Filter.Audio.Power" => {
                        let above = audio.power > n("Threshold", 0.);
                        if above != flag(p, "Below/Above", true) {
                            *bank = Bank::default();
                        }
                    }
                    "Pick.Audio.Beat" => {
                        if beat
                            && audio.level >= n("Threshold", 0.5)
                            && state.random() < n("Probability", 0.5)
                        {
                            state.index = state.index.wrapping_add(1);
                        }
                        if !inputs.is_empty() {
                            *bank = inputs[state.index % inputs.len()].clone();
                            bank.pick = Some(state.index % inputs.len());
                        }
                    }
                    _ => {
                        if !inputs.is_empty() {
                            if (key == "Pick.BPM" || c.time >= state.until) && input_changed {
                                let threshold = if key == "Pick.BPM" { "BPM" } else { "Power" };
                                let value = if key == "Pick.BPM" {
                                    audio.bpm
                                } else {
                                    audio.power
                                };
                                state.index = node
                                    .children
                                    .iter()
                                    .enumerate()
                                    .filter(|(_, child)| {
                                        child
                                            .properties
                                            .get(threshold)
                                            .and_then(crate::scenic::number)
                                            .unwrap_or(n(threshold, 0.5))
                                            <= value
                                    })
                                    .map(|(i, _)| i)
                                    .next_back()
                                    .unwrap_or(0);
                                state.until = c.time + n("Wait", 1.).max(0.);
                            }
                            *bank = inputs[state.index % inputs.len()].clone();
                            bank.pick = Some(state.index % inputs.len());
                        }
                    }
                }
            } else {
                diagnostic(c, "Audio analysis provider is not configured");
                if key.starts_with("Filter.") {
                    *bank = Bank::default();
                } else {
                    bank.value = None;
                    bank.colors = None;
                }
            }
        }
        "Pick.Controller" => {
            if !inputs.is_empty() {
                let index = c.host.last_controller.unwrap_or(0) % inputs.len();
                *bank = inputs[index].clone();
                bank.pick = Some(index);
            }
        }
        "Pick.Time" => {
            if !inputs.is_empty() {
                let time = c.host.clock_seconds as f64 / 3600.;
                let thresholds: Vec<_> = node
                    .children
                    .iter()
                    .map(|child| {
                        num(&child.properties, "Hour", n("Hour", 0.))
                            + num(&child.properties, "Minutes", n("Minutes", 0.)) / 60.
                    })
                    .collect();
                let index = thresholds
                    .iter()
                    .enumerate()
                    .filter(|(_, hour)| **hour < time)
                    .map(|(i, _)| i)
                    .next_back()
                    .unwrap_or(inputs.len() - 1);
                *bank = inputs[index].clone();
                bank.pick = Some(index);
            }
        }
        "Create.Texture.Color" => {
            let pixels = bank
                .colors
                .clone()
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| vec![[0., 0., 0., 1.]]);
            bank.texture = Some(Texture {
                width: pixels.len(),
                height: 1,
                pixels,
            });
        }
        "Create.Texture.Noise" => {
            let mut pixels = Vec::with_capacity(32 * 32);
            for _ in 0..32 * 32 {
                let value = state.random();
                pixels.push([value, value, value, 1.]);
            }
            bank.texture = Some(Texture {
                width: 32,
                height: 32,
                pixels,
            });
        }
        "Combine.Texture.Multiply" => {
            let textures: Vec<_> = inputs.iter().filter_map(|b| b.texture.as_ref()).collect();
            if let Some(first) = textures.first() {
                let mut texture = (*first).clone();
                for next in textures.iter().skip(1) {
                    for y in 0..texture.height {
                        for x in 0..texture.width {
                            let nx = x * next.width / texture.width;
                            let ny = y * next.height / texture.height;
                            for ch in 0..4 {
                                texture.pixels[y * texture.width + x][ch] *=
                                    next.pixels[ny * next.width + nx][ch];
                            }
                        }
                    }
                }
                bank.texture = Some(texture);
            } else {
                bank.texture = None;
            }
        }
        "S.Pong.Position" => {
            if let Some(pong) = &c.host.pong {
                bank.position = Some(pong.ball);
            } else {
                diagnostic(c, "Pong state is not configured");
                bank.position = None;
            }
        }
        "S.Pong.Paddle" => {
            if let Some(pong) = &c.host.pong {
                let position = pong
                    .paddles
                    .get(c.fixture.setup_channel % pong.paddles.len().max(1))
                    .copied()
                    .unwrap_or(0.5);
                let width = if pong.width > 0. { pong.width } else { 0.2 };
                bank.colors = Some(
                    (0..count)
                        .map(|i| {
                            let x = (i as f64 + 0.5) / count as f64;
                            let v = if (x - position).abs() < width / 2. {
                                1.
                            } else {
                                0.
                            };
                            [v, v, v, 1.]
                        })
                        .collect(),
                );
            } else {
                diagnostic(c, "Pong state is not configured");
                bank.colors = None;
            }
        }
        _ => return false,
    }
    true
}
