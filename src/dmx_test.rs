//! Short-lived manual channel overrides, never authored or restored from a project/session.
use crate::clock::Instant;
use crate::model::Result;
use std::{collections::BTreeMap, time::Duration};
#[derive(Clone, Debug)]
pub struct Lease {
    owner: String,
    pub universe: u16,
    pub channels: BTreeMap<u16, u8>,
    expires: Instant,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Status {
    pub universe: u16,
    pub channels: BTreeMap<u16, u8>,
}
pub fn validate(owner: &str, universe: u16, channels: &BTreeMap<u16, u8>) -> Result<()> {
    if owner.is_empty()
        || owner.len() > 64
        || !owner
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
        || universe > 32767
        || channels.len() > 32
        || channels.keys().any(|c| !(1..=512).contains(c))
    {
        return Err(
            "Invalid channel test: select universe 0..32767 and at most 32 channels 1..512".into(),
        );
    }
    Ok(())
}
pub fn update_at(
    lease: &mut Option<Lease>,
    owner: String,
    universe: u16,
    channels: BTreeMap<u16, u8>,
    now: Instant,
) -> Result<()> {
    validate(&owner, universe, &channels)?;
    if lease
        .as_ref()
        .is_some_and(|l| l.expires > now && l.owner != owner)
    {
        if channels.is_empty() {
            return Ok(());
        }
        return Err(
            "Another test panel owns the channel test. Release it or wait 5 seconds".into(),
        );
    }
    *lease = if channels.is_empty() {
        None
    } else {
        Some(Lease {
            owner,
            universe,
            channels,
            expires: now + Duration::from_secs(5),
        })
    };
    Ok(())
}
pub fn apply_at(
    lease: &Option<Lease>,
    universes: &mut BTreeMap<u16, Vec<u8>>,
    blackout: bool,
    now: Instant,
) -> Option<Status> {
    let l = lease.as_ref().filter(|l| l.expires > now)?;
    let bytes = universes.entry(l.universe).or_insert_with(|| vec![0; 512]);
    bytes.resize(512, 0);
    for (channel, value) in &l.channels {
        bytes[*channel as usize - 1] = if blackout { 0 } else { *value };
    }
    Some(Status {
        universe: l.universe,
        channels: l.channels.clone(),
    })
}

pub fn deserialize_channels<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<BTreeMap<u16, u8>, D::Error> {
    use serde::Deserialize;
    let raw = BTreeMap::<String, u8>::deserialize(deserializer)?;
    raw.into_iter()
        .map(|(key, value)| {
            key.parse::<u16>()
                .map(|key| (key, value))
                .map_err(serde::de::Error::custom)
        })
        .collect()
}
