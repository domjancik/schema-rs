use crate::model::Fixture;
use std::collections::BTreeMap;
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Emitter {
    pub fixture: String,
    pub node: u64,
    pub layer: usize,
    pub position: [f64; 3],
    pub direction: [f64; 3],
    pub kind: u8,
    pub color: [f64; 4],
    pub intensity: f64,
    pub range: f64,
    pub falloff: f64,
    pub angle: f64,
    pub softness: f64,
    pub width: f64,
    pub height: f64,
    pub two_sided: bool,
    pub channel: String,
    #[serde(default)]
    pub follows_fixture: bool,
}
#[derive(Default)]
pub(crate) struct Frame {
    pub emitters: Vec<Emitter>,
    pub positions: BTreeMap<String, [f64; 3]>,
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a.iter().zip(b).map(|(a, b)| a * b).sum()
}
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| a[i] - b[i])
}
fn scale(a: [f64; 3], s: f64) -> [f64; 3] {
    a.map(|v| v * s)
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn length(a: [f64; 3]) -> f64 {
    dot(a, a).sqrt()
}
fn normalized(a: [f64; 3]) -> [f64; 3] {
    scale(a, 1. / length(a).max(1e-12))
}
impl Emitter {
    pub fn sample(&self, point: [f64; 3]) -> f64 {
        let delta = sub(point, self.position);
        let direction = normalized(self.direction);
        let axial = dot(delta, direction);
        let mut distance = length(delta);
        let mut angular = 1.;
        if self.kind == 1 {
            if !self.two_sided && axial < 0. {
                return 0.;
            }
            let up = if direction[1].abs() < 0.99 {
                [0., 1., 0.]
            } else {
                [1., 0., 0.]
            };
            let u = normalized(cross(up, direction));
            let v = cross(direction, u);
            let x = dot(delta, u);
            let y = dot(delta, v);
            distance = (axial * axial
                + (x.abs() - self.width / 2.).max(0.).powi(2)
                + (y.abs() - self.height / 2.).max(0.).powi(2))
            .sqrt();
        } else if self.kind == 2 && distance > 1e-12 {
            let cosine = axial / distance;
            let outer = (self.angle.to_radians() / 2.).cos();
            if cosine <= outer {
                return 0.;
            }
            let inner = (self.angle.to_radians() / 2. * (1. - self.softness)).cos();
            angular = if inner - outer < 1e-12 {
                1.
            } else {
                let t = ((cosine - outer) / (inner - outer)).clamp(0., 1.);
                t * t * (3. - 2. * t)
            };
        }
        if distance >= self.range {
            return 0.;
        }
        self.intensity * (1. - distance / self.range).max(0.).powf(self.falloff) * angular
    }
}
pub(crate) fn group_bounds(fixtures: &[Fixture], group: &str) -> Option<([f64; 3], [f64; 3])> {
    let mut selected = fixtures
        .iter()
        .filter(|f| group.is_empty() || f.groups.iter().any(|g| g == group));
    let first = selected.next()?.position;
    let mut min = first;
    let mut max = first;
    for f in selected {
        for i in 0..3 {
            min[i] = min[i].min(f.position[i]);
            max[i] = max[i].max(f.position[i]);
        }
    }
    Some((min, max))
}
