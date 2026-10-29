use super::{flag, num};
use crate::model::Bank;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, VecDeque};
#[derive(Default, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct State {
    pub event: u64,
    pub last_time: Option<f64>,
    pub start: Option<f64>,
    pub previous: f64,
    pub output: f64,
    pub velocity: f64,
    pub high: bool,
    pub latch: bool,
    pub until: f64,
    pub index: usize,
    pub phase: f64,
    pub rng: u64,
    pub colors: Vec<[f64; 4]>,
    pub samples: VecDeque<(f64, Bank)>,
    pub playing: bool,
}
impl State {
    pub fn validate(&self, time: f64) -> crate::model::Result<()> {
        if [
            self.previous,
            self.output,
            self.velocity,
            self.until,
            self.phase,
        ]
        .iter()
        .any(|v| !v.is_finite())
            || self
                .last_time
                .is_some_and(|v| !v.is_finite() || v < 0. || v > time + 1e-6)
            || self
                .start
                .is_some_and(|v| !v.is_finite() || v < 0. || v > time + 1e-6)
            || self.colors.len() > 512
            || self.colors.iter().flatten().any(|v| !v.is_finite())
            || self.samples.len() > 1024
        {
            return Err("Invalid native runtime state".into());
        }
        for (timestamp, bank) in &self.samples {
            if !timestamp.is_finite() {
                return Err("Invalid sampled timestamp".into());
            }
            bank.validate_payload()?;
        }
        Ok(())
    }
    pub fn delta(&mut self, time: f64) -> f64 {
        let dt = (time - self.last_time.unwrap_or(time)).max(0.);
        self.last_time = Some(time);
        dt
    }
    pub fn random(&mut self) -> f64 {
        self.rng = self.rng.wrapping_add(0x9e3779b97f4a7c15);
        let mut x = self.rng;
        x = (x ^ (x >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        x = (x ^ (x >> 27)).wrapping_mul(0x94d049bb133111eb);
        ((x ^ (x >> 31)) >> 11) as f64 / (1_u64 << 53) as f64
    }
}
pub(crate) fn supported(key: &str) -> bool {
    matches!(
        key,
        "Modify.Value.Edge.Rising"
            | "Modify.Value.Edge.Falling"
            | "Modify.Value.Toggle"
            | "Modify.Value.Hold"
            | "Modify.Value.Cooldown"
            | "Modify.Value.AttackDecay"
            | "Modify.Value.Damper"
            | "Modify.Value.Smooth"
            | "Modify.Value.SmoothAcceleration"
            | "Modify.Value.ADSR"
            | "Modify.Value.RunningMap"
            | "Modify.Value.Shape"
            | "Modify.Delay"
            | "Modify.Repeat"
            | "Convert.Speed.PositionToTarget"
            | "Pick.Alternate"
            | "Create.Value.Trigger"
            | "Create.Rotation.Timed"
            | "Create.Color.Flash.BPM"
            | "Create.Color.Flash.Timed"
            | "Modify.Color.AttackDecay"
            | "Modify.Color.Paint.AlphaBased"
            | "Modify.Color.Paint.BlackAndWhite"
            | "Modify.Color.Strobe"
            | "Modify.Color.Flickerize"
            | "Modify.Deprecated.Color.Flickerize"
    )
}
pub(crate) fn ease(t: f64, shape: usize, mode: usize) -> f64 {
    let t = t.clamp(0., 1.);
    fn bounce(t: f64) -> f64 {
        let n = 7.5625;
        let d = 2.75;
        if t < 1. / d {
            n * t * t
        } else if t < 2. / d {
            let t = t - 1.5 / d;
            n * t * t + 0.75
        } else if t < 2.5 / d {
            let t = t - 2.25 / d;
            n * t * t + 0.9375
        } else {
            let t = t - 2.625 / d;
            n * t * t + 0.984375
        }
    }
    let inside = |t: f64| match shape {
        0 => t,
        1 => 1. - (t * std::f64::consts::FRAC_PI_2).cos(),
        2..=5 => t.powi(shape as i32),
        6 => {
            if t == 0. {
                0.
            } else {
                2_f64.powf(10. * t - 10.)
            }
        }
        7 => 1. - (1. - t * t).max(0.).sqrt(),
        8 => {
            if t == 0. || t == 1. {
                t
            } else {
                -2_f64.powf(10. * t - 10.) * ((t * 10. - 10.75) * std::f64::consts::TAU / 3.).sin()
            }
        }
        9 => 2.70158 * t * t * t - 1.70158 * t * t,
        _ => 1. - bounce(1. - t),
    };
    match mode {
        0 => inside(t),
        1 => 1. - inside(1. - t),
        2 => {
            if t < 0.5 {
                inside(t * 2.) / 2.
            } else {
                1. - inside((1. - t) * 2.) / 2.
            }
        }
        _ => {
            if t < 0.5 {
                (1. - inside(1. - t * 2.)) / 2.
            } else {
                (1. + inside(t * 2. - 1.)) / 2.
            }
        }
    }
}
fn approach(current: f64, target: f64, time: f64, dt: f64) -> f64 {
    if time <= 0. {
        target
    } else {
        current + (target - current) * (1. - (-dt / time).exp())
    }
}
pub(crate) fn evaluate(
    key: &str,
    p: &BTreeMap<String, Value>,
    inputs: &[Bank],
    bank: &mut Bank,
    state: &mut State,
    time: f64,
) -> bool {
    if !supported(key) {
        return false;
    }
    let n = |key, default| num(p, key, default);
    let v = bank.value.unwrap_or(0.);
    let first = state.last_time.is_none();
    let dt = state.delta(time);
    let high = v > n("Threshold", 0.5);
    let rising = high && !state.high;
    let falling = !high && state.high;
    match key {
        "Modify.Value.Edge.Rising" => bank.value = Some(if rising { 1. } else { 0. }),
        "Modify.Value.Edge.Falling" => bank.value = Some(if falling { 1. } else { 0. }),
        "Modify.Value.Toggle" => {
            if rising {
                state.latch = !state.latch;
            }
            bank.value = Some(if state.latch { 1. } else { 0. });
        }
        "Modify.Value.Hold" => {
            if high {
                state.until = time + n("Time", 0.).max(0.);
            }
            bank.value = Some(if high || time < state.until { 1. } else { 0. });
        }
        "Modify.Value.Cooldown" => {
            if flag(p, "Reset", false) {
                state.until = 0.;
            }
            let pulse = high && time >= state.until;
            if pulse {
                state.until = time + n("Duration", 2.).max(0.001);
            }
            bank.value = Some(if pulse { 1. } else { 0. });
            bank.progress =
                Some(1. - ((state.until - time) / n("Duration", 2.).max(0.001)).clamp(0., 1.));
        }
        "Modify.Value.AttackDecay" | "Modify.Value.Damper" => {
            let duration = if key == "Modify.Value.Damper" {
                n("Time", 0.)
            } else if v > state.output {
                n("Attack", 0.01)
            } else {
                n("Decay", 0.01)
            };
            state.output = if first && key == "Modify.Value.Damper" {
                v
            } else {
                approach(state.output, v, duration, dt)
            };
            bank.value = Some(state.output);
        }
        "Modify.Value.Smooth" => {
            let speed = if v > state.output {
                n("Up Speed", 0.)
            } else {
                n("Down Speed", 1.)
            }
            .max(0.);
            let delta = (v - state.output).clamp(-speed * dt, speed * dt);
            state.output += delta;
            bank.value = Some(state.output);
        }
        "Modify.Value.SmoothAcceleration" => {
            let acceleration = if v > state.output {
                n("Up Acceleration", 0.)
            } else {
                n("Down Acceleration", 1.)
            }
            .max(0.);
            let direction = (v - state.output).signum();
            state.velocity += direction * acceleration * dt;
            let next = state.output + state.velocity * dt;
            if (v - next).signum() != direction {
                state.output = v;
                state.velocity = 0.;
            } else {
                state.output = next;
            }
            bank.value = Some(state.output);
        }
        "Modify.Value.ADSR" => {
            if rising {
                state.start = Some(time);
            }
            let elapsed = time - state.start.unwrap_or(time);
            let attack = n("Attack", 0.).max(0.);
            let decay = n("Decay", 1.).max(0.001);
            state.output = if state.start.is_none() {
                0.
            } else if elapsed < attack {
                elapsed / attack
            } else {
                (1. - (elapsed - attack) / decay).clamp(0., 1.)
            };
            bank.value = Some(state.output);
        }
        "Modify.Value.RunningMap" => {
            state.samples.push_back((
                time,
                Bank {
                    value: Some(v),
                    ..Bank::default()
                },
            ));
            while state.samples.len() > 1024
                || state
                    .samples
                    .front()
                    .is_some_and(|(t, _)| time - t > n("Duration", 2.).max(0.))
            {
                state.samples.pop_front();
            }
            let min = state
                .samples
                .iter()
                .filter_map(|(_, b)| b.value)
                .fold(f64::INFINITY, f64::min);
            let max = state
                .samples
                .iter()
                .filter_map(|(_, b)| b.value)
                .fold(f64::NEG_INFINITY, f64::max);
            let target = if (max - min).abs() < 1e-12 {
                n("Minimum", 0.)
            } else {
                n("Minimum", 0.) + (v - min) / (max - min) * (n("Maximum", 1.) - n("Minimum", 0.))
            };
            state.output = approach(state.output, target, n("Filter Time", 1.), dt);
            bank.value = Some(if flag(p, "Clamp", true) {
                state.output.clamp(
                    n("Minimum", 0.).min(n("Maximum", 1.)),
                    n("Minimum", 0.).max(n("Maximum", 1.)),
                )
            } else {
                state.output
            });
        }
        "Modify.Value.Shape" => {
            bank.value = Some(ease(v, n("Shape", 0.) as usize, n("Ease", 0.) as usize))
        }
        "Modify.Delay" => {
            state.samples.push_back((time, bank.clone()));
            let target = time - n("Duration", 1.).max(0.);
            let mut result = None;
            while state
                .samples
                .front()
                .is_some_and(|(t, _)| *t <= target + 1e-9)
            {
                result = state.samples.pop_front().map(|(_, b)| b);
            }
            if let Some(result) = result {
                state.output = result.value.unwrap_or(0.);
                state.colors = result.colors.clone().unwrap_or_default();
                state.samples.push_front((target, result.clone()));
                *bank = result;
            } else {
                *bank = Bank::default();
            }
            while state.samples.len() > 1024 {
                state.samples.pop_front();
            }
        }
        "Modify.Repeat" => {
            let requested = flag(p, "Record", false);
            if !requested {
                state.latch = false;
            }
            // The source resets on either Record edge. Its counter > 400 guard
            // permits samples 0..=401, then switches to playback. Keep that guard
            // in runtime state rather than mutating authored properties each frame.
            let record = requested && !state.latch;
            if record != state.high {
                state.index = 0;
                if record {
                    state.samples.clear();
                }
            } else if flag(p, "Play", true) || record {
                state.index = state.index.wrapping_add(1);
            }
            if record {
                state.samples.push_back((time, bank.clone()));
                if state.samples.len() >= 402 {
                    state.latch = true;
                }
            } else if !state.samples.is_empty() {
                let count = state.samples.len();
                let forward = state.index % count;
                let index = match n("Mode", 0.) as usize {
                    // Source Switch(Boolean) selects its second input on even passes.
                    1 if (state.index / count).is_multiple_of(2) => count - 1 - forward,
                    2 => count - 1 - forward,
                    _ => forward,
                };
                *bank = state.samples[index].1.clone();
            }
            state.high = record;
            return true;
        }
        "Convert.Speed.PositionToTarget" => {
            let a = inputs.first().and_then(|b| b.value).unwrap_or(0.);
            let b = inputs.get(1).and_then(|b| b.value).unwrap_or(0.);
            let difference = b - a;
            state.output += (difference.abs() - state.output).clamp(-100., 1.5);
            *bank = Bank {
                speed: Some(difference.signum() * state.output),
                ..Bank::default()
            };
        }
        "Pick.Alternate" => {
            if !inputs.is_empty() {
                if first {
                    state.until = time + n("Period", 2.).max(0.001);
                }
                if !flag(p, "Pause", false)
                    && (time >= state.until || flag(p, "Skip", false) && !state.latch)
                {
                    state.index = if flag(p, "Random", false) {
                        (state.random() * inputs.len() as f64) as usize
                    } else if flag(p, "Reverse", false) {
                        (state.index + inputs.len() - 1) % inputs.len()
                    } else {
                        (state.index + 1) % inputs.len()
                    };
                    state.until = time
                        + n("Period", 2.).max(0.001)
                            * (1. + (state.random() * 2. - 1.) * n("Randomness", 0.).clamp(0., 1.));
                }
                state.latch = flag(p, "Skip", false);
                *bank = inputs[state.index % inputs.len()].clone();
                bank.pick = Some(state.index % inputs.len());
            }
        }
        "Create.Value.Trigger" => {
            if first {
                state.until = time;
            }
            if time >= state.until {
                state.start = Some(time);
                state.until = time
                    + n("Interval", 1.).max(0.001)
                        * (1. + (state.random() * 2. - 1.) * n("Randomness", 0.5).clamp(0., 1.));
                bank.value = Some(1.);
            } else {
                bank.value = Some(
                    if time - state.start.unwrap_or(time) < n("Hold", 0.).max(0.) {
                        1.
                    } else {
                        0.
                    },
                );
            }
        }
        "Create.Rotation.Timed" => {
            let period = 100. - 99.5 * n("Speed", 0.5).clamp(0., 1.);
            state.phase = (state.phase + dt / period).rem_euclid(1.);
            let mut rotation = bank.rotation.unwrap_or([0.; 3]);
            rotation[(n("Axis", 0.) as usize).min(2)] = state.phase;
            bank.rotation = Some(rotation);
        }
        "Create.Color.Flash.BPM" | "Create.Color.Flash.Timed" => {
            let period = if key.ends_with("BPM") {
                60. / n("BPM", 126.).max(1.)
            } else {
                n("Interval", 2.).max(0.001)
            };
            if first || time >= state.until {
                state.start = Some(time);
                state.until = time
                    + period
                        * (1. + (state.random() * 2. - 1.) * n("Randomness", 0.).clamp(0., 1.));
            }
            let elapsed = time - state.start.unwrap_or(time);
            let attack = n("Attack", 0.).max(0.);
            let intensity = if elapsed < attack {
                elapsed / attack
            } else {
                (1. - (elapsed - attack) / n("Decay", 1.).max(0.001)).clamp(0., 1.)
            };
            bank.colors = Some(vec![[intensity, intensity, intensity, 1.]]);
            bank.progress = Some((elapsed / period).clamp(0., 1.));
        }
        "Modify.Color.AttackDecay" => {
            if let Some(colors) = bank.colors.as_ref() {
                if state.colors.len() != colors.len() {
                    state.colors = vec![[0.; 4]; colors.len()];
                }
                for (out, input) in state.colors.iter_mut().zip(colors) {
                    for i in 0..4 {
                        let duration = if input[i] > out[i] {
                            n("Attack", 0.)
                        } else {
                            n("Decay", 0.)
                        };
                        out[i] = approach(out[i], input[i], duration, dt);
                    }
                }
                bank.colors = Some(state.colors.clone());
            }
        }
        "Modify.Color.Paint.AlphaBased" | "Modify.Color.Paint.BlackAndWhite" => {
            if let Some(colors) = bank.colors.as_ref() {
                if flag(p, "Clear", false) || state.colors.len() != colors.len() {
                    state.colors = vec![[0.; 4]; colors.len()];
                }
                let paint =
                    crate::scenic::color(super::text(p, "Color", "#ffffff")).unwrap_or([1.; 4]);
                for (out, input) in state.colors.iter_mut().zip(colors) {
                    let alpha = if key.ends_with("BlackAndWhite") {
                        super::hsv(*input)[2]
                    } else {
                        input[3]
                    }
                    .clamp(0., 1.);
                    for i in 0..4 {
                        let incoming = if key.ends_with("BlackAndWhite") {
                            paint[i]
                        } else {
                            input[i]
                        };
                        out[i] += (incoming - out[i]) * alpha;
                    }
                }
                bank.colors = Some(state.colors.clone());
            }
        }
        "Modify.Color.Strobe" => {
            if flag(p, "Enabled", true) {
                let on = (time / n("Period", 0.1).max(0.001)).rem_euclid(1.) < 0.5;
                if let Some(colors) = bank.colors.as_mut() {
                    for color in colors {
                        if !on {
                            color[..3].fill(0.);
                        } else if flag(p, "Maximum Lightness", false) {
                            let mut h = super::hsv(*color);
                            h[2] = 1.;
                            *color = super::rgb(h);
                        }
                    }
                }
            }
        }
        "Modify.Color.Flickerize" | "Modify.Deprecated.Color.Flickerize" => {
            if flag(p, "Enabled", true) {
                let intensity = state.random();
                if let Some(colors) = bank.colors.as_mut() {
                    for color in colors {
                        for channel in color.iter_mut().take(3) {
                            *channel *= intensity;
                        }
                    }
                }
            }
        }
        _ => return false,
    }
    state.high = high;
    state.previous = v;
    true
}

#[cfg(test)]
mod repeat_tests {
    use super::*;
    use serde_json::json;
    fn frame(s: &mut State, record: bool, play: bool, mode: u8, v: f64) -> f64 {
        let p = BTreeMap::from([
            ("Record".into(), json!(record)),
            ("Play".into(), json!(play)),
            ("Mode".into(), json!(mode)),
        ]);
        let mut bank = Bank {
            value: Some(v),
            position: Some([v, 2., 3.]),
            ..Bank::default()
        };
        evaluate(
            "Modify.Repeat",
            &p,
            &[],
            &mut bank,
            s,
            s.last_time.unwrap_or(0.) + 0.1,
        );
        assert_eq!(bank.position.unwrap()[0], bank.value.unwrap());
        bank.value.unwrap()
    }
    fn recorded() -> State {
        let mut s = State::default();
        for v in [0., 1., 2.] {
            assert_eq!(frame(&mut s, true, true, 0, v), v);
        }
        s
    }
    #[test]
    fn repeat_ping_pong_starts_reverse_and_includes_both_endpoint_frames() {
        let mut s = recorded();
        let actual: Vec<_> = (0..9).map(|_| frame(&mut s, false, true, 1, 99.)).collect();
        assert_eq!(actual, vec![2., 1., 0., 0., 1., 2., 2., 1., 0.]);
    }
    #[test]
    fn repeat_holds_last_frame_and_resumes_on_next_frame() {
        let mut s = recorded();
        assert_eq!(frame(&mut s, false, true, 0, 99.), 0.);
        assert_eq!(frame(&mut s, false, true, 0, 99.), 1.);
        for _ in 0..3 {
            assert_eq!(frame(&mut s, false, false, 0, 99.), 1.);
        }
        assert_eq!(frame(&mut s, false, true, 0, 99.), 2.);
        assert_eq!(frame(&mut s, true, true, 0, 7.), 7.);
        assert_eq!(frame(&mut s, false, true, 2, 99.), 7.);
    }
    #[test]
    fn repeat_recording_guard_stops_after_original_402_sample_window() {
        let mut s = State::default();
        for i in 0..402 {
            assert_eq!(frame(&mut s, true, true, 0, i as f64), i as f64);
        }
        assert_eq!(frame(&mut s, true, true, 0, 999.), 0.);
        assert_eq!(s.samples.len(), 402);
        assert_eq!(frame(&mut s, true, true, 0, 999.), 1.);
        frame(&mut s, false, true, 0, 999.);
        assert_eq!(frame(&mut s, true, true, 0, 7.), 7.);
        assert_eq!(s.samples.len(), 1);
    }
    #[test]
    fn repeat_reverse_and_serialized_playback_continue_identically() {
        let mut s = recorded();
        assert_eq!(frame(&mut s, false, true, 2, 99.), 2.);
        let mut restored: State =
            serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        for expected in [1., 0., 2., 1.] {
            assert_eq!(frame(&mut s, false, true, 2, 99.), expected);
            assert_eq!(frame(&mut restored, false, true, 2, 99.), expected);
        }
    }
}
