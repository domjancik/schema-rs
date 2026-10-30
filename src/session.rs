use crate::{
    engine::NodeState,
    model::{Project, Result, Snapshot},
};
use serde::Serialize;
use std::{collections::HashMap, sync::Arc};
#[cfg(feature = "native")]
const MAX_BYTES: u64 = crate::migrations::MAX_BYTES as u64;
crate::versioned_document! {crate::migrations::Kind::Session, SessionWire, "Session";
#[derive(Clone, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Session {
    #[serde(default)]
    pub plugins: Arc<crate::plugins::Registry>,
    #[serde(default)]
    pub inputs: std::sync::Arc<crate::inputs::Inputs>,
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub resources: Arc<std::collections::BTreeMap<String, Vec<u8>>>,
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub checkpoints: std::collections::BTreeMap<String, Arc<Project>>,
    pub version: u32,
    pub project: Arc<Project>,
    pub snapshot: Snapshot,
    pub undo: Vec<Arc<Project>>,
    pub redo: Vec<Arc<Project>>,
    pub(crate) states: HashMap<String, NodeState>,
}
}
impl Session {
    pub fn disarm_output(&mut self) {
        Arc::make_mut(&mut self.project).output.enabled = false;
        for project in self
            .undo
            .iter_mut()
            .chain(&mut self.redo)
            .chain(self.checkpoints.values_mut())
        {
            Arc::make_mut(project).output.enabled = false;
        }
    }

    pub fn validate(&self) -> Result<()> {
        crate::scenic::with_plugins(self.plugins.clone(), || self.validate_inner())
    }
    fn validate_inner(&self) -> Result<()> {
        if self.version != crate::migrations::SESSION_VERSION {
            return Err("Unsupported session version".into());
        }
        if self.undo.len() > 128 || self.redo.len() > 128 || self.undo.len() + self.redo.len() > 128
        {
            return Err("Session history exceeds 128 transactions".into());
        }
        if !self.snapshot.time.is_finite()
            || self.snapshot.time < 0.
            || self.snapshot.revision == u64::MAX
            || self.snapshot.frame == u64::MAX
        {
            return Err("Invalid session clock/revision".into());
        }
        if self.checkpoints.len() > 32 {
            return Err("At most 32 named checkpoints".into());
        }
        for (name, project) in &self.checkpoints {
            validate_checkpoint_name(name)?;
            project.validate()?;
        }
        crate::bundle::validate_resources(&self.resources)?;
        self.inputs.validate()?;
        self.project.validate()?;
        for p in self.undo.iter().chain(&self.redo) {
            p.validate()?;
        }
        if self.states.len() > (self.project.fixtures.len() + 1) * crate::model::MAX_LAYERS * 4096 {
            return Err("Session runtime state exceeds node limit".into());
        }
        let mut keys = std::collections::HashMap::new();
        fn walk<'a>(
            n: &'a crate::model::Node,
            prefix: &str,
            keys: &mut std::collections::HashMap<String, &'a crate::model::Node>,
            plugins: &'a crate::plugins::Registry,
            prefixes: &std::collections::HashSet<String>,
        ) {
            keys.insert(format!("{prefix}/{}", n.id), n);
            for child in n.children.iter().chain(n.drivers.values()) {
                walk(child, prefix, keys, plugins, prefixes);
            }
            if let Some(component) = plugins.component(&n.behavior) {
                let virtual_prefix = format!("{prefix}/{}@plugin/{}", n.id, component.key);
                if prefixes.contains(&virtual_prefix) {
                    walk(&component.root, &virtual_prefix, keys, plugins, prefixes);
                }
            }
        }
        let mut prefixes = std::collections::HashSet::new();
        for key in self.states.keys() {
            for (index, _) in key.match_indices("@plugin/") {
                let start = index + 8;
                if let Some(end) = key[start..].find('/') {
                    prefixes.insert(key[..start + end].to_owned());
                }
            }
        }
        let director_key = "__director".to_owned();
        for (fixture, layers) in self
            .project
            .content
            .iter()
            .chain(std::iter::once((&director_key, &self.project.director)))
        {
            for (layer, stack) in layers.iter().enumerate() {
                if let Some(stack) = stack {
                    walk(
                        &stack.root,
                        &format!("{fixture}/{layer}"),
                        &mut keys,
                        &self.plugins,
                        &prefixes,
                    );
                }
            }
        }
        if self.states.keys().any(|key| !keys.contains_key(key)) {
            return Err("Session runtime state references a missing node".into());
        }
        for (key, state) in &self.states {
            state.validate_for(keys[key], self.snapshot.time)?;
        }
        Ok(())
    }
}
pub fn decode(bytes: &[u8]) -> Result<Session> {
    decode_with_report(bytes).map(|(session, _)| session)
}
pub fn decode_with_report(bytes: &[u8]) -> Result<(Session, crate::migrations::Report)> {
    let migrated = crate::migrations::migrate(
        crate::migrations::Kind::Session,
        crate::migrations::parse(bytes)?,
    )?;
    let session: Session =
        serde_json::from_value(migrated.document).map_err(|e| format!("Session format: {e}"))?;
    session.validate()?;
    Ok((session, migrated.report))
}
#[cfg(feature = "native")]
mod storage;
#[cfg(feature = "native")]
pub use storage::{load, Status, Store};

pub fn validate_checkpoint_name(name: &str) -> Result<()> {
    if name.trim().is_empty() || name.len() > 80 || name.chars().any(char::is_control) {
        Err("Checkpoint name must contain 1–80 printable characters".into())
    } else {
        Ok(())
    }
}
