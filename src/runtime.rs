//! Synchronous, I/O-free session host. Transports, scheduling, storage and sinks live outside it.
use crate::{
    engine::{Command, Engine},
    model::{Project, Result, Snapshot},
};
use std::sync::Arc;
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Published {
    #[serde(skip)]
    pub(crate) host: Arc<crate::inputs::Inputs>,
    #[serde(skip)]
    pub plugins: Arc<crate::plugins::Registry>,
    #[serde(skip)]
    pub resources: Arc<std::collections::BTreeMap<String, Vec<u8>>>,
    pub project: Project,
    pub snapshot: Snapshot,
}
impl Published {
    pub fn capture(engine: &Engine) -> Self {
        Self {
            host: engine.host(),
            plugins: engine.plugins(),
            resources: engine.resources(),
            project: engine.project().clone(),
            snapshot: engine.current_snapshot(),
        }
    }
}
pub struct EngineSession {
    engine: Engine,
    action_diagnostics: Vec<String>,
}
impl EngineSession {
    pub fn new(project: Project) -> Result<Self> {
        Ok(Self {
            engine: Engine::new(project)?,
            action_diagnostics: Vec::new(),
        })
    }
    pub fn publication(&self) -> Published {
        let mut frame = Published::capture(&self.engine);
        frame
            .snapshot
            .diagnostics
            .extend(self.action_diagnostics.iter().cloned());
        frame
    }
    pub fn command(&mut self, command: Command, expected: Option<u64>) -> Result<u64> {
        if expected.is_some_and(|revision| revision != self.engine.revision()) {
            return Err("Revision conflict: refresh state before editing".into());
        }
        self.engine.apply(command)?;
        Ok(self.engine.revision())
    }
    pub fn advance(&mut self, dt: f64) -> Result<Published> {
        if !dt.is_finite() || dt < 0. {
            return Err("Frame delta must be finite and nonnegative".into());
        }
        self.engine.tick(dt);
        let resolved = crate::native::actions::resolve(&self.publication());
        let mut errors = resolved.errors;
        for event in resolved.events {
            if let Err(error) = self.engine.apply(Command::Input { event }) {
                errors.push(error);
            }
        }
        for command in resolved.commands {
            if let Err(error) = self.engine.apply(command) {
                errors.push(error);
            }
        }
        for command in resolved.internal {
            let result = Engine::prepare_internal(self.engine.project().clone(), command)
                .and_then(|prepared| self.engine.install(prepared));
            if let Err(error) = result {
                errors.push(error);
            }
        }
        self.action_diagnostics = errors;
        Ok(self.publication())
    }
    pub fn encode_session(&self) -> Result<Vec<u8>> {
        serde_json::to_vec(&self.engine.checkpoint()).map_err(|e| e.to_string())
    }
    pub fn decode_session(bytes: &[u8]) -> Result<Self> {
        let mut session = crate::session::decode(bytes)?;
        session.disarm_output();
        Ok(Self {
            engine: Engine::from_session(session)?,
            action_diagnostics: Vec::new(),
        })
    }
    pub fn history(&self) -> serde_json::Value {
        crate::history::view(&self.engine.checkpoint())
    }
    pub fn bundle(&self) -> crate::bundle::Bundle {
        crate::bundle::Bundle::from_session(&self.engine.checkpoint())
    }
    pub fn import_bundle(
        &mut self,
        bundle: crate::bundle::Bundle,
        expected: Option<u64>,
    ) -> Result<u64> {
        if expected.is_some_and(|r| r != self.engine.revision()) {
            return Err("Revision conflict: refresh state before editing".into());
        }
        let prepared = Engine::prepare_bundle(self.engine.checkpoint(), bundle)?;
        self.engine.install_bundle(prepared)
    }
    pub fn catalog(&self) -> Vec<serde_json::Value> {
        let mut rows = crate::scenic::catalog()
            .iter()
            .map(|row| serde_json::to_value(row).expect("catalog row"))
            .collect::<Vec<_>>();
        rows.extend(self.engine.plugins().catalog());
        rows
    }
    pub fn import_stack(
        &mut self,
        fixture: String,
        layer: usize,
        yaml: &str,
        expected: Option<u64>,
    ) -> Result<u64> {
        let stack =
            crate::scenic::with_plugins(self.engine.plugins(), || crate::scenic::parse(yaml))?;
        self.command(
            Command::ReplaceLayer {
                fixture,
                layer,
                stack: Some(stack),
            },
            expected,
        )
    }
}
