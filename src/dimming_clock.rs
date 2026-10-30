//! Temporary, owner-scoped daily-dimming clock. Never part of authored or recovery state.
use crate::{clock::Instant, model::Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;
const DAY: f64 = 86400.;
const LEASE: Duration = Duration::from_secs(15);
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum Control {
    Simulate {
        owner: String,
        seconds: f64,
        speed: f64,
    },
    Renew {
        owner: String,
    },
    Live {
        #[serde(default)]
        owner: Option<String>,
    },
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Status {
    pub live_seconds: u32,
    pub seconds: u32,
    pub simulating: bool,
    pub speed: f64,
    pub owner: Option<String>,
    pub expires_in: f64,
    pub level: f64,
    pub utc_offset_minutes: i32,
}
struct Simulation {
    owner: String,
    seconds: f64,
    speed: f64,
    started: Instant,
    expires: Instant,
}
#[derive(Default)]
pub struct Clock {
    simulation: Option<Simulation>,
}
impl Clock {
    fn active(&self, now: Instant) -> Option<&Simulation> {
        self.simulation.as_ref().filter(|s| now < s.expires)
    }
    pub fn control_at(&mut self, control: Control, now: Instant) -> Result<()> {
        match control {
            Control::Simulate {
                owner,
                seconds,
                speed,
            } => {
                if owner.is_empty()
                    || owner.len() > 80
                    || !owner
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
                    || !seconds.is_finite()
                    || !(0.0..DAY).contains(&seconds)
                    || !speed.is_finite()
                    || !(0.0..=3600.).contains(&speed)
                {
                    return Err("Invalid dimming time simulation".into());
                }
                self.simulation = Some(Simulation {
                    owner,
                    seconds,
                    speed,
                    started: now,
                    expires: now + LEASE,
                });
            }
            Control::Renew { owner } => {
                if self.active(now).is_none_or(|s| s.owner != owner) {
                    return Err("Time simulation expired or is controlled from another view".into());
                }
                self.simulation.as_mut().unwrap().expires = now + LEASE;
            }
            Control::Live { owner } => {
                if owner
                    .as_ref()
                    .is_none_or(|owner| self.simulation.as_ref().is_some_and(|s| s.owner == *owner))
                {
                    self.simulation = None;
                }
            }
        }
        Ok(())
    }
    pub fn seconds_at(&self, live: u32, now: Instant) -> u32 {
        self.active(now).map_or(live, |s| {
            (s.seconds
                + now
                    .checked_duration_since(s.started)
                    .unwrap_or_default()
                    .as_secs_f64()
                    * s.speed)
                .rem_euclid(DAY) as u32
        })
    }
    pub fn status_at(
        &self,
        live: u32,
        timeline: &crate::output_design::DimmingTimeline,
        now: Instant,
    ) -> Status {
        let active = self.active(now);
        let seconds = self.seconds_at(live, now);
        Status {
            live_seconds: live,
            seconds,
            simulating: active.is_some(),
            speed: active.map_or(0., |s| s.speed),
            owner: active.map(|s| s.owner.clone()),
            expires_in: active.map_or(0., |s| s.expires.duration_since(now).as_secs_f64()),
            level: timeline.level(seconds),
            utc_offset_minutes: timeline.utc_offset_minutes,
        }
    }
}
