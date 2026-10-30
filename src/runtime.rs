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
        Self::decode_session_with_report(bytes).map(|(session, _)| session)
    }
    pub fn decode_session_with_report(bytes: &[u8]) -> Result<(Self, crate::migrations::Report)> {
        let (mut session, report) = crate::session::decode_with_report(bytes)?;
        session.disarm_output();
        Ok((
            Self {
                engine: Engine::from_session(session)?,
                action_diagnostics: Vec::new(),
            },
            report,
        ))
    }
    pub fn storage_snapshot(
        &self,
        kind: crate::migrations::Kind,
        title: Option<String>,
        fixtures: Vec<String>,
        layer: Option<usize>,
    ) -> Result<String> {
        let mut project = self.engine.project().clone();
        if let Some(title) = title {
            project.name = title;
        }
        project.output.enabled = false;
        crate::scenic::with_plugins(self.engine.plugins(), || {
            project.validate()?;
            match kind {
                crate::migrations::Kind::Project => serde_json::to_string(&project),
                crate::migrations::Kind::Layout => {
                    serde_json::to_string(&crate::storage_formats::Layout {
                        version: crate::migrations::LAYOUT_VERSION,
                        name: project.name,
                        fixtures: project.fixtures,
                        dmx_models: project.dmx_models,
                        dimming: project.dimming,
                    })
                }
                crate::migrations::Kind::Content => {
                    let director = "__director".to_owned();
                    let entries = project
                        .content
                        .iter()
                        .chain(std::iter::once((&director, &project.director)))
                        .filter(|(id, _)| fixtures.is_empty() || fixtures.contains(id))
                        .flat_map(|(id, layers)| {
                            layers
                                .iter()
                                .enumerate()
                                .filter(|(i, _)| layer.is_none_or(|selected| selected == *i))
                                .map(|(layer, stack)| crate::model::ContentEntry {
                                    fixture: id.clone(),
                                    layer,
                                    stack: stack.clone(),
                                })
                        })
                        .collect();
                    serde_json::to_string(&crate::storage_formats::Content {
                        version: crate::migrations::CONTENT_VERSION,
                        entries,
                    })
                }
                _ => return Err("Expected project, layout or content snapshot".into()),
            }
            .map_err(|e| e.to_string())
        })
    }
    pub fn content_resource(&mut self, name: String, text: &str) -> Result<()> {
        let document: crate::storage_formats::Content =
            serde_json::from_value(crate::migrations::parse(text.as_bytes())?)
                .map_err(|e| e.to_string())?;
        self.command(
            Command::Input {
                event: crate::inputs::Event::Content {
                    name,
                    entries: document.entries,
                },
            },
            None,
        )?;
        Ok(())
    }
    pub fn load_content_text(
        &mut self,
        text: &str,
        group: Option<String>,
        expected: Option<u64>,
    ) -> Result<u64> {
        let document: crate::storage_formats::Content =
            serde_json::from_value(crate::migrations::parse(text.as_bytes())?)
                .map_err(|e| e.to_string())?;
        let entries = document
            .entries
            .into_iter()
            .filter(|entry| {
                group.as_ref().is_none_or(|group| {
                    group.is_empty()
                        || self.engine.project().fixtures.iter().any(|fixture| {
                            fixture.id == entry.fixture && fixture.groups.contains(group)
                        })
                })
            })
            .collect();
        self.command(Command::ApplyContent { entries }, expected)
    }
    pub fn compose_project(
        &mut self,
        layout: Option<String>,
        content: Option<String>,
        expected: Option<u64>,
    ) -> Result<u64> {
        let mut project = self.engine.project().clone();
        if let Some(text) = layout {
            let layout: crate::storage_formats::Layout =
                serde_json::from_value(crate::migrations::parse(text.as_bytes())?)
                    .map_err(|e| e.to_string())?;
            project.name = layout.name;
            project.fixtures = layout.fixtures;
            project.dmx_models = layout.dmx_models;
            project.dimming = layout.dimming;
            project
                .content
                .retain(|id, _| project.fixtures.iter().any(|f| &f.id == id));
            for f in &project.fixtures {
                project
                    .content
                    .entry(f.id.clone())
                    .or_insert_with(|| vec![None, None, None]);
            }
        }
        if let Some(text) = content {
            let document: crate::storage_formats::Content =
                serde_json::from_value(crate::migrations::parse(text.as_bytes())?)
                    .map_err(|e| e.to_string())?;
            for entry in document.entries {
                if entry.layer >= crate::model::MAX_LAYERS {
                    return Err("Invalid saved layer".into());
                }
                let layers = if entry.fixture == "__director" {
                    &mut project.director
                } else {
                    project
                        .content
                        .get_mut(&entry.fixture)
                        .ok_or("Saved content references an unknown fixture")?
                };
                layers.resize(layers.len().max(entry.layer + 1), None);
                layers[entry.layer] = entry.stack;
            }
        }
        project.output.enabled = false;
        self.command(
            Command::ReplaceProject {
                project: Box::new(project),
            },
            expected,
        )
    }
    pub fn migrate_document(
        &self,
        kind: crate::migrations::Kind,
        document: serde_json::Value,
    ) -> Result<crate::migrations::Migrated> {
        crate::scenic::with_plugins(self.engine.plugins(), || {
            crate::migrations::validated(kind, document)
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
