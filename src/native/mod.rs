pub(crate) mod actions;
pub(crate) mod pixels;
pub(crate) mod providers;
pub(crate) mod spatial;
pub(crate) mod timed;
use crate::model::{Bank, Fixture};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;
#[derive(Deserialize)]
pub(crate) struct Spec {
    #[serde(default)]
    pub basis: String,
    pub defaults: BTreeMap<String, Value>,
    pub types: BTreeMap<String, String>,
    pub options: BTreeMap<String, Vec<String>>,
}
pub(crate) fn specs() -> &'static BTreeMap<String, Spec> {
    static SPECS: std::sync::OnceLock<BTreeMap<String, Spec>> = std::sync::OnceLock::new();
    SPECS
        .get_or_init(|| serde_json::from_str(include_str!("catalog.json")).expect("native catalog"))
}
pub(crate) fn supported(key: &str) -> bool {
    pure_supported(key)
        || timed::supported(key)
        || pixels::supported(key)
        || providers::supported(key)
        || spatial::supported(key)
        || actions::supported(key)
}
pub(crate) fn pure_supported(key: &str) -> bool {
    matches!(
        key,
        "Create.Duration.Constant"
            | "Create.Speed.Constant"
            | "Convert.Value.ToDuration"
            | "Convert.Value.ToSpeed"
            | "Create.AnimationWheel.Position"
            | "Create.AnimationWheel.Rotation"
            | "Create.BeamShaper.Position"
            | "Create.Blade.Position"
            | "Create.Blade.Rotation"
            | "Create.Focus.Value"
            | "Create.Frost.Value"
            | "Create.Gobo.Constant"
            | "Create.Gobo.Rotate.Value"
            | "Create.Gobo.Shake.Constant"
            | "Create.Iris.Value"
            | "Create.Macro.Constant"
            | "Create.Prism.Rotate"
            | "Create.Prism.Select"
            | "Create.Temperature.Constant"
            | "Create.Zoom.Value"
            | "Modify.ColorWheel.Offset"
            | "Create.Rotation.Constant"
            | "Create.Rotation.PanTilt"
            | "Create.Rotation.XYZ"
            | "Create.Rotation.Perpendicular"
            | "Combine.Value.Boolean"
            | "Combine.Value.Compare"
            | "Combine.Color.Distance"
            | "Modify.Value.SelectRange"
            | "Modify.Value.Wave"
            | "Modify.Color.Alpha.AsAlpha"
            | "Modify.Color.Clamp"
            | "Modify.Color.Hue.Rotate"
            | "Modify.Color.Saturation.Set"
            | "Create.Resource.Constant"
            | "Create.Shapes.ResourceMode.Constant"
    )
}
pub(crate) fn num(p: &BTreeMap<String, Value>, key: &str, default: f64) -> f64 {
    p.get(key)
        .and_then(crate::scenic::number)
        .unwrap_or(default)
}
pub(crate) fn flag(p: &BTreeMap<String, Value>, key: &str, default: bool) -> bool {
    p.get(key).and_then(Value::as_bool).unwrap_or(default)
}
pub(crate) fn text<'a>(p: &'a BTreeMap<String, Value>, key: &str, default: &'a str) -> &'a str {
    p.get(key).and_then(Value::as_str).unwrap_or(default)
}
fn slot(bank: &mut Bank, key: &str, index: f64, value: f64) {
    let map = bank.channels.entry(key.into()).or_insert(json!({}));
    if let Some(map) = map.as_object_mut() {
        map.insert((index as i64).to_string(), json!(value));
    }
}
pub(crate) fn hsv(color: [f64; 4]) -> [f64; 4] {
    let max = color[0].max(color[1]).max(color[2]);
    let min = color[0].min(color[1]).min(color[2]);
    let d = max - min;
    let h = if d.abs() < 1e-12 {
        0.
    } else if max == color[0] {
        ((color[1] - color[2]) / d).rem_euclid(6.) / 6.
    } else if max == color[1] {
        ((color[2] - color[0]) / d + 2.) / 6.
    } else {
        ((color[0] - color[1]) / d + 4.) / 6.
    };
    [
        h,
        if max.abs() < 1e-12 { 0. } else { d / max },
        max,
        color[3],
    ]
}
pub(crate) fn rgb(hsv: [f64; 4]) -> [f64; 4] {
    let [h, s, v, a] = hsv;
    let h = h.rem_euclid(1.) * 6.;
    let c = v * s;
    let x = c * (1. - (h.rem_euclid(2.) - 1.).abs());
    let m = v - c;
    let [r, g, b] = match h as usize {
        0 => [c, x, 0.],
        1 => [x, c, 0.],
        2 => [0., c, x],
        3 => [0., x, c],
        4 => [x, 0., c],
        _ => [c, 0., x],
    };
    [r + m, g + m, b + m, a]
}
fn speed(value: f64, range: usize) -> f64 {
    let (a, b) = [(0., 1.), (0., -1.), (-1., 1.)][range.min(2)];
    a + (b - a) * value
}
pub(crate) fn evaluate(
    key: &str,
    p: &BTreeMap<String, Value>,
    inputs: &[Bank],
    bank: &mut Bank,
    _fixture: &Fixture,
) -> bool {
    let n = |key, default| num(p, key, default);
    let v = bank.value.unwrap_or(0.);
    match key {
        "Create.Duration.Constant" => bank.duration = Some(n("Value", 1.)),
        "Convert.Value.ToDuration" => bank.duration = Some(v),
        "Create.Speed.Constant" => {
            bank.speed = Some(speed(n("Value", 1.), n("Range", 0.) as usize))
        }
        "Convert.Value.ToSpeed" => bank.speed = Some(speed(v, n("Range", 0.) as usize)),
        "Create.Rotation.Constant" => bank.rotation = crate::scenic::vector(&p["Rotation"]).ok(),
        "Create.Rotation.XYZ" => bank.rotation = Some([n("X", 0.), n("Y", 0.), n("Z", 0.)]),
        "Create.Rotation.PanTilt" => bank.rotation = Some([0., n("Pan", 0.5), n("Tilt", 0.5)]),
        "Create.Rotation.Perpendicular" => {
            bank.rotation =
                Some([[0., 0., 0.25], [0.; 3], [0.25, 0., 0.]][(n("Axis", 0.) as usize).min(2)])
        }
        "Create.Focus.Value"
        | "Create.Iris.Value"
        | "Create.Zoom.Value"
        | "Create.Temperature.Constant"
        | "Create.BeamShaper.Position" => {
            let channel = match key {
                "Create.Focus.Value" => "Focus",
                "Create.Iris.Value" => "Iris",
                "Create.Zoom.Value" => "Zoom",
                "Create.Temperature.Constant" => "Temperature",
                _ => "BeamShaper",
            };
            bank.channels.insert(channel.into(), json!(n("Value", 0.)));
        }
        "Create.Blade.Position"
        | "Create.Blade.Rotation"
        | "Create.Frost.Value"
        | "Create.Gobo.Constant"
        | "Create.Gobo.Rotate.Value"
        | "Create.Gobo.Shake.Constant"
        | "Create.Prism.Rotate"
        | "Create.Prism.Select"
        | "Create.AnimationWheel.Position"
        | "Create.AnimationWheel.Rotation" => {
            let channel = match key {
                "Create.Blade.Position" => "BladePosition",
                "Create.Blade.Rotation" => "BladeRotation",
                "Create.Frost.Value" => "Frost",
                "Create.Gobo.Constant" => "Gobo",
                "Create.Gobo.Rotate.Value" => "GoboRotation",
                "Create.Gobo.Shake.Constant" => "GoboShake",
                "Create.Prism.Rotate" => "PrismRotation",
                "Create.Prism.Select" => "Prism",
                "Create.AnimationWheel.Position" => "AnimationWheelPosition",
                _ => "AnimationWheelRotation",
            };
            let property = if key == "Create.Gobo.Constant" {
                "Gobo"
            } else if key == "Create.Prism.Select" {
                "Index"
            } else {
                "Value"
            };
            slot(bank, channel, n("Slot", 0.), n(property, 0.));
        }
        "Create.Macro.Constant" => {
            bank.channels.insert("Macro".into(), p["Macro"].clone());
        }
        "Create.Resource.Constant" => {
            bank.channels
                .insert("Resource".into(), p["Resource"].clone());
        }
        "Create.Shapes.ResourceMode.Constant" => {
            bank.channels
                .insert("ShapesResourceMode".into(), p["ShapesResourceMode"].clone());
        }
        "Modify.ColorWheel.Offset" => {
            let previous = bank
                .channels
                .get("ColorWheel")
                .and_then(Value::as_f64)
                .unwrap_or(0.);
            bank.channels
                .insert("ColorWheel".into(), json!(previous + n("Value", -1.)));
        }
        "Combine.Value.Boolean" => {
            let or = flag(p, "And/Or", false);
            let result = if or {
                inputs.iter().any(|b| b.value.unwrap_or(0.) > 0.5)
            } else {
                inputs.iter().all(|b| b.value.unwrap_or(0.) > 0.5)
            };
            bank.value = Some(if result { 1. } else { 0. });
        }
        "Combine.Value.Compare" => {
            let a = inputs.first().and_then(|b| b.value).unwrap_or(0.);
            let b = inputs.get(1).and_then(|b| b.value).unwrap_or(0.);
            let result = match n("Operator", 0.) as i64 {
                0 => a > b,
                1 => a >= b,
                2 => a < b,
                3 => a <= b,
                _ => (a - b).abs() <= n("Epsilon", 0.).abs(),
            };
            bank.value = Some(if result { 1. } else { 0. });
        }
        "Combine.Color.Distance" => {
            let a = inputs
                .first()
                .and_then(|b| b.colors.as_ref())
                .and_then(|c| c.first())
                .copied()
                .unwrap_or([0.; 4]);
            let b = inputs
                .get(1)
                .and_then(|b| b.colors.as_ref())
                .and_then(|c| c.first())
                .copied()
                .unwrap_or([0.; 4]);
            bank.value = Some(
                a[..3]
                    .iter()
                    .zip(&b)
                    .map(|(a, b)| (a - b).powi(2))
                    .sum::<f64>()
                    .sqrt(),
            );
        }
        "Modify.Value.SelectRange" => {
            let a = n("Min", 0.);
            let b = n("Max", 1.);
            let t = if (b - a).abs() < 1e-12 {
                0.
            } else {
                (v - a) / (b - a)
            };
            bank.value = Some(if flag(p, "Clamp", true) {
                t.clamp(0., 1.)
            } else {
                t
            });
        }
        "Modify.Value.Wave" => {
            let phase = v.rem_euclid(1.);
            let result = match n("Shape", 0.) as i64 {
                0 => phase,
                1 => 1. - (2. * phase - 1.).abs(),
                2 => 0.5 + 0.5 * (phase * std::f64::consts::TAU).sin(),
                3 => 0.5 + 0.5 * (phase * std::f64::consts::TAU).cos(),
                _ => {
                    if phase < n("Pulse Width", 0.5) {
                        1.
                    } else {
                        0.
                    }
                }
            };
            bank.value = Some(result);
        }
        "Modify.Color.Alpha.AsAlpha"
        | "Modify.Color.Clamp"
        | "Modify.Color.Hue.Rotate"
        | "Modify.Color.Saturation.Set" => {
            if let Some(colors) = bank.colors.as_mut() {
                for color in colors {
                    let mut h = hsv(*color);
                    match key {
                        "Modify.Color.Alpha.AsAlpha" => *color = [1., 1., 1., h[2]],
                        "Modify.Color.Clamp" => {
                            let a = n("Minimum", 0.);
                            let b = n("Maximum", 1.);
                            for c in color.iter_mut().take(3) {
                                *c = c.clamp(a.min(b), a.max(b));
                            }
                            color[3] = 1.;
                        }
                        "Modify.Color.Hue.Rotate" => {
                            h[0] += n("Value", 0.);
                            *color = rgb(h);
                        }
                        _ => {
                            h[1] = n("Value", 0.);
                            *color = rgb(h);
                        }
                    }
                }
            }
        }
        _ => return false,
    }
    true
}
