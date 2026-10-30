use crate::{model::*, scenic};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Command {
    ImportProject {
        json: String,
    },
    SetNodeBypassed {
        fixture: String,
        layer: usize,
        node: u64,
        bypassed: bool,
    },
    SetDebugFocus {
        target: Option<DebugFocus>,
    },
    HistoryJump {
        offset: i32,
    },
    SaveCheckpoint {
        name: String,
    },
    RemoveCheckpoint {
        name: String,
    },
    RestoreCheckpoint {
        name: String,
    },
    ReplaceProject {
        project: Box<Project>,
    },
    SetDimming {
        timeline: crate::output_design::DimmingTimeline,
    },
    UpsertDmxModel {
        id: String,
        model: crate::output_design::ModelDefinition,
    },
    RemoveDmxModel {
        id: String,
    },
    AdoptNode {
        fixture: String,
        layer: usize,
        node: u64,
        child: u64,
    },
    ReplaceChildren {
        fixture: String,
        layer: usize,
        node: u64,
        children: Vec<Node>,
    },
    #[serde(skip)]
    PreparedInput {
        event: crate::inputs::PreparedEvent,
    },
    Input {
        event: crate::inputs::Event,
    },
    MoveNode {
        fixture: String,
        layer: usize,
        node: u64,
        target: u64,
        branch: bool,
        below: bool,
    },
    MoveFixture {
        fixture: String,
        position: [f64; 3],
    },
    SetDimmer {
        fixture: String,
        value: f64,
    },
    SetFixture {
        fixture: Fixture,
    },
    AddFixture {
        fixture: Fixture,
    },
    RemoveFixture {
        fixture: String,
    },
    AddLayer {
        fixture: String,
    },
    SetLayerEnabled {
        fixture: String,
        layer: usize,
        enabled: bool,
    },
    ReplaceLayer {
        fixture: String,
        layer: usize,
        stack: Option<Stack>,
    },
    ApplyContent {
        entries: Vec<ContentEntry>,
    },
    SetProperty {
        fixture: String,
        layer: usize,
        node: u64,
        key: String,
        value: Value,
    },
    SetDriver {
        fixture: String,
        layer: usize,
        node: u64,
        key: String,
        driver: Option<Node>,
    },
    AddInput {
        #[serde(default)]
        index: Option<usize>,
        fixture: String,
        layer: usize,
        node: u64,
        behavior: String,
    },
    WrapNode {
        fixture: String,
        layer: usize,
        node: u64,
        behavior: String,
    },
    DeleteNode {
        fixture: String,
        layer: usize,
        node: u64,
        branch: bool,
    },
    SetOutput {
        config: OutputConfig,
    },
    Batch {
        commands: Vec<Command>,
    },
    Undo,
    Redo,
    Pause {
        paused: bool,
    },
    Blackout {
        enabled: bool,
    },
}
#[derive(Default, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NodeState {
    driven: BTreeMap<String, Value>,
    held: Bank,
    accumulator: f64,
    motion: crate::motion::MotionState,
    #[serde(default)]
    native: crate::native::timed::State,
}
impl NodeState {
    pub(crate) fn validate_for(&self, node: &Node, time: f64) -> Result<()> {
        self.native.validate(time)?;
        self.held.validate_payload()?;
        let defs = scenic::defaults(&node.behavior)?;
        if !self.accumulator.is_finite() || !self.motion.valid(time) {
            return Err("Invalid session runtime values".into());
        }
        for (key, value) in &self.driven {
            if !node.drivers.contains_key(key)
                || !defs.get(key).is_some_and(|default| {
                    if default.is_boolean() {
                        value.is_boolean()
                    } else {
                        value.as_f64().is_some_and(|v| v.is_finite())
                    }
                })
            {
                return Err("Session contains incompatible driven properties".into());
            }
        }
        let held = &self.held;
        if held.emitters.len() > 1024
            || held.value.is_some_and(|v| !v.is_finite())
            || held
                .position
                .is_some_and(|p| p.iter().any(|v| !v.is_finite()))
            || held
                .colors
                .as_ref()
                .is_some_and(|colors| colors.iter().flatten().any(|v| !v.is_finite()))
        {
            return Err("Invalid held session bank".into());
        }
        if held.emitters.iter().any(|e| {
            !e.intensity.is_finite()
                || e.range <= 0.
                || !e.range.is_finite()
                || e.color
                    .iter()
                    .chain(e.position.iter())
                    .chain(e.direction.iter())
                    .any(|v| !v.is_finite())
        }) {
            return Err("Invalid held session emitter".into());
        }
        Ok(())
    }
}
pub struct PreparedEdit {
    reset_runtime: bool,
    plugin_generation: u64,
    record_history: bool,
    project: Project,
    scripts: HashMap<String, rhai::AST>,
}
pub struct PreparedPluginUpdate {
    changed_components: std::collections::BTreeSet<String>,
    base_revision: u64,
    base_generation: u64,
    plugins: Arc<crate::plugins::Registry>,
    scripts: HashMap<String, rhai::AST>,
    undo_scripts: Vec<HashMap<String, rhai::AST>>,
    redo_scripts: Vec<HashMap<String, rhai::AST>>,
}
pub struct PreparedBundle {
    engine: Engine,
    revision: u64,
    generation: u64,
}
pub struct Engine {
    plugins: Arc<crate::plugins::Registry>,
    inputs: std::sync::Arc<crate::inputs::Inputs>,
    project: Project,
    undo: Vec<(Arc<Project>, HashMap<String, rhai::AST>)>,
    redo: Vec<(Arc<Project>, HashMap<String, rhai::AST>)>,
    revision: u64,
    frame: u64,
    time: f64,
    paused: bool,
    blackout: bool,
    last: Snapshot,
    debug_focus: Option<DebugFocus>,
    dimming_clock: crate::dimming_clock::Clock,
    inspection_pending: Vec<DebugFocus>,
    inspection_frame: crate::inspection::Frame,
    checkpoints: BTreeMap<String, Arc<Project>>,
    resources: Arc<BTreeMap<String, Vec<u8>>>,
    nodes: HashMap<String, NodeState>,
    scripts: HashMap<String, rhai::AST>,
    script_engine: rhai::Engine,
}
impl Engine {
    pub fn new(project: Project) -> Result<Self> {
        project.validate()?;
        let script_engine = script_engine();
        let scripts = compile_scripts(&project, &script_engine)?;
        Ok(Self {
            plugins: scenic::active_plugins(),
            inputs: std::sync::Arc::new(crate::inputs::Inputs::default()),
            project,
            undo: vec![],
            redo: vec![],
            revision: 0,
            frame: 0,
            time: 0.,
            paused: false,
            blackout: false,
            last: Snapshot::default(),
            debug_focus: None,
            dimming_clock: crate::dimming_clock::Clock::default(),
            inspection_pending: Vec::new(),
            inspection_frame: crate::inspection::Frame::default(),
            checkpoints: BTreeMap::new(),
            resources: Arc::new(BTreeMap::new()),
            nodes: HashMap::new(),
            scripts,
            script_engine,
        })
    }
    pub fn new_with_plugins(
        project: Project,
        plugins: Arc<crate::plugins::Registry>,
    ) -> Result<Self> {
        scenic::with_plugins(plugins, || Self::new(project))
    }
    pub fn plugins(&self) -> Arc<crate::plugins::Registry> {
        self.plugins.clone()
    }
    pub fn prepare_plugin_update(
        session: crate::session::Session,
        plugins: Arc<crate::plugins::Registry>,
    ) -> Result<PreparedPluginUpdate> {
        if plugins.state.generation <= session.plugins.state.generation {
            return Err("Plugin generation must advance".into());
        }
        let mut changed_components = std::collections::BTreeSet::new();
        for (id, old) in &session.plugins.state.packages {
            for component in &old.manifest.components {
                let same = plugins.state.packages.get(id).is_some_and(|new| {
                    old.enabled == new.enabled
                        && new.manifest.components.iter().any(|c| c == component)
                });
                if !same {
                    changed_components.insert(format!("Plugin.{id}.{}", component.name));
                }
            }
        }
        let registry = plugins.clone();
        scenic::with_plugins(registry, || {
            session.inputs.validate()?;
            session.project.validate()?;
            let script_engine = script_engine();
            let scripts = compile_scripts(&session.project, &script_engine)?;
            for project in session.checkpoints.values() {
                project.validate()?;
                compile_scripts(project, &script_engine)?;
            }
            let mut undo_scripts = Vec::new();
            let mut redo_scripts = Vec::new();
            for p in &session.undo {
                p.validate()?;
                undo_scripts.push(compile_scripts(p, &script_engine)?);
            }
            for p in &session.redo {
                p.validate()?;
                redo_scripts.push(compile_scripts(p, &script_engine)?);
            }
            Ok(PreparedPluginUpdate {
                changed_components,
                base_revision: session.snapshot.revision,
                base_generation: session.plugins.state.generation,
                plugins,
                scripts,
                undo_scripts,
                redo_scripts,
            })
        })
    }
    pub fn install_plugin_update(&mut self, update: PreparedPluginUpdate) -> Result<u64> {
        if update.base_revision != self.revision
            || update.base_generation != self.plugins.state.generation
        {
            return Err("Revision conflict during plugin update".into());
        }
        self.plugins = update.plugins;
        self.scripts = update.scripts;
        for ((_, scripts), updated) in self.undo.iter_mut().zip(update.undo_scripts) {
            *scripts = updated;
        }
        for ((_, scripts), updated) in self.redo.iter_mut().zip(update.redo_scripts) {
            *scripts = updated;
        }
        self.nodes.retain(|key, _| {
            !key.split("@plugin/").skip(1).any(|part| {
                update
                    .changed_components
                    .contains(part.split('/').next().unwrap_or(""))
            })
        });
        Ok(self.plugins.state.generation)
    }
    /// Resume a trusted local publication during a development binary reload.
    /// Native state is reconstructed from node telemetry; session history is not exported.
    pub fn resume_snapshot(&mut self, snapshot: Snapshot) -> Result<()> {
        if !snapshot.time.is_finite() || snapshot.time < 0. {
            return Err("Invalid resume clock".into());
        }
        fn restore(
            n: &Node,
            fixture: &str,
            layer: usize,
            trace: &BTreeMap<String, Bank>,
            states: &mut HashMap<String, NodeState>,
        ) {
            let bank = trace
                .get(&format!("{layer}/{}", n.id))
                .cloned()
                .unwrap_or_default();
            let mut state = NodeState {
                held: bank.clone(),
                accumulator: bank.value.unwrap_or(0.),
                ..NodeState::default()
            };
            let defaults = scenic::defaults(&n.behavior).unwrap_or_default();
            for (key, driver) in &n.drivers {
                let value = trace
                    .get(&format!("{layer}/{}", driver.id))
                    .and_then(|b| b.value)
                    .unwrap_or(0.);
                let value = driver_value(n, key, value);
                let default = defaults.get(key).cloned().unwrap_or(json!(0.));
                state.driven.insert(
                    key.clone(),
                    if default.is_boolean() {
                        json!(value > 0.5)
                    } else if default.as_i64().is_some() {
                        json!(value.round())
                    } else {
                        json!(value)
                    },
                );
            }
            states.insert(format!("{fixture}/{layer}/{}", n.id), state);
            for child in n.children.iter().chain(n.drivers.values()) {
                restore(child, fixture, layer, trace, states);
            }
        }
        self.nodes.clear();
        for fixture in &snapshot.fixtures {
            if let Some(layers) = self.project.content.get(&fixture.fixture.id) {
                for (layer, stack) in layers.iter().enumerate() {
                    if let Some(stack) = stack {
                        restore(
                            &stack.root,
                            &fixture.fixture.id,
                            layer,
                            &fixture.nodes,
                            &mut self.nodes,
                        );
                    }
                }
            }
        }
        self.time = snapshot.time;
        self.frame = snapshot.frame;
        self.revision = snapshot.revision;
        self.paused = snapshot.paused;
        self.blackout = snapshot.blackout;
        self.last = snapshot;
        Ok(())
    }
    #[cfg(feature = "native")]
    pub(crate) fn paused_for_session(&self) -> bool {
        self.paused
    }
    #[cfg(feature = "native")]
    pub(crate) fn blackout_for_session(&self) -> bool {
        self.blackout
    }
    pub fn current_snapshot(&self) -> Snapshot {
        let mut snapshot = self.last.clone();
        snapshot.revision = self.revision;
        snapshot.plugin_generation = self.plugins.state.generation;
        snapshot.time = self.time;
        snapshot.frame = self.frame;
        snapshot.paused = self.paused;
        snapshot.blackout = self.blackout;
        snapshot.debug_focus = self.debug_focus.clone();
        snapshot.can_undo = !self.undo.is_empty();
        snapshot.can_redo = !self.redo.is_empty();
        snapshot
    }
    pub fn checkpoint(&self) -> crate::session::Session {
        let snapshot = self.current_snapshot();
        crate::session::Session {
            inputs: self.inputs.clone(),
            plugins: self.plugins.clone(),
            checkpoints: self.checkpoints.clone(),
            resources: self.resources.clone(),
            version: crate::migrations::SESSION_VERSION,
            project: Arc::new(self.project.clone()),
            snapshot,
            undo: self.undo.iter().map(|(p, _)| p.clone()).collect(),
            redo: self.redo.iter().map(|(p, _)| p.clone()).collect(),
            states: self.nodes.clone(),
        }
    }
    pub fn from_session(session: crate::session::Session) -> Result<Self> {
        scenic::with_plugins(session.plugins.clone(), || {
            Self::from_session_inner(session)
        })
    }
    fn from_session_inner(session: crate::session::Session) -> Result<Self> {
        session.validate()?;
        let mut engine = Self::new((*session.project).clone())?;
        for project in session.undo {
            let scripts = compile_scripts(&project, &engine.script_engine)?;
            engine.undo.push((project, scripts));
        }
        for project in session.redo {
            let scripts = compile_scripts(&project, &engine.script_engine)?;
            engine.redo.push((project, scripts));
        }
        engine.time = session.snapshot.time;
        engine.frame = session.snapshot.frame;
        engine.revision = session.snapshot.revision;
        engine.paused = session.snapshot.paused;
        engine.blackout = session.snapshot.blackout;
        engine.resources = session.resources;
        engine.checkpoints = session.checkpoints;
        engine.last = session.snapshot;
        engine.last.debug_focus = None;
        engine.nodes = session.states;
        engine.inputs = session.inputs;
        engine.last.can_undo = !engine.undo.is_empty();
        engine.last.can_redo = !engine.redo.is_empty();
        Ok(engine)
    }
    pub fn prepare_bundle(
        session: crate::session::Session,
        bundle: crate::bundle::Bundle,
    ) -> Result<PreparedBundle> {
        let revision = session.snapshot.revision;
        let generation = session.plugins.state.generation;
        Ok(PreparedBundle {
            engine: Self::from_session(bundle.prepare(session)?)?,
            revision,
            generation,
        })
    }
    pub fn install_bundle(&mut self, mut prepared: PreparedBundle) -> Result<u64> {
        if self.revision != prepared.revision
            || self.plugins.state.generation != prepared.generation
        {
            return Err("Revision conflict during bundle import".into());
        }
        let mut inputs = (*self.inputs).clone();
        crate::bundle::merge_assets(
            &mut inputs,
            &crate::bundle::Assets::from_inputs(&prepared.engine.inputs),
        )?;
        for name in prepared.engine.resources.keys() {
            inputs.files.insert(name.clone(), true);
        }
        inputs.validate()?;
        prepared.engine.inputs = Arc::new(inputs);
        prepared.engine.time = self.time;
        prepared.engine.frame = self.frame;
        prepared.engine.paused = self.paused;
        prepared.engine.blackout = self.blackout;
        *self = prepared.engine;
        Ok(self.revision)
    }
    pub fn resources(&self) -> Arc<BTreeMap<String, Vec<u8>>> {
        self.resources.clone()
    }
    pub fn project(&self) -> &Project {
        &self.project
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn snapshot(&self) -> Snapshot {
        self.last.clone()
    }
    pub fn apply(&mut self, command: Command) -> Result<()> {
        scenic::with_plugins(self.plugins.clone(), || self.apply_inner(command))
    }
    fn apply_inner(&mut self, command: Command) -> Result<()> {
        match command {
            Command::SetDebugFocus { target } => {
                if let Some(t) = &target {
                    if t.fixture == "__director" {
                        return Err("Debug output requires a fixture".into());
                    }
                    debug_node(&self.project, t).ok_or("Node does not exist")?;
                }
                self.debug_focus = target;
                return Ok(());
            }
            Command::HistoryJump { offset } => {
                let count = offset.unsigned_abs() as usize;
                if count
                    > if offset < 0 {
                        self.undo.len()
                    } else {
                        self.redo.len()
                    }
                {
                    return Err("History entry is no longer available".into());
                }
                if count == 0 {
                    return Ok(());
                }
                let (from, to) = if offset < 0 {
                    (&mut self.undo, &mut self.redo)
                } else {
                    (&mut self.redo, &mut self.undo)
                };
                let mut span = from.split_off(from.len() - count);
                let (project, scripts) = span.remove(0);
                to.push((
                    Arc::new(std::mem::replace(
                        &mut self.project,
                        Arc::unwrap_or_clone(project),
                    )),
                    self.scripts.clone(),
                ));
                to.extend(span.into_iter().rev());
                self.scripts = scripts;
                self.nodes.clear();
                self.debug_focus = self
                    .debug_focus
                    .take()
                    .filter(|t| debug_node(&self.project, t).is_some());
                self.revision += 1;
                return Ok(());
            }
            Command::SaveCheckpoint { name } => {
                crate::session::validate_checkpoint_name(&name)?;
                if self.checkpoints.contains_key(&name) {
                    return Err("A checkpoint with this name already exists".into());
                }
                if self.checkpoints.len() >= 32 {
                    return Err("At most 32 named checkpoints".into());
                }
                let mut project = self.project.clone();
                project.output.enabled = false;
                self.checkpoints.insert(name, Arc::new(project));
                self.revision += 1;
                return Ok(());
            }
            Command::RemoveCheckpoint { name } => {
                self.checkpoints
                    .remove(&name)
                    .ok_or("Checkpoint not found")?;
                self.revision += 1;
                return Ok(());
            }
            Command::RestoreCheckpoint { name } => {
                let project = self
                    .checkpoints
                    .get(&name)
                    .ok_or("Checkpoint not found")?
                    .as_ref()
                    .clone();
                return self.apply_inner(Command::ReplaceProject {
                    project: Box::new(project),
                });
            }
            Command::PreparedInput { event } => {
                std::sync::Arc::make_mut(&mut self.inputs).apply(event.0, self.frame + 1)?;
                return Ok(());
            }
            Command::Input { mut event } => {
                event.prepare()?;
                std::sync::Arc::make_mut(&mut self.inputs).apply(event, self.frame + 1)?;
                return Ok(());
            }
            Command::Pause { paused } => {
                self.paused = paused;
                return Ok(());
            }
            Command::Blackout { enabled } => {
                self.blackout = enabled;
                return Ok(());
            }
            Command::Undo => {
                let (p, scripts) = self.undo.pop().ok_or("Nothing to undo")?;
                self.redo.push((
                    Arc::new(std::mem::replace(
                        &mut self.project,
                        Arc::unwrap_or_clone(p),
                    )),
                    self.scripts.clone(),
                ));
                self.scripts = scripts;
                self.nodes.clear();
                self.debug_focus = self
                    .debug_focus
                    .take()
                    .filter(|t| debug_node(&self.project, t).is_some());
                self.revision += 1;
                return Ok(());
            }
            Command::Redo => {
                let (p, scripts) = self.redo.pop().ok_or("Nothing to redo")?;
                self.undo.push((
                    Arc::new(std::mem::replace(
                        &mut self.project,
                        Arc::unwrap_or_clone(p),
                    )),
                    self.scripts.clone(),
                ));
                self.scripts = scripts;
                self.nodes.clear();
                self.debug_focus = self
                    .debug_focus
                    .take()
                    .filter(|t| debug_node(&self.project, t).is_some());
                self.revision += 1;
                return Ok(());
            }
            _ => {}
        }
        let prepared = Self::prepare(self.project.clone(), command)?;
        self.install(prepared)
    }
    /// Build/validate/compile on a control worker; commit only on the owning engine thread.
    pub fn prepare(mut draft: Project, command: Command) -> Result<PreparedEdit> {
        let reset_runtime = matches!(
            command,
            Command::ReplaceProject { .. } | Command::ImportProject { .. }
        );
        edit(&mut draft, command, 0)?;
        draft.validate()?;
        let scripts = compile_scripts(&draft, &script_engine())?;
        Ok(PreparedEdit {
            reset_runtime,
            plugin_generation: scenic::active_plugins().state.generation,
            record_history: true,
            project: draft,
            scripts,
        })
    }
    pub(crate) fn prepare_internal(project: Project, command: Command) -> Result<PreparedEdit> {
        let mut prepared = Self::prepare(project, command)?;
        prepared.record_history = false;
        Ok(prepared)
    }
    pub(crate) fn host(&self) -> std::sync::Arc<crate::inputs::Inputs> {
        self.inputs.clone()
    }
    pub fn install(&mut self, prepared: PreparedEdit) -> Result<()> {
        let PreparedEdit {
            reset_runtime,
            plugin_generation,
            record_history,
            project: draft,
            scripts,
        } = prepared;
        if plugin_generation != self.plugins.state.generation {
            return Err("Plugin registry changed while preparing edit; retry".into());
        }
        if draft == self.project && !reset_runtime {
            return Ok(());
        }
        // Retain native state only for nodes whose authored definition did not change.
        let previous = self.project.clone();
        let mut preserve = HashMap::new();
        for (fixture, layers) in draft
            .content
            .iter()
            .chain(std::iter::once((&"__director".to_owned(), &draft.director)))
        {
            for (layer, stack) in layers.iter().enumerate() {
                if let Some(stack) = stack {
                    if let Some(Some(old)) = if fixture == "__director" {
                        previous.director.get(layer)
                    } else {
                        previous.content.get(fixture).and_then(|v| v.get(layer))
                    } {
                        for (key, state) in &self.nodes {
                            let prefix = format!("{fixture}/{layer}/");
                            if let Some(id) = key.strip_prefix(&prefix).and_then(|s| s.parse().ok())
                            {
                                let current = stack.root.find(id);
                                let before = old.root.find(id);
                                let retained_motion = current.zip(before).is_some_and(|(a, b)| {
                                    a.behavior == b.behavior
                                        && matches!(
                                            a.behavior.as_str(),
                                            "Create.Value.Random"
                                                | "Create.Position.Random"
                                                | "Create.Position.Group.Random"
                                                | "Create.Position.Group.Slide"
                                                | "Modify.Value.RandomizeTrigger"
                                        )
                                        && a.children == b.children
                                        && a.drivers == b.drivers
                                        && a.properties.get("Seed") == b.properties.get("Seed")
                                });
                                let retained_native = current.zip(before).is_some_and(|(a, b)| {
                                    a.behavior == b.behavior
                                        && crate::native::supported(&a.behavior)
                                        && a.properties.get("Seed") == b.properties.get("Seed")
                                });
                                if current == before || retained_motion || retained_native {
                                    let mut state = state.clone();
                                    if let Some(node) = current {
                                        state
                                            .driven
                                            .retain(|key, _| node.drivers.contains_key(key));
                                    }
                                    preserve.insert(key.clone(), state);
                                }
                            }
                        }
                    }
                }
            }
        }
        for (key, state) in &self.nodes {
            if key.contains("@plugin/") {
                let mut path = key.split('/');
                let fixture = path.next().unwrap_or("");
                let layer = path.next().and_then(|s| s.parse::<usize>().ok());
                let owner = path
                    .next()
                    .and_then(|s| s.split('@').next())
                    .and_then(|s| s.parse::<u64>().ok());
                fn roots<'a>(
                    p: &'a Project,
                    fixture: &str,
                    layer: Option<usize>,
                ) -> Option<&'a crate::model::Stack> {
                    if fixture == "__director" {
                        p.director.get(layer.unwrap_or(99)).and_then(Option::as_ref)
                    } else {
                        p.content
                            .get(fixture)
                            .and_then(|v| v.get(layer.unwrap_or(99)))
                            .and_then(Option::as_ref)
                    }
                }
                if let (Some(before), Some(current), Some(owner)) = (
                    roots(&previous, fixture, layer),
                    roots(&draft, fixture, layer),
                    owner,
                ) {
                    if before
                        .root
                        .find(owner)
                        .zip(current.root.find(owner))
                        .is_some_and(|(a, b)| a.behavior == b.behavior)
                    {
                        preserve.insert(key.clone(), state.clone());
                    }
                }
            }
        }
        self.nodes = if reset_runtime {
            HashMap::new()
        } else {
            preserve
        };
        if record_history {
            self.undo.push((Arc::new(previous), self.scripts.clone()));
        }
        if self.undo.len() > 128 {
            self.undo.remove(0);
        }
        if record_history {
            self.redo.clear();
        }
        self.debug_focus = self
            .debug_focus
            .take()
            .filter(|t| debug_node(&draft, t).is_some());
        self.project = draft;
        self.scripts = scripts;
        self.revision += 1;
        Ok(())
    }
    pub fn tick(&mut self, dt: f64) {
        scenic::with_plugins(self.plugins.clone(), || self.tick_inner(dt));
    }
    fn tick_inner(&mut self, dt: f64) {
        let started = crate::clock::Instant::now();
        if !self.paused {
            self.time += dt.clamp(0., 0.25)
        }
        self.frame += 1;
        let mut lighting = crate::lighting::Frame::default();
        fn has_lighting(n: &Node, plugins: &crate::plugins::Registry) -> bool {
            n.behavior.starts_with("Light.")
                || n.children
                    .iter()
                    .chain(n.drivers.values())
                    .any(|n| has_lighting(n, plugins))
                || plugins
                    .component(&n.behavior)
                    .is_some_and(|c| c.enabled && has_lighting(&c.root, plugins))
        }
        if self
            .project
            .content
            .values()
            .flatten()
            .flatten()
            .any(|s| has_lighting(&s.root, &self.plugins))
        {
            // Probe on cloned state: each actual generator/driver advances only once.
            let mut probe_nodes = self.nodes.clone();
            let probe = self.evaluate_scene(
                &mut probe_nodes,
                &lighting,
                &mut crate::inspection::Capture::default(),
            );
            lighting.emitters = probe.emitters;
            lighting.positions = probe
                .fixtures
                .iter()
                .map(|f| {
                    (
                        f.fixture.id.clone(),
                        f.result.position.unwrap_or(f.fixture.position),
                    )
                })
                .collect();
        }
        let mut nodes = std::mem::take(&mut self.nodes);
        let mut capture =
            crate::inspection::Capture::new(std::mem::take(&mut self.inspection_pending));
        let mut snap = self.evaluate_scene(&mut nodes, &lighting, &mut capture);
        if !capture.targets.is_empty() {
            self.inspection_frame.frame = self.frame;
            self.inspection_frame.revision = self.revision;
            self.inspection_frame
                .blocks
                .retain(|b| b.revision == self.revision);
            for mut block in capture.blocks {
                block.frame = self.frame;
                block.revision = self.revision;
                self.inspection_frame
                    .blocks
                    .retain(|b| b.target != block.target);
                self.inspection_frame.blocks.push(block);
            }
            let excess = self
                .inspection_frame
                .blocks
                .len()
                .saturating_sub(crate::inspection::MAX_TARGETS);
            self.inspection_frame.blocks.drain(..excess);
        }
        self.nodes = nodes;
        snap.evaluation_us = started.elapsed().as_micros() as u64;
        self.last = snap;
    }
    pub fn set_dimming_clock(
        &mut self,
        control: crate::dimming_clock::Control,
    ) -> Result<crate::dimming_clock::Status> {
        self.dimming_clock
            .control_at(control, crate::clock::Instant::now())?;
        Ok(self.dimming_status())
    }
    pub fn dimming_status(&self) -> crate::dimming_clock::Status {
        self.dimming_clock.status_at(
            self.inputs.clock_seconds,
            &self.project.dimming,
            crate::clock::Instant::now(),
        )
    }
    pub fn request_inspection(
        &mut self,
        targets: Vec<DebugFocus>,
    ) -> Result<crate::inspection::Frame> {
        if targets.len() > crate::inspection::MAX_TARGETS {
            return Err("At most 256 inspected blocks".into());
        }
        for target in &targets {
            debug_node(&self.project, target).ok_or("Inspected block no longer exists")?;
        }
        let mut pending = self.inspection_pending.clone();
        for target in &targets {
            if !pending.contains(target) {
                pending.push(target.clone());
            }
        }
        if pending.len() > crate::inspection::MAX_TARGETS {
            return Err("Inspection queue is full; retry shortly".into());
        }
        self.inspection_pending = pending;
        Ok(crate::inspection::Frame {
            frame: self.inspection_frame.frame,
            revision: self.inspection_frame.revision,
            blocks: self
                .inspection_frame
                .blocks
                .iter()
                .filter(|b| {
                    b.revision == self.revision
                        && self.frame.saturating_sub(b.frame) <= 120
                        && (targets.is_empty() || targets.contains(&b.target))
                })
                .cloned()
                .collect(),
        })
    }

    fn evaluate_scene(
        &self,
        nodes: &mut HashMap<String, NodeState>,
        lighting: &crate::lighting::Frame,
        capture: &mut crate::inspection::Capture,
    ) -> Snapshot {
        let dimming_level = self.dimming_status().level;
        let mut snap = Snapshot {
            debug_focus: self.debug_focus.clone(),
            frame: self.frame,
            revision: self.revision,
            plugin_generation: self.plugins.state.generation,
            time: self.time,
            paused: self.paused,
            blackout: self.blackout,
            can_undo: !self.undo.is_empty(),
            can_redo: !self.redo.is_empty(),
            ..Snapshot::default()
        };
        // Match the native live-object ordering: earlier fixtures publish this tick;
        // later fixtures retain their previous evaluated Position, with setup fallback.
        let mut live_positions: BTreeMap<String, [f64; 3]> = self
            .project
            .fixtures
            .iter()
            .map(|f| {
                let position = self
                    .last
                    .fixtures
                    .iter()
                    .find(|s| s.fixture.id == f.id)
                    .and_then(|s| s.result.position)
                    .unwrap_or(f.position);
                (f.id.clone(), position)
            })
            .collect();
        let mut live_layers: BTreeMap<String, Vec<Bank>> = self
            .last
            .fixtures
            .iter()
            .map(|f| (f.fixture.id.clone(), f.layers.clone()))
            .collect();
        for (index, fixture) in self.project.fixtures.iter().enumerate() {
            let mut effective_fixture = fixture.clone();
            if let Some(positions) = self.inputs.pixels.get(&fixture.id) {
                effective_fixture.pixel_positions = positions.clone();
            }
            let fixture = &effective_fixture;
            let layers = &self.project.content[&fixture.id];
            let mut higher = Vec::new();
            let mut results = vec![Bank::default(); layers.len()];
            let mut trace = BTreeMap::new();
            for layer in (0..layers.len()).rev() {
                let result = if let Some(stack) = layers[layer].as_ref().filter(|s| s.enabled) {
                    let mut ctx = Context {
                        fixture,
                        host: &self.inputs,
                        frame: self.frame,
                        prior: &self.last,
                        index,
                        layer,
                        lighting,
                        fixtures: &self.project.fixtures,
                        live_positions: &live_positions,
                        live_layers: &live_layers,
                        time: self.time,
                        higher: &higher,
                        prefix: format!("{}/{layer}", fixture.id),
                        nodes,
                        scripts: &self.scripts,
                        script_engine: &self.script_engine,
                        plugins: self.plugins.clone(),
                        parameters: Vec::new(),
                        plugin_inputs: Vec::new(),
                        bindings: Vec::new(),
                        diagnostics: &mut snap.diagnostics,
                        trace: &mut trace,
                        capture,
                        budget: 4096,
                    };
                    evaluate(&stack.root, &mut ctx)
                } else {
                    Bank::default()
                };
                results[layer] = result.clone();
                higher.insert(0, result);
            }
            for bank in &results {
                snap.actions.extend(bank.actions.clone());
                for emitter in bank.emitters.iter().filter(|_| {
                    self.debug_focus
                        .as_ref()
                        .is_none_or(|t| t.fixture != fixture.id)
                }) {
                    if snap.emitters.len() < 1024
                        && !snap.emitters.iter().any(|e| {
                            e.fixture == emitter.fixture
                                && e.layer == emitter.layer
                                && e.node == emitter.node
                        })
                    {
                        snap.emitters.push(emitter.clone());
                    }
                }
            }
            live_layers.insert(fixture.id.clone(), results.clone());
            let result = if let Some(t) = self
                .debug_focus
                .as_ref()
                .filter(|t| t.fixture == fixture.id)
            {
                trace
                    .get(&format!("{}/{}", t.layer, t.node))
                    .cloned()
                    .unwrap_or_default()
            } else {
                results[0].clone()
            };
            if self
                .debug_focus
                .as_ref()
                .is_some_and(|t| t.fixture == fixture.id)
            {
                for emitter in &result.emitters {
                    if snap.emitters.len() < 1024
                        && !snap.emitters.iter().any(|e| {
                            e.fixture == emitter.fixture
                                && e.layer == emitter.layer
                                && e.node == emitter.node
                        })
                    {
                        snap.emitters.push(emitter.clone());
                    }
                }
            }
            live_positions.insert(
                fixture.id.clone(),
                result.position.unwrap_or(fixture.position),
            );
            let universe = snap
                .universes
                .entry(fixture.universe)
                .or_insert_with(|| vec![0; 512]);
            let bytes = crate::output_design::encode(
                &self.project,
                fixture,
                &result,
                self.blackout,
                dimming_level,
            );
            let start = fixture.address as usize - 1;
            if start + bytes.len() <= 512 {
                universe[start..start + bytes.len()].copy_from_slice(&bytes);
            } else {
                snap.diagnostics.push(format!(
                    "{}: runtime pixel footprint exceeds universe",
                    fixture.name
                ));
            }
            snap.fixtures.push(FixtureSnapshot {
                fixture: fixture.clone(),
                result,
                layers: results,
                nodes: trace,
            });
        }
        // Director programs run after all fixture programs and contribute deferred actions.
        let mut director_fixture =
            self.project
                .fixtures
                .first()
                .cloned()
                .unwrap_or_else(|| Fixture {
                    dmx_model: None,
                    id: "__director".into(),
                    name: "Director".into(),
                    position: [0.; 3],
                    groups: vec![],
                    model: FixtureModel::Dimmer,
                    universe: 0,
                    address: 1,
                    dimmer: 1.,
                    setup_channel: 0,
                    rotation: [0.; 3],
                    pixel_positions: vec![[0.; 3]],
                    pixel_reverse: false,
                });
        {
            let fixture = &mut director_fixture;
            fixture.id = "__director".into();
            let mut higher = Vec::new();
            snap.director = vec![Bank::default(); self.project.director.len()];
            let mut trace = BTreeMap::new();
            for layer in (0..self.project.director.len()).rev() {
                let result = if let Some(stack) =
                    self.project.director[layer].as_ref().filter(|s| s.enabled)
                {
                    let mut ctx = Context {
                        fixture,
                        host: &self.inputs,
                        frame: self.frame,
                        prior: &self.last,
                        index: 0,
                        layer,
                        lighting,
                        fixtures: &self.project.fixtures,
                        live_positions: &live_positions,
                        live_layers: &live_layers,
                        time: self.time,
                        higher: &higher,
                        prefix: format!("__director/{layer}"),
                        nodes,
                        scripts: &self.scripts,
                        script_engine: &self.script_engine,
                        plugins: self.plugins.clone(),
                        parameters: Vec::new(),
                        plugin_inputs: Vec::new(),
                        bindings: Vec::new(),
                        diagnostics: &mut snap.diagnostics,
                        trace: &mut trace,
                        capture,
                        budget: 4096,
                    };
                    evaluate(&stack.root, &mut ctx)
                } else {
                    Bank::default()
                };
                snap.actions.extend(result.actions.clone());
                snap.director[layer] = result.clone();
                higher.insert(0, result);
            }
            snap.director_nodes = trace;
        }
        for emitter in &mut snap.emitters {
            if emitter.follows_fixture {
                if let Some(position) = snap
                    .fixtures
                    .iter()
                    .find(|f| f.fixture.id == emitter.fixture)
                    .and_then(|f| f.result.position)
                {
                    emitter.position = position;
                }
            }
        }
        snap.channel_test = crate::dmx_test::apply_at(
            &self.inputs.channel_test,
            &mut snap.universes,
            self.blackout,
            crate::clock::Instant::now(),
        );
        snap
    }
}
fn layers_mut<'a>(p: &'a mut Project, fixture: &str) -> Result<&'a mut Vec<Option<Stack>>> {
    if fixture == "__director" {
        Ok(&mut p.director)
    } else {
        p.content
            .get_mut(fixture)
            .ok_or_else(|| "Unknown fixture".into())
    }
}
fn root_mut<'a>(p: &'a mut Project, fixture: &str, layer: usize) -> Result<&'a mut Node> {
    if fixture == "__director" {
        return p
            .director
            .get_mut(layer)
            .and_then(Option::as_mut)
            .map(|s| &mut s.root)
            .ok_or_else(|| "Missing director layer".into());
    }
    p.content
        .get_mut(fixture)
        .and_then(|v| v.get_mut(layer))
        .and_then(Option::as_mut)
        .map(|s| &mut s.root)
        .ok_or_else(|| "Missing fixture, layer or program".into())
}
fn target<'a>(p: &'a mut Project, fixture: &str, layer: usize, id: u64) -> Result<&'a mut Node> {
    root_mut(p, fixture, layer)?
        .find_mut(id)
        .ok_or_else(|| "Node does not exist".into())
}
fn fixture_mut<'a>(p: &'a mut Project, id: &str) -> Result<&'a mut Fixture> {
    p.fixtures
        .iter_mut()
        .find(|f| f.id == id)
        .ok_or_else(|| "Unknown fixture".into())
}
fn edit(p: &mut Project, c: Command, depth: usize) -> Result<()> {
    if depth > 8 {
        return Err("Command nesting exceeds limit".into());
    }
    match c {
        Command::SetNodeBypassed {
            fixture,
            layer,
            node,
            bypassed,
        } => target(p, &fixture, layer, node)?.bypassed = bypassed,
        Command::ImportProject { json } => {
            if depth != 0 {
                return Err("Project import must be a standalone command".into());
            }
            let mut project: Project =
                serde_json::from_value(crate::migrations::parse(json.as_bytes())?)
                    .map_err(|e| e.to_string())?;
            project.output.enabled = false;
            *p = project;
        }
        Command::ReplaceProject { project } => {
            if depth != 0 {
                return Err("Project replacement must be a standalone command".into());
            }
            *p = *project;
        }
        Command::AdoptNode {
            fixture,
            layer,
            node,
            child,
        } => {
            let owner = target(p, &fixture, layer, node)?;
            let replacement = owner
                .children
                .get(1)
                .filter(|n| n.id == child)
                .cloned()
                .ok_or("Transition adoption target changed")?;
            *target(p, &fixture, layer, node)? = replacement;
        }
        Command::ReplaceChildren {
            fixture,
            layer,
            node,
            mut children,
        } => {
            let mut next = root_mut(p, &fixture, layer)?.max_id() + 1;
            for child in &mut children {
                child.reidentify(&mut next);
            }
            target(p, &fixture, layer, node)?.children = children;
        }
        Command::SetDimming { timeline } => p.dimming = timeline,
        Command::UpsertDmxModel { id, model } => {
            p.dmx_models.insert(id, model);
        }
        Command::RemoveDmxModel { id } => {
            if p.fixtures
                .iter()
                .any(|f| f.dmx_model.as_deref() == Some(&id))
            {
                return Err("DMX model is in use".into());
            }
            p.dmx_models.remove(&id);
        }
        Command::MoveFixture { fixture, position } => fixture_mut(p, &fixture)?.position = position,
        Command::SetDimmer { fixture, value } => fixture_mut(p, &fixture)?.dimmer = value,
        Command::SetFixture { fixture } => {
            let id = fixture.id.clone();
            *fixture_mut(p, &id)? = fixture;
        }
        Command::AddFixture { fixture } => {
            p.content.insert(fixture.id.clone(), vec![None, None, None]);
            p.fixtures.push(fixture)
        }
        Command::RemoveFixture { fixture } => {
            fixture_mut(p, &fixture)?;
            p.fixtures.retain(|f| f.id != fixture);
            p.content.remove(&fixture);
        }
        Command::AddLayer { fixture } => {
            let layers = layers_mut(p, &fixture)?;
            if layers.len() >= MAX_LAYERS {
                return Err("Layer limit reached (32)".into());
            }
            layers.push(None);
        }
        Command::SetLayerEnabled {
            fixture,
            layer,
            enabled,
        } => {
            layers_mut(p, &fixture)?
                .get_mut(layer)
                .and_then(Option::as_mut)
                .ok_or("Layer has no program")?
                .enabled = enabled;
        }
        Command::ReplaceLayer {
            fixture,
            layer,
            stack,
        } => {
            if fixture == "__director" {
                *p.director.get_mut(layer).ok_or("Layer does not exist")? = stack;
            } else {
                fixture_mut(p, &fixture)?;
                *p.content
                    .get_mut(&fixture)
                    .and_then(|v| v.get_mut(layer))
                    .ok_or("Layer does not exist")? = stack;
            }
        }
        Command::ApplyContent { entries } => {
            let mut seen = std::collections::HashSet::new();
            for entry in entries {
                if entry.layer >= MAX_LAYERS || !seen.insert((entry.fixture.clone(), entry.layer)) {
                    return Err("Invalid or duplicate content target".into());
                }
                let layers = layers_mut(p, &entry.fixture)?;
                layers.resize_with(layers.len().max(entry.layer + 1), || None);
                layers[entry.layer] = entry.stack;
            }
        }
        Command::SetProperty {
            fixture,
            layer,
            node,
            key,
            value,
        } => {
            let n = target(p, &fixture, layer, node)?;
            if !scenic::defaults(&n.behavior)?.contains_key(&key) {
                return Err("Unknown property".into());
            }
            n.properties.insert(key, value);
        }
        Command::SetDriver {
            fixture,
            layer,
            node,
            key,
            driver,
        } => {
            let root = root_mut(p, &fixture, layer)?;
            let mut next = root.max_id() + 1;
            let n = root.find_mut(node).ok_or("Unknown node")?;
            if let Some(mut driver) = driver {
                driver.reidentify(&mut next);
                n.drivers.insert(key, driver);
            } else {
                n.drivers.remove(&key);
            }
        }
        Command::AddInput {
            index,
            fixture,
            layer,
            node,
            behavior,
        } => {
            let root = root_mut(p, &fixture, layer)?;
            let next = root.max_id() + 1;
            let target = root.find_mut(node).ok_or("Unknown node")?;
            let index = index.unwrap_or(target.children.len());
            if index > target.children.len() {
                return Err("Invalid input index".into());
            }
            target
                .children
                .insert(index, scenic::new_node(&behavior, next)?);
        }
        Command::WrapNode {
            fixture,
            layer,
            node,
            behavior,
        } => {
            let root = root_mut(p, &fixture, layer)?;
            let next = root.max_id() + 1;
            let n = root.find_mut(node).ok_or("Unknown node")?;
            let mut wrapper = scenic::new_node(&behavior, next)?;
            wrapper.children.push(n.clone());
            *n = wrapper;
        }
        Command::MoveNode {
            fixture,
            layer,
            node,
            target,
            branch,
            below,
        } => {
            let root = root_mut(p, &fixture, layer)?;
            if root.id == node {
                return Err("The attached root cannot be moved; wrap it instead".into());
            }
            let source = root.find(node).ok_or("Source node missing")?;
            if source.find(target).is_some() || root.find(target).is_none() {
                return Err("Invalid target or cycle-forming move".into());
            }
            fn detach(n: &mut Node, id: u64, branch: bool) -> Result<Option<Node>> {
                if let Some(index) = n.children.iter().position(|c| c.id == id) {
                    let mut moving = n.children.remove(index);
                    if !branch {
                        n.children.splice(index..index, moving.children.drain(..));
                    }
                    return Ok(Some(moving));
                }
                let driver_key = n
                    .drivers
                    .iter()
                    .find(|(_, driver)| driver.id == id)
                    .map(|(key, _)| key.clone());
                if let Some(key) = driver_key {
                    let mut moving = n.drivers.remove(&key).ok_or("Driver disappeared")?;
                    if !branch {
                        if moving.children.len() > 1 {
                            return Err(
                                "A driver root with multiple inputs must move as a branch".into()
                            );
                        }
                        if let Some(child) = moving.children.pop() {
                            n.drivers.insert(key, child);
                        }
                    }
                    return Ok(Some(moving));
                }
                for child in n.children.iter_mut().chain(n.drivers.values_mut()) {
                    if let Some(moving) = detach(child, id, branch)? {
                        return Ok(Some(moving));
                    }
                }
                Ok(None)
            }
            let mut moving = detach(root, node, branch)?.ok_or("Source node missing")?;
            let target = root.find_mut(target).ok_or("Target disappeared")?;
            if below {
                std::mem::swap(target, &mut moving);
                target.children.push(moving);
            } else {
                target.children.push(moving);
            }
        }
        Command::DeleteNode {
            fixture,
            layer,
            node,
            branch,
        } => {
            let root = root_mut(p, &fixture, layer)?;
            if root.id == node {
                if branch || root.children.is_empty() {
                    edit(
                        p,
                        Command::ReplaceLayer {
                            fixture,
                            layer,
                            stack: None,
                        },
                        depth + 1,
                    )?;
                } else if root.children.len() == 1 {
                    *root = root.children.remove(0);
                } else {
                    return Err(
                        "Root has multiple children; remove the branch or its inputs".into(),
                    );
                }
            } else {
                fn remove(n: &mut Node, id: u64, branch: bool) -> bool {
                    if let Some(i) = n.children.iter().position(|c| c.id == id) {
                        let mut child = n.children.remove(i);
                        if !branch {
                            n.children.splice(i..i, child.children.drain(..));
                        }
                        return true;
                    }
                    for c in n.children.iter_mut().chain(n.drivers.values_mut()) {
                        if remove(c, id, branch) {
                            return true;
                        }
                    }
                    false
                }
                if !remove(root, node, branch) {
                    return Err("Node missing; remove drivers through the driver command".into());
                }
            }
        }
        Command::SetOutput { config } => p.output = config,
        Command::Batch { commands } => {
            if commands.len() > 512 {
                return Err("Too many batch commands".into());
            }
            for command in commands {
                edit(p, command, depth + 1)?
            }
        }
        _ => return Err("History and transport controls cannot be nested in a transaction".into()),
    }
    Ok(())
}
fn script_engine() -> rhai::Engine {
    let mut e = rhai::Engine::new();
    e.set_max_operations(2000)
        .set_max_call_levels(16)
        .set_max_expr_depths(32, 16)
        .set_max_string_size(4096)
        .set_max_array_size(128)
        .set_max_map_size(64);
    for (name, function) in [
        ("Sin", f64::sin as fn(f64) -> f64),
        ("Cos", f64::cos),
        ("Tan", f64::tan),
        ("Asin", f64::asin),
        ("Acos", f64::acos),
        ("Atan", f64::atan),
        ("Abs", f64::abs),
        ("Ceiling", f64::ceil),
        ("Floor", f64::floor),
        ("Exp", f64::exp),
        ("Log10", f64::log10),
        ("Sqrt", f64::sqrt),
        ("Round", f64::round),
        ("Truncate", f64::trunc),
    ] {
        e.register_fn(name, function);
    }
    e.register_fn("Pow", f64::powf)
        .register_fn("Min", f64::min)
        .register_fn("Max", f64::max)
        .register_fn("Log", f64::log)
        .register_fn(
            "If",
            |condition: bool, a: f64, b: f64| if condition { a } else { b },
        );
    e.disable_symbol("eval");
    e.on_print(|_| {});
    e.on_debug(|_, _, _| {});
    e
}
fn compile_scripts(p: &Project, e: &rhai::Engine) -> Result<HashMap<String, rhai::AST>> {
    fn walk(n: &Node, e: &rhai::Engine, out: &mut HashMap<String, rhai::AST>) -> Result<()> {
        if matches!(
            n.behavior.as_str(),
            "Script.Value" | "Create.Value.Expression"
        ) {
            let code = n
                .properties
                .get(if n.behavior == "Script.Value" {
                    "Code"
                } else {
                    "Expression"
                })
                .and_then(Value::as_str)
                .unwrap_or("input");
            match e.compile(code) {
                Ok(ast) => {
                    out.insert(code.into(), ast);
                }
                Err(_) if n.behavior == "Create.Value.Expression" => {}
                Err(error) => return Err(format!("Script compile: {error}")),
            }
        }
        for c in n.children.iter().chain(n.drivers.values()) {
            walk(c, e, out)?
        }
        Ok(())
    }
    let mut out = HashMap::new();
    for s in p
        .content
        .values()
        .chain(std::iter::once(&p.director))
        .flatten()
        .flatten()
    {
        walk(&s.root, e, &mut out)?
    }
    for component in scenic::active_plugins().components().filter(|c| c.enabled) {
        walk(&component.root, e, &mut out)?;
    }
    Ok(out)
}
struct Context<'a> {
    host: &'a crate::inputs::Inputs,
    frame: u64,
    prior: &'a Snapshot,
    fixture: &'a Fixture,
    index: usize,
    layer: usize,
    lighting: &'a crate::lighting::Frame,
    fixtures: &'a [Fixture],
    live_positions: &'a BTreeMap<String, [f64; 3]>,
    live_layers: &'a BTreeMap<String, Vec<Bank>>,
    time: f64,
    higher: &'a [Bank],
    prefix: String,
    nodes: &'a mut HashMap<String, NodeState>,
    scripts: &'a HashMap<String, rhai::AST>,
    script_engine: &'a rhai::Engine,
    plugins: Arc<crate::plugins::Registry>,
    parameters: Vec<BTreeMap<String, Value>>,
    plugin_inputs: Vec<Bank>,
    bindings: Vec<Arc<crate::plugins::Bindings>>,
    diagnostics: &'a mut Vec<String>,
    trace: &'a mut BTreeMap<String, Bank>,
    capture: &'a mut crate::inspection::Capture,
    budget: usize,
}
fn evaluate(n: &Node, c: &mut Context<'_>) -> Bank {
    if c.budget == 0 {
        c.diagnostics
            .push("Evaluation node budget exhausted".into());
        return Bank::default();
    }
    c.budget -= 1;
    let inputs: Vec<_> = n.children.iter().map(|n| evaluate(n, c)).collect();
    let mut bank = inputs.iter().cloned().fold(Bank::default(), Bank::merge);
    let incoming = (!c.prefix.contains("@plugin/")
        && c.capture.wants(&c.fixture.id, c.layer, n.id))
    .then(|| crate::inspection::Attributes::capture(&bank));
    if n.bypassed {
        if let Some(incoming) = incoming {
            c.capture.record(
                DebugFocus {
                    fixture: c.fixture.id.clone(),
                    layer: c.layer,
                    node: n.id,
                },
                incoming,
                &bank,
                true,
            );
        }
        if !c.prefix.contains("@plugin/") {
            c.trace
                .insert(format!("{}/{}", c.layer, n.id), bank.clone());
        }
        return bank;
    }
    match crate::native::specs()
        .get(&n.behavior)
        .map(|s| s.basis.as_str())
    {
        Some("first") => bank = inputs.first().cloned().unwrap_or_default(),
        Some("fresh") => bank = Bank::default(),
        _ => {}
    }

    let id = format!("{}/{}", c.prefix, n.id);
    let mut state = c.nodes.remove(&id).unwrap_or_default();
    let mut props = scenic::defaults(&n.behavior).unwrap_or_default();
    props.extend(n.properties.clone());
    props.extend(state.driven.clone());
    if let Some(bindings) = c.bindings.last().and_then(|b| b.get(&n.id)) {
        for (key, name) in bindings {
            if let Some(value) = c.parameters.last().and_then(|p| p.get(name)) {
                props.insert(key.clone(), value.clone());
            }
        }
    }
    if matches!(
        n.behavior.as_str(),
        "Create.Position.Group.Slide" | "Create.Position.Group.Random" | "Create.Rotation.Timed"
    ) {
        let base = props.get("Speed").and_then(scenic::number).unwrap_or(0.5);
        let amount = props
            .get("Speed Audio")
            .and_then(scenic::number)
            .unwrap_or(0.);
        let level = c.host.audio.get(&0).map_or(0., |a| a.level);
        props.insert("Speed".into(), json!((base + level * amount).min(1.)));
    }
    let num = |key: &str, default: f64| props.get(key).and_then(scenic::number).unwrap_or(default);
    let value = bank.value.unwrap_or(0.);
    match n.behavior.as_str() {
        "Plugin.Input" => {
            bank = c.plugin_inputs.last().cloned().unwrap_or_else(|| {
                c.diagnostics
                    .push("Plugin.Input requires a component context".into());
                Bank::default()
            });
        }
        "Create.Value.Constant" => bank.value = Some(num("Value", 1.)),
        "Create.Value.LFO" => {
            let phase = (c.time / num("Period", 1.).max(0.0001)).rem_euclid(1.);
            bank.value = Some(match num("Type", 0.) as i64 {
                1 => 1. - (2. * phase - 1.).abs(),
                2 => 0.5 - 0.5 * (phase * std::f64::consts::TAU).cos(),
                _ => phase,
            });
        }
        "Create.Position.Current" => bank.position = Some(c.fixture.position),
        "Create.Position.XYZ" => bank.position = Some([num("X", 0.), num("Y", 0.), num("Z", 0.)]),
        "Create.Position.Bezier" | "Create.Value.Bezier" => {
            let t = crate::motion::progress(
                num("Progress", -1.),
                bank.value,
                c.time,
                num("Duration", 4.),
                props.get("Loop").and_then(Value::as_bool).unwrap_or(true),
                props
                    .get("Ping Pong")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            );
            if n.behavior == "Create.Position.Bezier" {
                let points =
                    ["P0", "P1", "P2", "P3"].map(|k| scenic::vector(&props[k]).unwrap_or([0.; 3]));
                bank.position = Some(std::array::from_fn(|i| {
                    crate::motion::cubic(points.map(|p| p[i]), t)
                }));
            } else {
                bank.value = Some(crate::motion::cubic(
                    [
                        num("Start", 0.),
                        num("Control1", 0.),
                        num("Control2", 1.),
                        num("End", 1.),
                    ],
                    t,
                ));
            }
        }
        "Create.Value.Random" | "Create.Position.Random" => {
            let sample = state.motion.random(crate::motion::RandomParams {
                time: c.time,
                seed: crate::motion::seed(&id, num("Seed", 0.)),
                interval: num("Interval", 1.),
                filter: num("Filter Time", 1.),
                jitter: num("Randomness", 0.),
                paused: props.get("Pause").and_then(Value::as_bool).unwrap_or(false),
            });
            if n.behavior == "Create.Value.Random" {
                bank.value = Some(sample[0]);
            } else {
                let a = scenic::vector(&props["Minimum"]).unwrap_or([-1.; 3]);
                let b = scenic::vector(&props["Maximum"]).unwrap_or([1.; 3]);
                bank.position = Some(std::array::from_fn(|i| a[i] + sample[i] * (b[i] - a[i])));
            }
        }
        "Modify.Value.RandomizeTrigger" => {
            state.motion.init(crate::motion::seed(&id, num("Seed", 0.)));
            let high = value > 0.5;
            if high && !state.motion.trigger {
                state.motion.admitted = state.motion.draw() < num("Probability", 0.5).clamp(0., 1.);
            }
            state.motion.trigger = high;
            bank.value = Some(if high && state.motion.admitted {
                1.
            } else {
                0.
            });
        }
        "Create.Position.Group.Center"
        | "Create.Position.Group.XYZ"
        | "Create.Position.Group.Slide"
        | "Create.Position.Group.Random" => {
            let group = props["Group"].as_str().unwrap_or("");
            if let Some((mut min, mut max)) = crate::lighting::group_bounds(c.fixtures, group) {
                let padding = num("Padding", 0.);
                for i in 0..3 {
                    min[i] -= padding;
                    max[i] += padding;
                }
                let mut position = std::array::from_fn(|i| (min[i] + max[i]) / 2.);
                match n.behavior.as_str() {
                    "Create.Position.Group.XYZ" => {
                        let reverse = props["Reverse"].as_bool().unwrap_or(false);
                        let xyz = [num("X", 0.5), num("Y", 0.5), num("Z", 0.5)];
                        position = std::array::from_fn(|i| {
                            let t = if reverse { 1. - xyz[i] } else { xyz[i] };
                            min[i] + (max[i] - min[i]) * t
                        });
                    }
                    "Create.Position.Group.Slide" => {
                        let period = 100. - 99.5 * num("Speed", 0.5).clamp(0., 1.);
                        let dt = state.motion.delta(c.time);
                        state.motion.phase = (state.motion.phase + dt / period).rem_euclid(1.);
                        let t = if props["Reverse"].as_bool().unwrap_or(false) {
                            1. - state.motion.phase
                        } else {
                            state.motion.phase
                        };
                        let axis = (num("Axis", 0.) as usize).min(2);
                        position[axis] = min[axis] + (max[axis] - min[axis]) * t;
                    }
                    "Create.Position.Group.Random" => {
                        let period = 100. - 99.8 * num("Speed", 0.5).clamp(0., 1.);
                        let filter = if props["Continuous"].as_bool().unwrap_or(true) {
                            period
                        } else {
                            0.
                        };
                        let sample = state.motion.random(crate::motion::RandomParams {
                            time: c.time,
                            seed: crate::motion::seed(&id, num("Seed", 0.)),
                            interval: period,
                            filter,
                            jitter: 0.,
                            paused: false,
                        });
                        position = std::array::from_fn(|i| min[i] + (max[i] - min[i]) * sample[i]);
                    }
                    _ => {}
                }
                bank.position = Some(position);
            } else {
                bank.position = None;
                c.diagnostics.push(format!(
                    "{} node {}: group '{}' has no fixtures",
                    c.fixture.name, n.id, group
                ));
            }
        }
        "Light.Emitter" => {
            if props["Enabled"].as_bool().unwrap_or(true) {
                let tint =
                    scenic::color(props["Color"].as_str().unwrap_or("#ffffff")).unwrap_or([1.; 4]);
                let input = bank
                    .colors
                    .as_ref()
                    .and_then(|c| c.first())
                    .copied()
                    .unwrap_or([1.; 4]);
                let color = std::array::from_fn(|i| input[i] * tint[i]);
                let intensity = num("Intensity", 1.).max(0.) * bank.value.unwrap_or(1.).max(0.);
                bank.emitters.push(crate::lighting::Emitter {
                    fixture: c.fixture.id.clone(),
                    node: if c.prefix.contains("@plugin/") {
                        use std::hash::{Hash, Hasher};
                        let mut hash = std::collections::hash_map::DefaultHasher::new();
                        id.hash(&mut hash);
                        hash.finish() | (1_u64 << 63)
                    } else {
                        n.id
                    },
                    layer: c.layer,
                    position: bank.position.unwrap_or(c.fixture.position),
                    follows_fixture: bank.position.is_none(),
                    direction: scenic::vector(&props["Direction"]).unwrap_or([0., -1., 0.]),
                    kind: num("Type", 0.) as u8,
                    color,
                    intensity: intensity * color[3],
                    range: num("Range", 10.).max(0.001),
                    falloff: num("Falloff", 2.).max(0.),
                    angle: num("Angle", 60.).clamp(0.001, 179.),
                    softness: num("Softness", 0.2).clamp(0., 1.),
                    width: num("Width", 2.).max(0.001),
                    height: num("Height", 2.).max(0.001),
                    two_sided: props["Two Sided"].as_bool().unwrap_or(false),
                    channel: props["Channel"].as_str().unwrap_or("default").to_owned(),
                });
                if bank.colors.is_none() {
                    bank.colors = Some(vec![[
                        color[0] * intensity,
                        color[1] * intensity,
                        color[2] * intensity,
                        color[3],
                    ]]);
                }
            }
        }
        "Light.Receiver" => {
            let point = bank
                .position
                .or_else(|| c.lighting.positions.get(&c.fixture.id).copied())
                .unwrap_or(c.fixture.position);
            let reference = bank
                .fixture_reference
                .take()
                .unwrap_or_else(|| props["Fixture"].as_str().unwrap_or("").to_owned());
            let channel = props["Channel"].as_str().unwrap_or("default");
            let include_self = props["Include Self"].as_bool().unwrap_or(false);
            let mut rgb = [0.; 3];
            let mut received = 0.;
            for emitter in &c.lighting.emitters {
                let matches_reference = reference.is_empty()
                    || emitter.fixture == reference
                    || c.fixtures
                        .iter()
                        .any(|f| f.id == emitter.fixture && f.name == reference);
                if (!include_self && emitter.fixture == c.fixture.id)
                    || (!channel.is_empty() && emitter.channel != channel)
                    || !matches_reference
                {
                    continue;
                }
                let amount = emitter.sample(point) * num("Gain", 1.).max(0.);
                received += amount;
                for (i, r) in rgb.iter_mut().enumerate() {
                    *r += emitter.color[i] * amount;
                }
            }
            if !reference.is_empty()
                && !c
                    .fixtures
                    .iter()
                    .any(|f| f.id == reference || f.name == reference)
            {
                c.diagnostics.push(format!(
                    "{} node {}: light fixture '{}' not found",
                    c.fixture.name, n.id, reference
                ));
            }
            bank.value = Some(received);
            bank.colors = Some(vec![[rgb[0], rgb[1], rgb[2], 1.]]);
        }
        "Create.Color.Constant" => {
            let v = num("Value", 1.);
            bank.colors = Some(vec![
                [v, v, v, 1.];
                crate::native::pixels::count(&bank, c.fixture)
            ])
        }
        "Create.Color.Color" => {
            let color = scenic::color(props["Color"].as_str().unwrap_or("#ffffff"))
                .unwrap_or([0., 0., 0., 1.]);
            bank.colors = Some(vec![color; crate::native::pixels::count(&bank, c.fixture)])
        }
        "Create.Layer.Above" => {
            bank = c
                .higher
                .get(num("Layer", 0.) as usize)
                .cloned()
                .unwrap_or_default()
        }
        "Create.Position.Constant" => {
            bank.position = Some(scenic::vector(&props["Position"]).unwrap_or([0.; 3]));
        }
        "Create.Reference.Fixture" => {
            bank.fixture_reference =
                Some(props["Fixture"].as_str().unwrap_or("Fixture").to_owned());
        }
        "Spatial.Distance" | "Create.Value.Distance" => {
            // A child reference takes precedence, then is consumed, as in native Schéma.
            let reference = bank.fixture_reference.take().unwrap_or_else(|| {
                props
                    .get("Fixture")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_owned()
            });
            let source = if props
                .get("Auto/Setup")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                c.fixture.position
            } else {
                bank.position
                    .or_else(|| c.live_positions.get(&c.fixture.id).copied())
                    .unwrap_or(c.fixture.position)
            };
            if reference.is_empty() && n.behavior == "Spatial.Distance" {
                let point = scenic::vector(&props["Position"]).unwrap_or([0.; 3]);
                bank.value = Some(distance(source, point));
            } else if let Some(target) = c
                .fixtures
                .iter()
                .find(|f| f.id == reference)
                .or_else(|| c.fixtures.iter().find(|f| f.name == reference))
            {
                let position = c
                    .live_positions
                    .get(&target.id)
                    .copied()
                    .unwrap_or(target.position);
                bank.value = Some(distance(source, position));
            } else {
                bank.value = None;
                c.diagnostics.push(format!(
                    "{} node {}: referenced fixture '{}' not found",
                    c.fixture.name, n.id, reference
                ));
            }
        }
        "Spatial.Axis" => {
            let axis = (num("Axis", 0.) as usize).min(2);
            bank.value = Some(map(
                c.fixture.position[axis],
                num("Minimum", -5.),
                num("Maximum", 5.),
                0.,
                1.,
            ));
        }
        "Modify.Value.Invert" => bank.value = Some(1. - value),
        "Modify.Value.Clamp" => {
            let a = num("Minimum", 0.);
            let b = num("Maximum", 1.);
            bank.value = Some(value.clamp(a.min(b), a.max(b)))
        }
        "Modify.Value.MapTo" => {
            bank.value = Some(map(value, 0., 1., num("Minimum", 0.), num("Maximum", 1.)))
        }
        "Modify.Value.MapFromTo" => {
            bank.value = Some(map(
                value,
                num("Source Min", 0.),
                num("Source Max", 1.),
                num("Minimum", 0.),
                num("Maximum", 1.),
            ))
        }
        "Modify.Value.Steps" => {
            let steps = (num("Count", 4.) - 1.).max(1.);
            bank.value = Some((value * steps).round() / steps)
        }
        "Modify.Value.Integrate" => {
            if props.get("Reset").and_then(Value::as_bool).unwrap_or(false) {
                state.accumulator = 0.;
            } else {
                state.accumulator += value;
                state.accumulator = if props
                    .get("Wrap/Clamp")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                {
                    state.accumulator.clamp(0., 1.)
                } else {
                    state.accumulator.rem_euclid(1.)
                };
            }
            bank.value = Some(state.accumulator)
        }
        "Modify.SampleAndHold" => {
            if props
                .get("Sample")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                state.held = bank.clone()
            }
            bank = state.held.clone()
        }
        "Filter.Manual" => {
            if !props.get("Open").and_then(Value::as_bool).unwrap_or(true) {
                bank = Bank::default()
            }
        }
        "Convert.Value.ToColor" => bank.colors = Some(vec![[value, value, value, 1.]]),
        "Modify.Color.Invert" => {
            if let Some(colors) = bank.colors.as_mut() {
                for color in colors {
                    for channel in color.iter_mut().take(3) {
                        *channel += (1. - 2. * *channel) * num("Value", 0.)
                    }
                }
            }
        }
        "Modify.Color.Dim" => {
            let amount = if n
                .properties
                .get("Value")
                .and_then(scenic::number)
                .is_some_and(|v| v != 1.)
                || n.drivers.contains_key("Value")
            {
                num("Value", 1.)
            } else {
                num("Amount", 1.)
            };
            if let Some(colors) = bank.colors.as_mut() {
                for color in colors {
                    for channel in color.iter_mut().take(3) {
                        *channel *= amount.clamp(0., 1.);
                    }
                }
            }
        }
        "Combine.Add" | "Combine.Multiply" => {
            if let Some(first) = inputs.first() {
                bank = first.clone();
                for next in inputs.iter().skip(1) {
                    if let Some(a) = bank.value.as_mut() {
                        let b =
                            next.value
                                .unwrap_or(if n.behavior == "Combine.Add" { 0. } else { 1. });
                        *a = if n.behavior == "Combine.Add" {
                            *a + b
                        } else {
                            *a * b
                        }
                    }
                    if let (Some(a), Some(b)) = (bank.colors.as_mut(), next.colors.as_ref()) {
                        a.truncate(a.len().min(b.len()));
                        for (a, b) in a.iter_mut().zip(b) {
                            for j in 0..4 {
                                a[j] = if n.behavior == "Combine.Add" {
                                    a[j] + b[j]
                                } else {
                                    a[j] * b[j]
                                }
                            }
                        }
                    }
                }
            }
        }
        "Tool.Merge" => {}
        "Combine.Crossfade" => {
            if inputs.len() >= 2 {
                let scaled = num("Fader", 0.).clamp(0., 1.) * (inputs.len() - 1) as f64;
                let index = (scaled.floor() as usize).min(inputs.len() - 2);
                let a = &inputs[index];
                let b = &inputs[index + 1];
                let f = scaled - index as f64;
                let f = crate::native::timed::ease(f, num("Shape", 0.) as usize, 0);
                bank = crate::native::actions::blend(a, b, f);
            }
        }
        "Script.Value" | "Create.Value.Expression" => {
            let code = props[if n.behavior == "Script.Value" {
                "Code"
            } else {
                "Expression"
            }]
            .as_str()
            .unwrap_or("input");
            let mut scope = rhai::Scope::new();
            scope
                .push("input", value)
                .push("s", c.time)
                .push("t", c.time)
                .push("x", c.fixture.position[0])
                .push("y", c.fixture.position[1])
                .push("z", c.fixture.position[2])
                .push("index", c.index as f64)
                .push("count", c.fixtures.len() as f64);
            if let Some(parameters) = c.parameters.last() {
                let mut map = rhai::Map::new();
                for (key, value) in parameters {
                    let dynamic = if let Some(v) = value.as_f64() {
                        rhai::Dynamic::from_float(v)
                    } else if let Some(v) = value.as_bool() {
                        rhai::Dynamic::from_bool(v)
                    } else if let Some(v) = value.as_str() {
                        rhai::Dynamic::from(v.to_owned())
                    } else {
                        rhai::Dynamic::from_array(
                            value
                                .as_array()
                                .unwrap_or(&vec![])
                                .iter()
                                .map(|v| rhai::Dynamic::from_float(v.as_f64().unwrap_or(0.)))
                                .collect(),
                        )
                    };
                    map.insert(key.clone().into(), dynamic);
                }
                scope.push("params", map);
            }
            let output = c
                .scripts
                .get(code)
                .ok_or_else(|| "Script not compiled".to_owned())
                .and_then(|ast| {
                    c.script_engine
                        .eval_ast_with_scope::<rhai::Dynamic>(&mut scope, ast)
                        .map_err(|e| e.to_string())
                })
                .and_then(|v| {
                    v.as_float()
                        .or_else(|_| v.as_int().map(|v| v as f64))
                        .map_err(|_| "Script must return a number".into())
                });
            match output {
                Ok(v) if v.is_finite() => {
                    bank.value = Some(v);
                    state.native.output = v;
                }
                _ => {
                    c.diagnostics.push(format!(
                        "{} node {}: script failed or exceeded its budget",
                        c.fixture.name, n.id
                    ));
                    if n.behavior == "Create.Value.Expression" {
                        bank.value = Some(state.native.output);
                    } else {
                        bank = Bank::default();
                    }
                }
            }
        }
        _ if c.plugins.component(&n.behavior).is_some() => {
            let component = c.plugins.component(&n.behavior).cloned().unwrap();
            if !component.enabled {
                c.diagnostics
                    .push(format!("Component {} is unloaded", n.behavior));
                bank = Bank::default();
            } else {
                let next_prefix = format!("{}/{}@plugin/{}", c.prefix, n.id, component.key);
                let prefix = std::mem::replace(&mut c.prefix, next_prefix);
                c.parameters.push(props.clone());
                c.plugin_inputs.push(bank);
                c.bindings.push(component.bindings.clone());
                bank = evaluate(&component.root, c);
                c.bindings.pop();
                c.plugin_inputs.pop();
                c.parameters.pop();
                c.prefix = prefix;
            }
        }
        _ => {
            let mut native_context = crate::native::providers::Context {
                fixture: c.fixture,
                fixtures: c.fixtures,
                positions: c.live_positions,
                live_layers: c.live_layers,
                host: c.host,
                prior: c.prior,
                time: c.time,
                frame: c.frame,
                diagnostics: c.diagnostics,
            };
            if !crate::native::actions::evaluate(
                n,
                &props,
                &inputs,
                &mut bank,
                &mut state.native,
                &mut native_context,
                c.layer,
            ) && !crate::native::spatial::evaluate(
                n,
                &props,
                &inputs,
                &mut bank,
                &mut state.native,
                &mut native_context,
            ) && !crate::native::providers::evaluate(
                n,
                &props,
                &inputs,
                &mut bank,
                &mut state.native,
                &mut native_context,
            ) && !crate::native::pixels::evaluate(
                &n.behavior,
                &props,
                &inputs,
                &mut bank,
                c.fixture,
            ) && !crate::native::timed::evaluate(
                &n.behavior,
                &props,
                &inputs,
                &mut bank,
                &mut state.native,
                c.time,
            ) && !crate::native::evaluate(&n.behavior, &props, &inputs, &mut bank, c.fixture)
            {
                c.diagnostics
                    .push(format!("Unsupported native behavior {}", n.behavior));
            }
        }
    }
    // Preserve selected-export driver timing: owner update precedes driver writes.
    for (key, driver) in &n.drivers {
        let result = driver_value(n, key, evaluate(driver, c).value.unwrap_or(0.));
        let default = scenic::defaults(&n.behavior)
            .ok()
            .and_then(|m| m.get(key).cloned())
            .unwrap_or(json!(0.));
        state.driven.insert(
            key.clone(),
            if default.is_boolean() {
                json!(result > 0.5)
            } else if default.as_i64().is_some() {
                json!(result.round())
            } else {
                json!(result)
            },
        );
    }
    if bank
        .emitters
        .iter()
        .any(|e| !e.intensity.is_finite() || e.color.iter().any(|v| !v.is_finite()))
        || bank
            .position
            .is_some_and(|p| p.iter().any(|v| !v.is_finite()))
        || bank.value.is_some_and(|v| !v.is_finite())
        || bank
            .colors
            .as_ref()
            .is_some_and(|v| v.iter().flatten().any(|v| !v.is_finite()))
    {
        c.diagnostics.push(format!(
            "{} node {}: non-finite result",
            c.fixture.name, n.id
        ));
        bank = Bank::default();
    }
    if !c.prefix.contains("@plugin/") {
        c.trace.insert(
            format!("{}/{}", c.prefix.rsplit('/').next().unwrap_or("0"), n.id),
            bank.clone(),
        );
    }
    if let Some(incoming) = incoming {
        c.capture.record(
            DebugFocus {
                fixture: c.fixture.id.clone(),
                layer: c.layer,
                node: n.id,
            },
            incoming,
            &bank,
            false,
        );
    }
    c.nodes.insert(id, state);
    bank
}
fn driver_value(n: &Node, key: &str, value: f64) -> f64 {
    let extended = n.behavior.starts_with("Light.")
        || n.behavior.starts_with("Create.Position.")
        || matches!(
            n.behavior.as_str(),
            "Create.Value.Bezier" | "Create.Value.Random" | "Modify.Value.RandomizeTrigger"
        );
    if !value.is_finite() {
        return 0.;
    }
    if !extended {
        return value.clamp(0., 1.);
    }
    match key {
        "Type" | "Axis" => value.clamp(0., 2.),
        "Range" | "Width" | "Height" | "Duration" | "Interval" => value.clamp(0.001, 1e9),
        "Intensity" | "Gain" | "Falloff" | "Filter Time" | "Padding" => value.clamp(0., 1e9),
        "Angle" => value.clamp(0.001, 179.999),
        "Speed" if n.behavior == "Create.Audio.Sample" => value,
        "Probability" | "Randomness" | "Softness" | "Speed" => value.clamp(0., 1.),
        "Progress" => value.clamp(-1., 1.),
        _ => value.clamp(-1e9, 1e9),
    }
}
fn map(v: f64, a: f64, b: f64, c: f64, d: f64) -> f64 {
    if (b - a).abs() < f64::EPSILON {
        c
    } else {
        c + ((v - a) / (b - a)) * (d - c)
    }
}
fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    a.iter()
        .zip(b)
        .map(|(a, b)| (a - b).powi(2))
        .sum::<f64>()
        .sqrt()
}
pub fn encode_fixture(f: &Fixture, b: &Bank, blackout: bool) -> Vec<u8> {
    let count = f.pixel_positions.len().clamp(1, 512);
    let mut result = Vec::with_capacity(count * f.model.channels());
    for pixel in 0..count {
        let color = b
            .colors
            .as_ref()
            .filter(|colors| !colors.is_empty())
            .map(|colors| colors[pixel % colors.len()])
            .unwrap_or_else(|| {
                let v = b.value.unwrap_or(0.);
                [v, v, v, 1.]
            });
        let scale = if blackout {
            0.
        } else {
            f.dimmer * color[3].clamp(0., 1.)
        };
        let byte = |v: f64| (v.clamp(0., 1.) * scale * 255.).round() as u8;
        match f.model {
            FixtureModel::Dimmer => result.push(byte(
                b.value.unwrap_or((color[0] + color[1] + color[2]) / 3.),
            )),
            FixtureModel::Rgb => result.extend([byte(color[0]), byte(color[1]), byte(color[2])]),
            FixtureModel::Rgbw => {
                let white = color[0].min(color[1]).min(color[2]);
                result.extend([
                    byte(color[0] - white),
                    byte(color[1] - white),
                    byte(color[2] - white),
                    byte(white),
                ]);
            }
        }
    }
    result
}

fn debug_node<'a>(p: &'a Project, t: &DebugFocus) -> Option<&'a Node> {
    (if t.fixture == "__director" {
        &p.director
    } else {
        p.content.get(&t.fixture)?
    })
    .get(t.layer)?
    .as_ref()?
    .root
    .find(t.node)
}
