#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MotionState {
    last_time: Option<f64>,
    rng: u64,
    initialized: bool,
    elapsed: f64,
    next: f64,
    transition: f64,
    from: [f64; 3],
    target: [f64; 3],
    current: [f64; 3],
    pub trigger: bool,
    pub admitted: bool,
    pub phase: f64,
}
pub(crate) fn seed(id: &str, explicit: f64) -> u64 {
    if explicit != 0. {
        return explicit as i64 as u64;
    }
    id.bytes().fold(0xcbf29ce484222325, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x100000001b3)
    })
}
impl MotionState {
    pub(crate) fn valid(&self, time: f64) -> bool {
        self.last_time
            .is_none_or(|v| v.is_finite() && v >= 0. && v <= time + 1e-6)
            && [self.elapsed, self.next, self.transition, self.phase]
                .iter()
                .all(|v| v.is_finite() && *v >= 0.)
            && self
                .from
                .iter()
                .chain(&self.target)
                .chain(&self.current)
                .all(|v| v.is_finite())
    }

    pub fn delta(&mut self, time: f64) -> f64 {
        let dt = (time - self.last_time.unwrap_or(time)).max(0.);
        self.last_time = Some(time);
        dt
    }
    pub fn init(&mut self, seed: u64) {
        if !self.initialized {
            self.rng = seed;
            self.initialized = true;
        }
    }
    pub fn draw(&mut self) -> f64 {
        self.rng = self.rng.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.rng;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        ((z ^ (z >> 31)) >> 11) as f64 / (1_u64 << 53) as f64
    }
    pub fn random(&mut self, params: RandomParams) -> [f64; 3] {
        let RandomParams {
            time,
            seed,
            interval,
            filter,
            jitter,
            paused,
        } = params;
        let dt = self.delta(time);
        let fresh = !self.initialized;
        self.init(seed);
        if fresh {
            self.target = std::array::from_fn(|_| self.draw());
            self.current = self.target;
            self.from = self.target;
            self.next = interval.max(0.001);
        }
        if paused {
            return self.current;
        }
        self.elapsed += dt;
        self.transition += dt;
        // Bounded by the engine's maximum 0.25s step and 0.001s minimum interval.
        for _ in 0..256 {
            if self.elapsed < self.next {
                break;
            }
            self.elapsed -= self.next;
            self.from = self.current;
            self.target = std::array::from_fn(|_| self.draw());
            self.transition = self.elapsed;
            self.next =
                (interval * (1. + (2. * self.draw() - 1.) * jitter.clamp(0., 1.))).max(0.001);
        }
        let t = if filter <= 0. {
            1.
        } else {
            (self.transition / filter).clamp(0., 1.)
        };
        self.current = std::array::from_fn(|i| self.from[i] + (self.target[i] - self.from[i]) * t);
        self.current
    }
}
pub(crate) struct RandomParams {
    pub time: f64,
    pub seed: u64,
    pub interval: f64,
    pub filter: f64,
    pub jitter: f64,
    pub paused: bool,
}
pub(crate) fn cubic(points: [f64; 4], t: f64) -> f64 {
    let u = 1. - t;
    u * u * u * points[0]
        + 3. * u * u * t * points[1]
        + 3. * u * t * t * points[2]
        + t * t * t * points[3]
}
pub(crate) fn progress(
    explicit: f64,
    input: Option<f64>,
    time: f64,
    duration: f64,
    repeat: bool,
    ping_pong: bool,
) -> f64 {
    if explicit >= 0. {
        return explicit.clamp(0., 1.);
    }
    if let Some(t) = input {
        return t.clamp(0., 1.);
    }
    let t = time / duration.max(0.001);
    if repeat {
        if ping_pong {
            1. - (t.rem_euclid(2.) - 1.).abs()
        } else {
            t.rem_euclid(1.)
        }
    } else {
        t.clamp(0., 1.)
    }
}
