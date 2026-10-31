pub use crate::runtime::Published;
use crate::{
    engine::{Command, Engine, PreparedEdit},
    model::*,
    project, scenic,
};
use axum::{
    extract::{DefaultBodyLimit, Path, Request, State},
    http::{header, StatusCode},
    middleware::{self, Next},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, SyncSender},
        Arc, RwLock,
    },
    time::{Duration, Instant},
};

enum RequestMessage {
    DimmingClock(
        crate::dimming_clock::Control,
        tokio::sync::oneshot::Sender<Result<crate::dimming_clock::Status>>,
    ),
    Inspection(
        Vec<DebugFocus>,
        tokio::sync::oneshot::Sender<Result<crate::inspection::Frame>>,
    ),
    Bundle(
        Box<crate::engine::PreparedBundle>,
        tokio::sync::oneshot::Sender<Result<u64>>,
    ),
    Edit(Box<Message>),
    Plugins(
        Box<crate::engine::PreparedPluginUpdate>,
        tokio::sync::oneshot::Sender<Result<u64>>,
    ),
    Checkpoint(tokio::sync::oneshot::Sender<crate::session::Session>),
}
struct Message {
    command: Command,
    prepared: Option<PreparedEdit>,
    expected: Option<u64>,
    plugin_generation: u64,
    reply: tokio::sync::oneshot::Sender<Result<u64>>,
}
#[derive(Clone)]
pub struct Controller {
    published: Arc<RwLock<Arc<Published>>>,
    logs: Arc<std::sync::Mutex<crate::logs::Journal>>,
    commands: SyncSender<RequestMessage>,
    session_store: Option<Arc<crate::session::Store>>,
}
impl Controller {
    pub fn current(&self) -> Arc<Published> {
        self.published
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
    pub fn current_plugins(&self) -> Arc<crate::plugins::Registry> {
        self.current().plugins.clone()
    }
    pub fn plugin_catalog(&self) -> Vec<serde_json::Value> {
        let mut rows = scenic::catalog()
            .iter()
            .map(|b| serde_json::to_value(b).expect("catalog row"))
            .collect::<Vec<_>>();
        rows.extend(self.current_plugins().catalog());
        rows
    }
    pub async fn replace_plugins(&self, plugins: Arc<crate::plugins::Registry>) -> Result<u64> {
        let session = self.checkpoint().await?;
        let update =
            tokio::task::spawn_blocking(move || Engine::prepare_plugin_update(session, plugins))
                .await
                .map_err(|e| e.to_string())??;
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.commands
            .try_send(RequestMessage::Plugins(Box::new(update), tx))
            .map_err(|_| "Engine command queue full or stopped")?;
        tokio::time::timeout(Duration::from_secs(3), rx)
            .await
            .map_err(|_| "Plugin commit timed out")?
            .map_err(|_| "Engine stopped".to_owned())?
    }
    pub async fn dimming_clock(
        &self,
        control: crate::dimming_clock::Control,
    ) -> Result<crate::dimming_clock::Status> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.commands
            .try_send(RequestMessage::DimmingClock(control, tx))
            .map_err(|_| "Engine queue full or stopped")?;
        tokio::time::timeout(Duration::from_secs(3), rx)
            .await
            .map_err(|_| "Dimming clock timed out")?
            .map_err(|_| "Engine stopped")?
    }
    pub async fn inspection(&self, targets: Vec<DebugFocus>) -> Result<crate::inspection::Frame> {
        if targets.len() > crate::inspection::MAX_TARGETS {
            return Err("At most 256 inspected blocks".into());
        }
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.commands
            .try_send(RequestMessage::Inspection(targets, tx))
            .map_err(|_| "Engine queue full or stopped")?;
        tokio::time::timeout(Duration::from_secs(3), rx)
            .await
            .map_err(|_| "Inspection timed out")?
            .map_err(|_| "Engine stopped")?
    }
    pub async fn checkpoint(&self) -> Result<crate::session::Session> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.commands
            .try_send(RequestMessage::Checkpoint(tx))
            .map_err(|_| "Engine command queue is full or stopped")?;
        tokio::time::timeout(Duration::from_secs(3), rx)
            .await
            .map_err(|_| "Session checkpoint timed out")?
            .map_err(|_| "Engine stopped".into())
    }
    pub fn session_status(&self) -> crate::session::Status {
        self.session_store
            .as_ref()
            .map(|s| s.status())
            .unwrap_or_default()
    }
    pub async fn persist_session(&self) -> Result<u64> {
        let store = self
            .session_store
            .clone()
            .ok_or("Session persistence is disabled for this runtime")?;
        let session = self.checkpoint().await?;
        let revision = session.snapshot.revision;
        tokio::task::spawn_blocking(move || store.save(&session))
            .await
            .map_err(|e| e.to_string())??;
        Ok(revision)
    }
    pub async fn import_bundle(
        &self,
        bundle: crate::bundle::Bundle,
        expected: Option<u64>,
    ) -> Result<u64> {
        let session = self.checkpoint().await?;
        if expected.is_some_and(|r| r != session.snapshot.revision) {
            return Err("Revision conflict: refresh state before editing".into());
        }
        let prepared = tokio::task::spawn_blocking(move || Engine::prepare_bundle(session, bundle))
            .await
            .map_err(|e| e.to_string())??;
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.commands
            .try_send(RequestMessage::Bundle(Box::new(prepared), tx))
            .map_err(|_| "Engine command queue is full")?;
        tokio::time::timeout(Duration::from_secs(3), rx)
            .await
            .map_err(|_| "Bundle commit timed out")?
            .map_err(|_| "Engine stopped")?
    }
    pub async fn import_library(
        &self,
        item: crate::library::Item,
        expected: Option<u64>,
    ) -> Result<u64> {
        let session = self.checkpoint().await?;
        if expected.is_some_and(|r| r != session.snapshot.revision) {
            return Err("Revision conflict: refresh state before editing".into());
        }
        let prepared = tokio::task::spawn_blocking(move || Engine::prepare_library(session, item))
            .await
            .map_err(|e| e.to_string())??;
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.commands
            .try_send(RequestMessage::Bundle(Box::new(prepared), tx))
            .map_err(|_| "Engine command queue is full")?;
        tokio::time::timeout(Duration::from_secs(3), rx)
            .await
            .map_err(|_| "Bundle commit timed out")?
            .map_err(|_| "Engine stopped")?
    }
    pub async fn command(&self, command: Command, expected: Option<u64>) -> Result<u64> {
        self.dispatch(command, expected, true).await
    }
    async fn dispatch(
        &self,
        mut command: Command,
        expected: Option<u64>,
        history: bool,
    ) -> Result<u64> {
        let frame = self.current();
        if let Command::RestoreCheckpoint { name } = &command {
            let session = self.checkpoint().await?;
            let project = session
                .checkpoints
                .get(name)
                .ok_or("Checkpoint not found")?
                .as_ref()
                .clone();
            command = Command::ReplaceProject {
                project: Box::new(project),
            };
        }
        if let Command::Input { event } = &command {
            let event = event.clone();
            let plugins = frame.plugins.clone();
            let prepared = tokio::task::spawn_blocking(move || {
                scenic::with_plugins(plugins, || event.prepared())
            })
            .await
            .map_err(|e| e.to_string())??;
            command = Command::PreparedInput { event: prepared };
        }
        let expected = if matches!(command, Command::PreparedInput { .. }) {
            None
        } else {
            Some(expected.unwrap_or(frame.snapshot.revision))
        };
        if expected.is_some_and(|revision| revision != frame.snapshot.revision) {
            return Err("Revision conflict: refresh state before editing".into());
        }
        let plugin_generation = frame.plugins.state.generation;
        let prepared = if matches!(
            command,
            Command::Undo
                | Command::HistoryJump { .. }
                | Command::SaveCheckpoint { .. }
                | Command::RemoveCheckpoint { .. }
                | Command::SetDebugFocus { .. }
                | Command::Redo
                | Command::Pause { .. }
                | Command::Blackout { .. }
                | Command::Input { .. }
                | Command::PreparedInput { .. }
        ) {
            None
        } else {
            let edit = command.clone();
            Some(
                tokio::task::spawn_blocking(move || {
                    scenic::with_plugins(frame.plugins.clone(), || {
                        if history {
                            Engine::prepare(frame.project.clone(), edit)
                        } else {
                            Engine::prepare_internal(frame.project.clone(), edit)
                        }
                    })
                })
                .await
                .map_err(|e| e.to_string())??,
            )
        };
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.commands
            .try_send(RequestMessage::Edit(Box::new(Message {
                command,
                prepared,
                expected,
                plugin_generation,
                reply: tx,
            })))
            .map_err(|_| "Engine command queue is full or stopped")?;
        tokio::time::timeout(Duration::from_secs(3), rx)
            .await
            .map_err(|_| "Command acknowledgment timed out")?
            .map_err(|_| "Engine stopped")?
    }
}
pub struct Runtime {
    pub controller: Controller,
    stop: Arc<AtomicBool>,
    threads: Vec<std::thread::JoinHandle<()>>,
    persistence_stop: Arc<AtomicBool>,
    persistence_worker: Option<std::thread::JoinHandle<()>>,
}
impl Runtime {
    pub fn start(project: Project) -> Result<Self> {
        Self::start_resumed(project, None)
    }
    pub fn start_resumed(project: Project, resume: Option<Snapshot>) -> Result<Self> {
        let mut engine = Engine::new(project)?;
        if let Some(snapshot) = resume {
            engine.resume_snapshot(snapshot)?;
        }
        Self::start_engine(engine, None)
    }
    pub fn start_engine(engine: Engine, session_path: Option<PathBuf>) -> Result<Self> {
        Self::start_engine_guarded(engine, session_path, None)
    }
    /// A startup-only barrier lets the installer verify restored history before runtime actions resume.
    pub fn start_engine_guarded(
        mut engine: Engine,
        session_path: Option<PathBuf>,
        barrier: Option<PathBuf>,
    ) -> Result<Self> {
        let mut updating = barrier.as_ref().is_some_and(|path| path.exists());
        if !updating {
            engine.tick(0.);
        }
        let session_store = session_path.map(|p| Arc::new(crate::session::Store::new(p)));
        let latest_session = Arc::new(RwLock::new(Arc::new(engine.checkpoint())));
        let published = Arc::new(RwLock::new(Arc::new(Published {
            host: engine.host(),
            resources: engine.resources(),
            plugins: engine.plugins(),
            dimming_clock: engine.dimming_status(),
            project: engine.project().clone(),
            snapshot: engine.snapshot(),
        })));
        let (tx, rx) = mpsc::sync_channel::<RequestMessage>(64);
        let stop = Arc::new(AtomicBool::new(false));
        let logs = Arc::new(std::sync::Mutex::new(crate::logs::Journal::new()));
        logs.lock()
            .unwrap_or_else(|e| e.into_inner())
            .observe(&engine.snapshot());
        let evaluation_logs = logs.clone();
        let controller = Controller {
            logs,
            published: published.clone(),
            commands: tx,
            session_store: session_store.clone(),
        };
        let (action_tx, action_rx) = mpsc::sync_channel::<Arc<Published>>(64);
        let stopping = stop.clone();
        let state = published.clone();
        let status = Arc::new(RwLock::new("Disabled — capture only".to_owned()));
        let engine_status = status.clone();
        let latest_checkpoint = latest_session.clone();
        let persistent = session_store.is_some();
        let evaluate = std::thread::Builder::new()
            .name("schema-evaluation".into())
            .spawn(move || {
                let interval = Duration::from_secs_f64(1. / 60.);
                let mut previous = Instant::now();
                let mut deadline = previous;
                let mut checkpoint_revision = engine.revision();
                let mut checkpoint_plugins = engine.plugins().state.generation;
                let mut checkpoint_controls =
                    (engine.paused_for_session(), engine.blackout_for_session());
                let mut checkpoint_time = Instant::now();
                while !stopping.load(Ordering::Relaxed) {
                    if updating {
                        updating = barrier.as_ref().is_some_and(|path| path.exists());
                    }
                    let mut checkpoint_replies = Vec::new();
                    let mut replies = Vec::new();
                    for _ in 0..4 {
                        let Ok(request) = rx.try_recv() else { break };
                        let message = match request {
                            RequestMessage::Bundle(prepared, reply) => {
                                let result = if updating {
                                    Err("Engine updating; retry shortly".into())
                                } else {
                                    engine.install_bundle(*prepared)
                                };
                                replies.push((reply, result));
                                continue;
                            }
                            RequestMessage::Inspection(targets, reply) => {
                                let _ = reply.send(engine.request_inspection(targets));
                                continue;
                            }
                            RequestMessage::DimmingClock(control, reply) => {
                                let _ = reply.send(if updating {
                                    Err("Engine updating; retry shortly".into())
                                } else {
                                    engine.set_dimming_clock(control)
                                });
                                continue;
                            }
                            RequestMessage::Checkpoint(reply) => {
                                checkpoint_replies.push(reply);
                                continue;
                            }
                            RequestMessage::Plugins(update, reply) => {
                                let result = if updating {
                                    Err("Engine updating; retry shortly".into())
                                } else {
                                    engine.install_plugin_update(*update)
                                };
                                replies.push((reply, result));
                                continue;
                            }
                            RequestMessage::Edit(message) => *message,
                        };
                        let result = if updating {
                            Err("Engine updating; retry shortly".into())
                        } else if message.plugin_generation != engine.plugins().state.generation {
                            Err("Revision conflict: plugin registry changed; retry the edit".into())
                        } else if message.expected.is_some_and(|r| r != engine.revision()) {
                            Err("Revision conflict: refresh state before editing".into())
                        } else if let Some(prepared) = message.prepared {
                            engine.install(prepared).map(|_| engine.revision())
                        } else {
                            engine.apply(message.command).map(|_| engine.revision())
                        };
                        replies.push((message.reply, result));
                    }
                    let now = Instant::now();
                    let dt = now.duration_since(previous).as_secs_f64();
                    previous = now;
                    if !updating {
                        engine.tick(dt);
                    }
                    let mut snapshot = engine.snapshot();
                    snapshot.output_status = engine_status
                        .read()
                        .unwrap_or_else(|e| e.into_inner())
                        .clone();
                    evaluation_logs
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .observe(&snapshot);
                    let next = Arc::new(Published {
                        host: engine.host(),
                        resources: engine.resources(),
                        plugins: engine.plugins(),
                        dimming_clock: engine.dimming_status(),
                        project: engine.project().clone(),
                        snapshot,
                    });
                    if !updating && !next.snapshot.actions.is_empty() {
                        let _ = action_tx.try_send(next.clone());
                    }
                    *state.write().unwrap_or_else(|e| e.into_inner()) = next;
                    let controls = (engine.paused_for_session(), engine.blackout_for_session());
                    if !checkpoint_replies.is_empty()
                        || persistent
                            && (engine.plugins().state.generation != checkpoint_plugins
                                || engine.revision() != checkpoint_revision
                                || controls != checkpoint_controls
                                || checkpoint_time.elapsed() >= Duration::from_secs(2))
                    {
                        let checkpoint = Arc::new(engine.checkpoint());
                        checkpoint_revision = engine.revision();
                        checkpoint_plugins = engine.plugins().state.generation;
                        checkpoint_controls = controls;
                        checkpoint_time = Instant::now();
                        *latest_checkpoint.write().unwrap_or_else(|e| e.into_inner()) =
                            checkpoint.clone();
                        for reply in checkpoint_replies {
                            let _ = reply.send((*checkpoint).clone());
                        }
                    }
                    for (reply, result) in replies {
                        let _ = reply.send(result);
                    }
                    deadline += interval;
                    let now = Instant::now();
                    if deadline > now {
                        std::thread::sleep(deadline - now)
                    } else {
                        deadline = now;
                    }
                }
                *latest_checkpoint.write().unwrap_or_else(|e| e.into_inner()) =
                    Arc::new(engine.checkpoint());
            })
            .map_err(|e| e.to_string())?;
        let stopping_actions = stop.clone();
        let action_controller = controller.clone();
        let actions = std::thread::Builder::new()
            .name("schema-actions".into())
            .spawn(move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("action runtime");
                let mut last_error = String::new();
                while !stopping_actions.load(Ordering::Relaxed) {
                    let Ok(frame) = action_rx.recv_timeout(Duration::from_millis(30)) else {
                        continue;
                    };
                    // Resolve captured pulses against the latest committed project and provider state.
                    let current = action_controller.current();
                    let mut captured = (*current).clone();
                    captured.snapshot.actions = frame.snapshot.actions.clone();
                    let resolved = crate::native::actions::resolve(&captured);
                    let mut errors = resolved.errors;
                    runtime.block_on(async {
                        for event in resolved.events {
                            if let Err(error) = action_controller
                                .command(Command::Input { event }, None)
                                .await
                            {
                                errors.push(error);
                            }
                        }
                        for (commands, history) in
                            [(resolved.commands, true), (resolved.internal, false)]
                        {
                            for command in commands {
                                if stopping_actions.load(Ordering::Relaxed) {
                                    break;
                                }
                                if let Err(error) =
                                    action_controller.dispatch(command, None, history).await
                                {
                                    errors.push(error);
                                }
                            }
                        }
                    });
                    let error = errors.join("; ");
                    if !error.is_empty() && error != last_error {
                        action_controller
                            .logs
                            .lock()
                            .unwrap_or_else(|e| e.into_inner())
                            .record("error", "actions", &error);
                        eprintln!("Node action: {error}");
                    }
                    if error.is_empty() && !last_error.is_empty() {
                        action_controller
                            .logs
                            .lock()
                            .unwrap_or_else(|e| e.into_inner())
                            .record("info", "actions", "Node action error cleared");
                    }
                    last_error = error;
                }
            })
            .map_err(|e| e.to_string())?;
        let stopping = stop.clone();
        let state = published;
        let output_status = status;
        let output = std::thread::Builder::new()
            .name("schema-output".into())
            .spawn(move || {
                let mut artnet = None;
                let mut usb: Option<crate::open_dmx::Sender> = None;
                let mut device = String::new();
                let mut retry = Instant::now();
                let mut last_error = String::new();
                while !stopping.load(Ordering::Relaxed) {
                    let started = Instant::now();
                    let frame = state.read().unwrap_or_else(|e| e.into_inner()).clone();
                    let config = &frame.project.output;
                    let mut message = if !config.enabled {
                        usb = None;
                        device.clear();
                        "Disabled — capture only".into()
                    } else if config.transport == OutputTransport::OpenDmx {
                        if device != config.device {
                            usb = None;
                            device = config.device.clone();
                            retry = Instant::now();
                        }
                        let attempted = usb.is_some() || Instant::now() >= retry;
                        let result = if !attempted {
                            Err(last_error.clone())
                        } else {
                            (if usb.is_none() {
                                crate::open_dmx::Sender::open(&device).map(|s| usb = Some(s))
                            } else {
                                Ok(())
                            })
                            .and_then(|_| {
                                usb.as_mut().unwrap().send(
                                    frame
                                        .snapshot
                                        .universes
                                        .get(&config.universe)
                                        .map(Vec::as_slice)
                                        .unwrap_or(&[]),
                                )
                            })
                        };
                        match result {
                            Ok(()) => format!(
                                "Open DMX USB {} · U{} · {} FPS",
                                device, config.universe, config.fps
                            ),
                            Err(e) => {
                                usb = None;
                                last_error = e;
                                if attempted {
                                    retry = Instant::now() + Duration::from_secs(1);
                                }
                                format!("Output error: {last_error}")
                            }
                        }
                    } else {
                        usb = None;
                        device.clear();
                        (if artnet.is_none() {
                            crate::artnet::Sender::new().map(|s| artnet = Some(s))
                        } else {
                            Ok(())
                        })
                        .and_then(|_| {
                            artnet.as_mut().unwrap().send(
                                config
                                    .target
                                    .parse()
                                    .map_err(|_| "Invalid output target".to_string())?,
                                &frame.snapshot.universes,
                            )
                        })
                        .map(|_| format!("ArtDmx → {}", config.target))
                        .unwrap_or_else(|e| format!("Output error: {e}"))
                    };
                    if let Some(test) = &frame.snapshot.channel_test {
                        message.push_str(&format!(
                            " · TEST U{} · {} channels",
                            test.universe,
                            test.channels.len()
                        ));
                    }
                    *output_status.write().unwrap_or_else(|e| e.into_inner()) = message;
                    // Device I/O never blocks evaluation. Deadline includes USB frame transmission time.
                    std::thread::sleep(
                        Duration::from_secs_f64(1. / config.fps as f64)
                            .saturating_sub(started.elapsed()),
                    );
                }
            })
            .map_err(|e| e.to_string())?;
        let persistence_stop = Arc::new(AtomicBool::new(false));
        let persistence_logs = controller.logs.clone();
        let persistence_worker = if let Some(store) = session_store {
            let stopping = persistence_stop.clone();
            Some(
                std::thread::Builder::new()
                    .name("schema-session-writer".into())
                    .spawn(move || {
                        let mut saved: Option<Arc<crate::session::Session>> = None;
                        let mut last_error = String::new();
                        loop {
                            let checkpoint = latest_session
                                .read()
                                .unwrap_or_else(|e| e.into_inner())
                                .clone();
                            if saved
                                .as_ref()
                                .is_none_or(|old| !Arc::ptr_eq(old, &checkpoint))
                            {
                                match store.save(&checkpoint) {
                                    Ok(()) => {
                                        if !last_error.is_empty() {
                                            persistence_logs
                                                .lock()
                                                .unwrap_or_else(|e| e.into_inner())
                                                .record(
                                                    "info",
                                                    "storage",
                                                    "Session saving recovered",
                                                );
                                        }
                                        last_error.clear();
                                        saved = Some(checkpoint);
                                    }
                                    Err(error) => {
                                        if error != last_error {
                                            persistence_logs
                                                .lock()
                                                .unwrap_or_else(|e| e.into_inner())
                                                .record(
                                                    "error",
                                                    "storage",
                                                    &format!("Session save failed: {error}"),
                                                );
                                        }
                                        last_error = error;
                                    }
                                }
                            }
                            if stopping.load(Ordering::Relaxed) {
                                break;
                            }
                            std::thread::sleep(Duration::from_millis(100));
                        }
                    })
                    .map_err(|e| e.to_string())?,
            )
        } else {
            None
        };
        Ok(Self {
            controller,
            stop,
            threads: vec![evaluate, output, actions],
            persistence_stop,
            persistence_worker,
        })
    }
}
impl Drop for Runtime {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        for thread in self.threads.drain(..) {
            let _ = thread.join();
        }
        self.persistence_stop.store(true, Ordering::Relaxed);
        if let Some(worker) = self.persistence_worker.take() {
            let _ = worker.join();
        }
    }
}
pub fn valid_lan_token(token: &str) -> bool {
    is_access_pin(token) || token.len() >= 16
}
fn is_access_pin(token: &str) -> bool {
    token.len() == 4 && token.bytes().all(|byte| byte.is_ascii_digit())
}
#[derive(Default)]
struct PinAttempts {
    peers: std::collections::BTreeMap<std::net::IpAddr, std::collections::VecDeque<Instant>>,
    global: std::collections::VecDeque<Instant>,
}
impl PinAttempts {
    fn authorize(
        &mut self,
        peer: std::net::IpAddr,
        candidate: &str,
        expected: &str,
        now: Instant,
    ) -> StatusCode {
        self.peers.retain(|_, attempts| {
            while attempts
                .front()
                .is_some_and(|time| now.duration_since(*time) >= Duration::from_secs(60))
            {
                attempts.pop_front();
            }
            !attempts.is_empty()
        });
        while self
            .global
            .front()
            .is_some_and(|time| now.duration_since(*time) >= Duration::from_secs(60))
        {
            self.global.pop_front();
        }
        if self.global.len() >= 32
            || self
                .peers
                .get(&peer)
                .is_some_and(|attempts| attempts.len() >= 5)
        {
            return StatusCode::TOO_MANY_REQUESTS;
        }
        if candidate.len() == expected.len()
            && candidate
                .bytes()
                .zip(expected.bytes())
                .fold(0, |different, (a, b)| different | (a ^ b))
                == 0
        {
            self.peers.remove(&peer);
            return StatusCode::OK;
        }
        if !self.peers.contains_key(&peer) && self.peers.len() >= 256 {
            return StatusCode::TOO_MANY_REQUESTS;
        }
        self.peers.entry(peer).or_default().push_back(now);
        self.global.push_back(now);
        StatusCode::UNAUTHORIZED
    }
}
#[derive(Clone)]
pub struct AppState {
    pub plugins: Arc<crate::plugin_host::Manager>,
    pub controller: Controller,
    pub root: PathBuf,
    token: Option<Arc<String>>,
    pin_attempts: Arc<std::sync::Mutex<PinAttempts>>,
    pub(crate) io_lock: Arc<tokio::sync::Mutex<()>>,
}
impl AppState {
    pub fn start_plugin_watcher(&self) -> Result<crate::plugin_host::Watcher> {
        self.plugins.start_watcher()
    }
    pub fn new(controller: Controller, root: PathBuf, token: Option<String>) -> Self {
        Self {
            plugins: Arc::new(crate::plugin_host::Manager::new(
                controller.clone(),
                root.clone(),
            )),
            controller,
            root,
            token: token.map(Arc::new),
            pin_attempts: Arc::new(std::sync::Mutex::new(PinAttempts::default())),
            io_lock: Arc::new(tokio::sync::Mutex::new(())),
        }
    }
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    command: Command,
    expected_revision: Option<u64>,
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Import {
    fixture: String,
    layer: usize,
    yaml: String,
    expected_revision: Option<u64>,
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Save {
    name: String,
    expected_revision: Option<u64>,
    #[serde(default)]
    scope: SaveScope,
    #[serde(default)]
    fixtures: Vec<String>,
    #[serde(default)]
    layer: Option<usize>,
}
#[derive(Default, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum SaveScope {
    #[default]
    All,
    Layout,
    Content,
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct LoadContent {
    name: String,
    group: Option<String>,
    expected_revision: Option<u64>,
}
pub(crate) type ApiResult<T> = std::result::Result<T, ApiError>;
pub(crate) struct ApiError(pub(crate) String);
impl From<String> for ApiError {
    fn from(e: String) -> Self {
        Self(e)
    }
}
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status =
            if self.0.contains("Revision conflict") || self.0.contains("Project already exists") {
                StatusCode::CONFLICT
            } else if self.0.contains("queue") {
                StatusCode::SERVICE_UNAVAILABLE
            } else {
                StatusCode::BAD_REQUEST
            };
        (status, Json(serde_json::json!({"error":self.0}))).into_response()
    }
}
async fn guard(State(state): State<AppState>, request: Request, next: Next) -> Response {
    let headers = request.headers();
    let host = headers
        .get(header::HOST)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if state.token.is_none()
        && !host.is_empty()
        && !host.starts_with("127.0.0.1:")
        && !host.starts_with("localhost:")
        && !host.starts_with("[::1]:")
    {
        return StatusCode::FORBIDDEN.into_response();
    }
    if let Some(origin) = headers.get(header::ORIGIN).and_then(|v| v.to_str().ok()) {
        if origin != format!("http://{host}") && origin != format!("https://{host}") {
            return StatusCode::FORBIDDEN.into_response();
        }
    }
    if request.uri().path().starts_with("/api/") {
        if let Some(token) = &state.token {
            let candidate = headers
                .get("x-schema-token")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");
            if is_access_pin(token) {
                // Missing/stale long tokens cannot guess a four-digit key or lock out a client.
                if !is_access_pin(candidate) {
                    return StatusCode::UNAUTHORIZED.into_response();
                }
                let peer = request
                    .extensions()
                    .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
                    .map(|info| info.0.ip())
                    .unwrap_or(std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED));
                let status = match state.pin_attempts.lock() {
                    Ok(mut attempts) => attempts.authorize(peer, candidate, token, Instant::now()),
                    Err(_) => StatusCode::SERVICE_UNAVAILABLE,
                };
                if status == StatusCode::TOO_MANY_REQUESTS {
                    return (status, [(header::RETRY_AFTER, "60")]).into_response();
                }
                if status != StatusCode::OK {
                    return status.into_response();
                }
            } else if candidate != token.as_str() {
                return StatusCode::UNAUTHORIZED.into_response();
            }
        }
    }
    if request.uri().path().starts_with("/api/")
        && request.method() != axum::http::Method::GET
        && request.method() != axum::http::Method::HEAD
        && state.root.join(".update-in-progress").exists()
    {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"error":"Engine updating; retry shortly"})),
        )
            .into_response();
    }
    let preview_frame = request.uri().path() == "/preview";
    let mut response = next.run(request).await;
    response.headers_mut().insert(
        "x-content-type-options",
        header::HeaderValue::from_static("nosniff"),
    );
    let policy = if preview_frame {
        "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; connect-src 'self' data: blob:; img-src 'self' blob: data:; frame-ancestors 'self'; base-uri 'none'"
    } else {
        "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; connect-src 'self' data: blob:; img-src 'self' blob: data:; frame-ancestors 'none'; base-uri 'none'"
    };
    response.headers_mut().insert(
        "content-security-policy",
        header::HeaderValue::from_static(policy),
    );
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("no-store"),
    );
    response
}
pub fn router(state: AppState) -> Router {
    Router::new()
        .merge(crate::web_product::routes())
        .merge(crate::scene_preview::routes())
.route("/output-core.js",get(||async{ ([(header::CONTENT_TYPE,"text/javascript")],include_str!("../web/output-core.js")) }))
.route("/output-ui.js",get(||async{ ([(header::CONTENT_TYPE,"text/javascript")],include_str!("../web/output-ui.js")) }))
.route("/output-design.css",get(||async{ ([(header::CONTENT_TYPE,"text/css")],include_str!("../web/output-design.css")) }))
        .route("/library-ui.js",get(||async{ ([(header::CONTENT_TYPE,"text/javascript; charset=utf-8")],include_str!("../web/library-ui.js")) }))
        .route("/library.css",get(||async{ ([(header::CONTENT_TYPE,"text/css; charset=utf-8")],include_str!("../web/library.css")) }))
        .route("/projects-ui.js",get(||async{ ([(header::CONTENT_TYPE,"text/javascript")],include_str!("../web/projects-ui.js")) }))
        .route("/bezier-editor.js",get(||async{ ([(header::CONTENT_TYPE,"text/javascript")],include_str!("../web/bezier-editor.js")) }))
        .route("/bezier-editor.css",get(||async{ ([(header::CONTENT_TYPE,"text/css")],include_str!("../web/bezier-editor.css")) }))
        .route("/workspace-core.js",get(||async{ ([(header::CONTENT_TYPE,"text/javascript")],include_str!("../web/workspace-core.js")) }))
        .route("/workspace-ui.js",get(||async{ ([(header::CONTENT_TYPE,"text/javascript")],include_str!("../web/workspace-ui.js")) }))
        .route("/studio-panels.js",get(||async{ ([(header::CONTENT_TYPE,"text/javascript")],include_str!("../web/studio-panels.js")) }))
        .route("/workspace.css",get(||async{ ([(header::CONTENT_TYPE,"text/css")],include_str!("../web/workspace.css")) }))
        .route("/fold-motion.js",get(||async{ ([(header::CONTENT_TYPE,"text/javascript")],include_str!("../web/fold-motion.js")) }))
        .route("/stack-layout.js",get(||async{ ([(header::CONTENT_TYPE,"text/javascript")],include_str!("../web/stack-layout.js")) }))
        .route("/stack-editor.css",get(||async{ ([(header::CONTENT_TYPE,"text/css")],include_str!("../web/stack-editor.css")) }))
        .route(
            "/",
            get(|| async { Html(include_str!("../web/index.html")) }),
        )
        .route(
            "/app.js",
            get(|| async {
                (
                    [(axum::http::header::CONTENT_TYPE, "text/javascript")],
                    include_str!("../web/app.js"),
                )
            }),
        )
        .route(
            "/style.css",
            get(|| async {
                (
                    [(axum::http::header::CONTENT_TYPE, "text/css")],
                    include_str!("../web/style.css"),
                )
            }),
        )
        .route(
            "/catalog-browser.js",
            get(|| async {
                (
                    [(
                        axum::http::header::CONTENT_TYPE,
                        "text/javascript; charset=utf-8",
                    )],
                    include_str!("../web/catalog-browser.js"),
                )
            }),
        )
        .route(
            "/block-metadata.json",
            get(|| async {
                (
                    [(axum::http::header::CONTENT_TYPE, "application/json")],
                    include_str!("../web/block-metadata.json"),
                )
            }),
        )
        .route(
            "/icons/{name}",
            get(|Path(name): Path<String>| async move {
                crate::block_icons::get(&name)
                    .map(|svg| ([(axum::http::header::CONTENT_TYPE, "image/svg+xml")], svg))
                    .ok_or(StatusCode::NOT_FOUND)
            }),
        )
        .route(
            "/favicon.ico",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "image/x-icon")],
                    include_bytes!("../web/favicon.ico").as_slice(),
                )
            }),
        )
        .route(
            "/inputs.js",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
                    include_str!("../web/inputs.js"),
                )
            }),
        )
        .route(
            "/mobile",
            get(|| async { Html(include_str!("../web/mobile.html")) }),
        )
        .route(
            "/mobile.html",
            get(|| async { Html(include_str!("../web/mobile.html")) }),
        )
        .route(
            "/mobile.css",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
                    include_str!("../web/mobile.css"),
                )
            }),
        )
        .route(
            "/mobile.js",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
                    include_str!("../web/mobile.js"),
                )
            }),
        )
        .route(
            "/mobile-skin-core.js",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
                    include_str!("../web/mobile-skin-core.js"),
                )
            }),
        )
        .route(
            "/mobile-skin.js",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
                    include_str!("../web/mobile-skin.js"),
                )
            }),
        )
        .route(
            "/mobile-skin.css",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
                    include_str!("../web/mobile-skin.css"),
                )
            }),
        )
        .route(
            "/schematic-camera.js",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
                    include_str!("../web/schematic-camera.js"),
                )
            }),
        )
        .route(
            "/schematic-camera.css",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
                    include_str!("../web/schematic-camera.css"),
                )
            }),
        )
        .route(
            "/mobile-core.js",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
                    include_str!("../web/mobile-core.js"),
                )
            }),
        )
        .route(
            "/view-variant.js",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
                    include_str!("../web/view-variant.js"),
                )
            }),
        )
        .route(
            "/engine-client.js",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
                    include_str!("../web/engine-client.js"),
                )
            }),
        )
        .route(
            "/block-edit.js",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
                    include_str!("../web/block-edit.js"),
                )
            }),
        )
        .route(
            "/block-ui.js",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
                    include_str!("../web/block-ui.js"),
                )
            }),
        )
        .route("/block-picker.js", get(|| async { ([(header::CONTENT_TYPE, "text/javascript; charset=utf-8")], include_str!("../web/block-picker.js")) }))
        .route("/fixture-placement.js", get(|| async { ([(header::CONTENT_TYPE,"text/javascript; charset=utf-8")], include_str!("../web/fixture-placement.js")) }))
        .route("/fixture-placement.css", get(|| async { ([(header::CONTENT_TYPE,"text/css; charset=utf-8")], include_str!("../web/fixture-placement.css")) }))
        .route("/interface.css", get(|| async { ([(header::CONTENT_TYPE,"text/css; charset=utf-8")], include_str!("../web/interface.css")) }))
        .route("/block-picker.css", get(|| async { ([(header::CONTENT_TYPE, "text/css; charset=utf-8")], include_str!("../web/block-picker.css")) }))
        .route("/plugins-ui.js", get(|| async { ([(header::CONTENT_TYPE, "text/javascript; charset=utf-8")], include_str!("../web/plugins-ui.js")) }))
        .route("/plugins.css", get(|| async { ([(header::CONTENT_TYPE, "text/css; charset=utf-8")], include_str!("../web/plugins.css")) }))
        .route(
            "/block-edit.css",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
                    include_str!("../web/block-edit.css"),
                )
            }),
        )
        .route("/api/plugins",get(plugin_status))
        .route("/api/plugins/load",post(plugin_load))
        .route("/api/plugins/reload",post(plugin_reload))
        .route("/api/plugins/unload",post(plugin_unload))
        .route("/api/plugins/watch",post(plugin_watch))
        .route("/api/storage/versions",get(||async{Json(crate::migrations::versions())}))
        .route("/api/storage/migrate",post(storage_migrate).layer(DefaultBodyLimit::max(64*1024*1024)))
        .route("/api/project/raw",get(project_raw))
        .route("/api/projects/import",post(project_import).layer(DefaultBodyLimit::max(64*1024*1024)))
        .route("/api/projects/bundle/raw",get(bundle_raw))
        .route("/api/history", get(history_get))
        .route("/api/projects/bundle", get(bundle_get).post(bundle_import).layer(DefaultBodyLimit::max(64*1024*1024)))
        .route("/api/session", get(session_get))
        .route("/api/session/save", post(session_save))
        .route("/api/session/status", get(session_status))
        .route("/api/inputs", post(input_event))
        .route("/dmx-test.js",get(||async{([(header::CONTENT_TYPE,"text/javascript; charset=utf-8")],include_str!("../web/dmx-test.js"))}))
        .route("/scene-stacks.js",get(||async{([(header::CONTENT_TYPE,"text/javascript; charset=utf-8")],include_str!("../web/scene-stacks.js"))}))
        .route("/api/output/devices", get(output_devices))
        .route("/dimming-editor.js",get(||async{ ([(header::CONTENT_TYPE,"text/javascript")],include_str!("../web/dimming-editor.js")) }))
        .route("/api/state", get(state_get))
        .route("/api/dimming/clock",get(dimming_clock_get).post(dimming_clock_control))
        .route("/api/state/compact",get(state_compact))
        .route("/api/inspection",post(inspection))
        .route("/block-inspection.js",get(||async{ ([(header::CONTENT_TYPE,"text/javascript")],include_str!("../web/block-inspection.js")) }))
        .route("/block-inspection.css",get(||async{ ([(header::CONTENT_TYPE,"text/css")],include_str!("../web/block-inspection.css")) }))
        .route("/property-editors.js", get(|| async { ([(header::CONTENT_TYPE,"text/javascript; charset=utf-8")],include_str!("../web/property-editors.js")) }))
        .route("/property-editors.css", get(|| async { ([(header::CONTENT_TYPE,"text/css; charset=utf-8")],include_str!("../web/property-editors.css")) }))
        .route("/stack-navigation.js", get(|| async { ([(header::CONTENT_TYPE,"text/javascript; charset=utf-8")],include_str!("../web/stack-navigation.js")) }))
        .route("/stack-navigation.css", get(|| async { ([(header::CONTENT_TYPE,"text/css; charset=utf-8")],include_str!("../web/stack-navigation.css")) }))
        .route("/stage-navigation.js", get(|| async { ([(header::CONTENT_TYPE,"text/javascript; charset=utf-8")],include_str!("../web/stage-navigation.js")) }))
        .route("/stage-navigation.css", get(|| async { ([(header::CONTENT_TYPE,"text/css; charset=utf-8")],include_str!("../web/stage-navigation.css")) }))
        .route("/system-logs.js", get(|| async { ([(header::CONTENT_TYPE,"text/javascript; charset=utf-8")], include_str!("../web/system-logs.js")) }))
        .route("/system-logs.css", get(|| async { ([(header::CONTENT_TYPE,"text/css; charset=utf-8")], include_str!("../web/system-logs.css")) }))
        .route("/api/logs", get(|State(state):State<AppState>|async move { Json(state.controller.logs.lock().unwrap_or_else(|e| e.into_inner()).read(crate::logs::Query::default())) }).post(logs_read))
        .route("/api/catalog", get(|State(state):State<AppState>|async move{Json(state.controller.plugin_catalog())}))
        .route("/api/command", post(command))
        .route("/api/import", post(import))
        .route("/api/export/{fixture}/{layer}", get(export))
        .route("/api/project/save", post(save))
        .route("/api/projects", get(project_list))
        .route("/api/projects/save", post(project_store))
        .route("/api/projects/load", post(project_open))
        .route("/api/content", get(content_list))
        .route("/api/content/load", post(load_content))
        .layer(DefaultBodyLimit::max(512 * 1024))
        .merge(crate::library_api::router())
        .layer(middleware::from_fn_with_state(state.clone(), guard))
        .with_state(state)
}
async fn output_devices() -> Json<serde_json::Value> {
    let found = tokio::task::spawn_blocking(crate::open_dmx::devices).await;
    match found {
        Ok(Ok(devices)) => Json(
            serde_json::json!({"open_dmx":crate::open_dmx::supported(),"devices":devices,"error":null}),
        ),
        result => Json(
            serde_json::json!({"open_dmx":crate::open_dmx::supported(),"devices":[],"error":match result {Ok(Err(e))=>e,Err(e)=>e.to_string(),_=>unreachable!()}}),
        ),
    }
}
async fn state_get(State(state): State<AppState>) -> Json<Published> {
    Json((*state.controller.current()).clone())
}
async fn command(
    State(state): State<AppState>,
    Json(body): Json<Envelope>,
) -> ApiResult<Json<serde_json::Value>> {
    let revision = state
        .controller
        .command(body.command, body.expected_revision)
        .await?;
    Ok(Json(serde_json::json!({"revision":revision})))
}
async fn import(
    State(state): State<AppState>,
    Json(body): Json<Import>,
) -> ApiResult<Json<serde_json::Value>> {
    let plugins = state.controller.current_plugins();
    let yaml = body.yaml;
    let stack =
        tokio::task::spawn_blocking(move || scenic::with_plugins(plugins, || scenic::parse(&yaml)))
            .await
            .map_err(|e| ApiError(e.to_string()))??;
    let revision = state
        .controller
        .command(
            Command::ReplaceLayer {
                fixture: body.fixture,
                layer: body.layer,
                stack: Some(stack),
            },
            body.expected_revision,
        )
        .await?;
    Ok(Json(serde_json::json!({"revision":revision})))
}
async fn export(
    State(state): State<AppState>,
    Path((fixture, layer)): Path<(String, usize)>,
) -> ApiResult<impl IntoResponse> {
    let p = state.controller.current();
    let layers = if fixture == "__director" {
        Some(&p.project.director)
    } else {
        p.project.content.get(&fixture)
    };
    let stack = layers
        .and_then(|v| v.get(layer))
        .and_then(Option::as_ref)
        .ok_or_else(|| "Empty fixture layer".to_string())?;
    Ok((
        [(axum::http::header::CONTENT_TYPE, "text/yaml; charset=utf-8")],
        scenic::with_plugins(p.plugins.clone(), || scenic::encode(stack))?,
    ))
}
async fn save(
    State(state): State<AppState>,
    Json(body): Json<Save>,
) -> ApiResult<Json<serde_json::Value>> {
    let _io_guard = state.io_lock.lock().await;
    let frame = state.controller.current();
    let root = state.root;
    let revision = frame.snapshot.revision;
    if body
        .expected_revision
        .is_some_and(|expected| expected != revision)
    {
        return Err(ApiError(
            "Revision conflict: refresh state before saving".into(),
        ));
    }
    tokio::task::spawn_blocking(move || {
        scenic::with_plugins(frame.plugins.clone(), || match body.scope {
            SaveScope::All => project::save(&root, &frame.project, &body.name),
            SaveScope::Layout => project::save_layout(&root, &frame.project, &body.name),
            SaveScope::Content => {
                let entries: Vec<_> = frame
                    .project
                    .content
                    .iter()
                    .chain(std::iter::once((
                        &"__director".to_owned(),
                        &frame.project.director,
                    )))
                    .filter(|(id, _)| body.fixtures.is_empty() || body.fixtures.contains(id))
                    .flat_map(|(id, layers)| {
                        layers
                            .iter()
                            .enumerate()
                            .filter(|(layer, _)| body.layer.is_none_or(|l| l == *layer))
                            .map(move |(layer, stack)| ContentEntry {
                                fixture: id.clone(),
                                layer,
                                stack: stack.clone(),
                            })
                    })
                    .collect();
                project::save_content(&root, &entries, &body.name)
            }
        })
    })
    .await
    .map_err(|e| ApiError(e.to_string()))??;
    if state.controller.session_store.is_some() {
        state.controller.persist_session().await?;
    }
    Ok(Json(serde_json::json!({"saved_revision":revision})))
}
async fn content_list(State(state): State<AppState>) -> ApiResult<Json<serde_json::Value>> {
    let _io_guard = state.io_lock.lock().await;
    let root = state.root;
    let plugins = state.controller.current().plugins.clone();
    let result = tokio::task::spawn_blocking(move || {
        scenic::with_plugins(plugins, || project::list_content(&root))
    })
    .await
    .map_err(|e| ApiError(e.to_string()))??;
    Ok(Json(result))
}
async fn load_content(
    State(state): State<AppState>,
    Json(body): Json<LoadContent>,
) -> ApiResult<Json<serde_json::Value>> {
    let frame = state.controller.current();
    let root = state.root;
    let name = body.name;
    let mut entries = tokio::task::spawn_blocking(move || project::read_content(&root, &name))
        .await
        .map_err(|e| ApiError(e.to_string()))??;
    if let Some(group) = body.group.filter(|g| !g.is_empty()) {
        entries.retain(|e| {
            frame
                .project
                .fixtures
                .iter()
                .any(|f| f.id == e.fixture && f.groups.contains(&group))
        });
    }
    let revision = state
        .controller
        .command(Command::ApplyContent { entries }, body.expected_revision)
        .await?;
    Ok(Json(serde_json::json!({"revision":revision})))
}

async fn session_get(State(state): State<AppState>) -> ApiResult<Json<crate::session::Session>> {
    Ok(Json(state.controller.checkpoint().await?))
}
async fn session_save(State(state): State<AppState>) -> ApiResult<Json<serde_json::Value>> {
    let revision = state.controller.persist_session().await?;
    Ok(Json(serde_json::json!({"saved_revision":revision})))
}
async fn session_status(State(state): State<AppState>) -> ApiResult<Json<crate::session::Status>> {
    Ok(Json(
        tokio::task::spawn_blocking(move || state.controller.session_status())
            .await
            .map_err(|e| ApiError(e.to_string()))?,
    ))
}

async fn input_event(
    State(state): State<AppState>,
    Json(event): Json<crate::inputs::Event>,
) -> ApiResult<Json<serde_json::Value>> {
    let revision = state
        .controller
        .command(Command::Input { event }, None)
        .await?;
    Ok(Json(serde_json::json!({"revision":revision})))
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct PluginFolder {
    folder: String,
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct PluginId {
    id: String,
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct PluginWatch {
    enabled: bool,
}
async fn plugin_status(
    State(state): State<AppState>,
) -> ApiResult<Json<crate::plugin_host::Status>> {
    Ok(Json(state.plugins.status().await?))
}
async fn plugin_load(
    State(state): State<AppState>,
    Json(body): Json<PluginFolder>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(
        serde_json::json!({"generation":state.plugins.load(&body.folder).await?}),
    ))
}
async fn plugin_reload(
    State(state): State<AppState>,
    Json(body): Json<PluginId>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(
        serde_json::json!({"generation":state.plugins.reload(&body.id).await?}),
    ))
}
async fn plugin_unload(
    State(state): State<AppState>,
    Json(body): Json<PluginId>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(
        serde_json::json!({"generation":state.plugins.unload(&body.id).await?}),
    ))
}
async fn plugin_watch(
    State(state): State<AppState>,
    Json(body): Json<PluginWatch>,
) -> ApiResult<Json<serde_json::Value>> {
    state.plugins.set_watch(body.enabled).await?;
    Ok(Json(serde_json::json!({"watch":body.enabled})))
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectStore {
    id: String,
    title: String,
    #[serde(default)]
    overwrite: bool,
    expected_revision: u64,
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectOpen {
    id: String,
    expected_revision: u64,
}
async fn project_list(State(state): State<AppState>) -> ApiResult<Json<serde_json::Value>> {
    let _guard = state.io_lock.lock().await;
    let root = state.root;
    let entries = tokio::task::spawn_blocking(move || crate::project_library::list(&root))
        .await
        .map_err(|e| e.to_string())??;
    Ok(Json(serde_json::json!({"projects":entries})))
}
async fn project_store(
    State(state): State<AppState>,
    Json(body): Json<ProjectStore>,
) -> ApiResult<Json<serde_json::Value>> {
    let _guard = state.io_lock.lock().await;
    let frame = state.controller.current();
    if frame.snapshot.revision != body.expected_revision {
        return Err("Revision conflict: refresh state before saving"
            .to_owned()
            .into());
    }
    let root = state.root;
    let revision = frame.snapshot.revision;
    let entry = tokio::task::spawn_blocking(move || {
        scenic::with_plugins(frame.plugins.clone(), || {
            crate::project_library::store(
                &root,
                &frame.project,
                &body.id,
                &body.title,
                body.overwrite,
                revision,
            )
        })
    })
    .await
    .map_err(|e| e.to_string())??;
    Ok(Json(
        serde_json::json!({"saved_revision":revision,"project":entry}),
    ))
}
async fn project_open(
    State(state): State<AppState>,
    Json(body): Json<ProjectOpen>,
) -> ApiResult<Json<serde_json::Value>> {
    let _guard = state.io_lock.lock().await;
    let frame = state.controller.current();
    let root = state.root;
    let project = tokio::task::spawn_blocking(move || {
        scenic::with_plugins(frame.plugins.clone(), || {
            crate::project_library::load(&root, &body.id)
        })
    })
    .await
    .map_err(|e| e.to_string())??;
    let revision = state
        .controller
        .command(
            Command::ReplaceProject {
                project: Box::new(project),
            },
            Some(body.expected_revision),
        )
        .await?;
    Ok(Json(serde_json::json!({"revision":revision})))
}

#[cfg(test)]
mod pin_access_tests {
    use super::*;
    #[test]
    fn failed_pin_windows_expire_without_a_sleep() {
        let mut attempts = PinAttempts::default();
        let peer = "10.42.0.20".parse().unwrap();
        let now = Instant::now();
        for _ in 0..5 {
            assert_eq!(
                attempts.authorize(peer, "0000", "1357", now),
                StatusCode::UNAUTHORIZED
            );
        }
        assert_eq!(
            attempts.authorize(peer, "1357", "1357", now + Duration::from_secs(59)),
            StatusCode::TOO_MANY_REQUESTS
        );
        assert_eq!(
            attempts.authorize(peer, "1357", "1357", now + Duration::from_secs(60)),
            StatusCode::OK
        );
        assert!(attempts.peers.is_empty());
        assert!(attempts.global.is_empty());
    }
}

async fn history_get(State(state): State<AppState>) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(crate::history::view(
        &state.controller.checkpoint().await?,
    )))
}
async fn bundle_get(State(state): State<AppState>) -> ApiResult<Json<crate::bundle::Bundle>> {
    let session = state.controller.checkpoint().await?;
    let root = state.root;
    Ok(Json(
        tokio::task::spawn_blocking(move || {
            let mut bundle = crate::bundle::Bundle::from_session(&session);
            bundle.collect(&root)?;
            Ok::<_, String>(bundle)
        })
        .await
        .map_err(|e| ApiError(e.to_string()))??,
    ))
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct BundleImport {
    bundle: Option<crate::bundle::Bundle>,
    text: Option<String>,
    expected_revision: u64,
}
async fn bundle_import(
    State(state): State<AppState>,
    Json(body): Json<BundleImport>,
) -> ApiResult<Json<serde_json::Value>> {
    let bundle = match (body.bundle, body.text) {
        (Some(bundle), None) => bundle,
        (None, Some(text)) => tokio::task::spawn_blocking(move || {
            serde_json::from_value::<crate::bundle::Bundle>(crate::migrations::parse(
                text.as_bytes(),
            )?)
            .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| ApiError(e.to_string()))??,
        _ => return Err(ApiError("Supply exactly one bundle or text".into())),
    };
    let revision = state
        .controller
        .import_bundle(bundle, Some(body.expected_revision))
        .await?;
    Ok(Json(serde_json::json!({"revision":revision})))
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct StorageMigration {
    format: crate::migrations::Kind,
    #[serde(default, deserialize_with = "crate::migrations::optional_value")]
    document: Option<serde_json::Value>,
    #[serde(default)]
    text: Option<String>,
}
async fn storage_migrate(
    State(state): State<AppState>,
    Json(body): Json<StorageMigration>,
) -> ApiResult<Json<serde_json::Value>> {
    let document = match (body.document, body.text) {
        (Some(document), None) => document,
        (None, Some(text)) => crate::migrations::parse(text.as_bytes())?,
        _ => return Err(ApiError("Supply exactly one document or text".into())),
    };
    let plugins = state.controller.current_plugins();
    let migrated = tokio::task::spawn_blocking(move || {
        scenic::with_plugins(plugins, || {
            crate::migrations::validated(body.format, document)
        })
    })
    .await
    .map_err(|e| ApiError(e.to_string()))??;
    Ok(Json(
        serde_json::json!({"document":migrated.document,"report":migrated.report}),
    ))
}

async fn project_raw(State(state): State<AppState>) -> ApiResult<impl IntoResponse> {
    Ok((
        [(header::CONTENT_TYPE, "application/octet-stream")],
        serde_json::to_vec(&state.controller.current().project)
            .map_err(|e| ApiError(e.to_string()))?,
    ))
}
async fn bundle_raw(State(state): State<AppState>) -> ApiResult<impl IntoResponse> {
    let Json(bundle) = bundle_get(State(state)).await?;
    Ok((
        [(header::CONTENT_TYPE, "application/octet-stream")],
        serde_json::to_vec(&bundle).map_err(|e| ApiError(e.to_string()))?,
    ))
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectImport {
    json: String,
    expected_revision: u64,
}
async fn project_import(
    State(state): State<AppState>,
    Json(body): Json<ProjectImport>,
) -> ApiResult<Json<serde_json::Value>> {
    let revision = state
        .controller
        .command(
            Command::ImportProject { json: body.json },
            Some(body.expected_revision),
        )
        .await?;
    Ok(Json(serde_json::json!({"revision":revision})))
}

async fn state_compact(State(state): State<AppState>) -> Json<Published> {
    Json((*state.controller.current()).clone().compact())
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct InspectionRequest {
    targets: Vec<DebugFocus>,
}
async fn inspection(
    State(state): State<AppState>,
    Json(body): Json<InspectionRequest>,
) -> ApiResult<Json<crate::inspection::Frame>> {
    Ok(Json(state.controller.inspection(body.targets).await?))
}

async fn dimming_clock_get(State(state): State<AppState>) -> Json<crate::dimming_clock::Status> {
    Json(state.controller.current().dimming_clock.clone())
}
async fn dimming_clock_control(
    State(state): State<AppState>,
    Json(control): Json<crate::dimming_clock::Control>,
) -> ApiResult<Json<crate::dimming_clock::Status>> {
    Ok(Json(state.controller.dimming_clock(control).await?))
}

async fn logs_read(
    State(state): State<AppState>,
    Json(query): Json<crate::logs::Query>,
) -> Json<crate::logs::Page> {
    Json(
        state
            .controller
            .logs
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .read(query),
    )
}
