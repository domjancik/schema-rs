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
            plugins: engine.plugins(),
            project: engine.project().clone(),
            snapshot: engine.snapshot(),
        })));
        let (tx, rx) = mpsc::sync_channel::<RequestMessage>(64);
        let stop = Arc::new(AtomicBool::new(false));
        let controller = Controller {
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
                    let next = Arc::new(Published {
                        host: engine.host(),
                        plugins: engine.plugins(),
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
                        eprintln!("Node action: {error}");
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
        let persistence_worker = if let Some(store) = session_store {
            let stopping = persistence_stop.clone();
            Some(
                std::thread::Builder::new()
                    .name("schema-session-writer".into())
                    .spawn(move || {
                        let mut saved: Option<Arc<crate::session::Session>> = None;
                        loop {
                            let checkpoint = latest_session
                                .read()
                                .unwrap_or_else(|e| e.into_inner())
                                .clone();
                            if saved
                                .as_ref()
                                .is_none_or(|old| !Arc::ptr_eq(old, &checkpoint))
                                && store.save(&checkpoint).is_ok()
                            {
                                saved = Some(checkpoint);
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
#[derive(Clone)]
pub struct AppState {
    pub plugins: Arc<crate::plugin_host::Manager>,
    pub controller: Controller,
    pub root: PathBuf,
    token: Option<Arc<String>>,
    io_lock: Arc<tokio::sync::Mutex<()>>,
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
type ApiResult<T> = std::result::Result<T, ApiError>;
struct ApiError(String);
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
            if headers.get("x-schema-token").and_then(|v| v.to_str().ok()) != Some(token.as_str()) {
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
        .route("/api/session", get(session_get))
        .route("/api/session/save", post(session_save))
        .route("/api/session/status", get(session_status))
        .route("/api/inputs", post(input_event))
        .route("/dmx-test.js",get(||async{([(header::CONTENT_TYPE,"text/javascript; charset=utf-8")],include_str!("../web/dmx-test.js"))}))
        .route("/scene-stacks.js",get(||async{([(header::CONTENT_TYPE,"text/javascript; charset=utf-8")],include_str!("../web/scene-stacks.js"))}))
        .route("/api/output/devices", get(output_devices))
        .route("/api/state", get(state_get))
        .route("/api/catalog", get(|State(state):State<AppState>|async move{Json(state.controller.plugin_catalog())}))
        .route("/api/command", post(command))
        .route("/api/import", post(import))
        .route("/api/export/{fixture}/{layer}", get(export))
        .route("/api/project/save", post(save))
        .route("/api/projects", get(project_list))
        .route("/api/projects/save", post(project_store))
        .route("/api/projects/load", post(project_open))
        .route("/api/content/load", post(load_content))
        .layer(DefaultBodyLimit::max(512 * 1024))
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
