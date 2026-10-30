use schema_rs::{
    model::Project,
    server::{router, AppState, Runtime},
};
use std::{net::SocketAddr, path::PathBuf};
#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("schema-rs: {error}");
        std::process::exit(1)
    }
}
async fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let option = |name: &str, default: &str| {
        args.iter()
            .position(|v| v == name)
            .and_then(|i| args.get(i + 1))
            .cloned()
            .unwrap_or_else(|| default.into())
    };
    let token = std::env::var("SCHEMA_TOKEN").ok();
    match args.first().map(String::as_str).unwrap_or("serve") {
        "serve" | "desktop" => {
            let desktop = args.first().is_some_and(|arg| arg == "desktop");
            let default_root = if desktop {
                schema_rs::desktop::project_root()?
                    .to_string_lossy()
                    .into_owned()
            } else {
                "schema-project".into()
            };
            let root = PathBuf::from(option("--project", &default_root));
            if ["--resume-state", "--resume-session", "--fresh-session"]
                .iter()
                .filter(|flag| args.contains(&flag.to_string()))
                .count()
                > 1
            {
                return Err(
                    "Choose only one of --resume-state, --resume-session or --fresh-session".into(),
                );
            }
            let (configured_plugins, plugin_watch) =
                schema_rs::plugin_host::load_configuration(&root)?;
            let session_path = root.join(".session/session.json");
            let session = if let Some(i) = args.iter().position(|a| a == "--resume-session") {
                Some(schema_rs::session::load(std::path::Path::new(
                    args.get(i + 1)
                        .ok_or("--resume-session requires a checkpoint file")?,
                ))?)
            } else if session_path.exists()
                && !args
                    .iter()
                    .any(|a| a == "--fresh-session" || a == "--resume-state")
            {
                let mut checkpoint = schema_rs::session::load(&session_path)?;
                checkpoint.disarm_output();
                Some(checkpoint)
            } else {
                None
            };
            let resume: Option<schema_rs::server::Published> =
                if let Some(i) = args.iter().position(|v| v == "--resume-state") {
                    let file = args
                        .get(i + 1)
                        .ok_or("--resume-state requires a trusted local publication JSON file")?;
                    Some(
                        serde_json::from_slice(&std::fs::read(file).map_err(|e| e.to_string())?)
                            .map_err(|e| format!("Resume state: {e}"))?,
                    )
                } else {
                    None
                };
            let mut project = if let Some(session) = &session {
                (*session.project).clone()
            } else if let Some(state) = &resume {
                state.project.clone()
            } else if root.join("registry/layouts/init.json").exists() {
                schema_rs::scenic::with_plugins(configured_plugins.clone(), || {
                    schema_rs::project::load(&root, "init")
                })?
            } else {
                Project::demo()
            };
            if let Some(i) = args.iter().position(|v| v == "--artnet") {
                project.output.transport = schema_rs::model::OutputTransport::ArtNet;
                project.output.target = args.get(i + 1).ok_or("--artnet requires IP:port")?.clone();
                project.output.enabled = true;
            }
            let bind: SocketAddr = option(
                "--bind",
                if desktop {
                    "127.0.0.1:8797"
                } else {
                    "127.0.0.1:8787"
                },
            )
            .parse()
            .map_err(|_| "Invalid bind address")?;
            if !bind.ip().is_loopback()
                && token
                    .as_ref()
                    .is_none_or(|t| !schema_rs::server::valid_lan_token(t))
            {
                return Err("LAN hosting requires SCHEMA_TOKEN with four ASCII digits or at least 16 characters".into());
            }
            let is_live = project.output.enabled;
            let mut engine = if let Some(session) = session {
                println!("Restoring session history and runtime state");
                schema_rs::engine::Engine::from_session(session)?
            } else {
                let mut engine = schema_rs::engine::Engine::new_with_plugins(
                    project.clone(),
                    configured_plugins.clone(),
                )?;
                if let Some(state) = resume {
                    engine.resume_snapshot(state.snapshot)?;
                }
                engine
            };
            if configured_plugins.state.generation > engine.plugins().state.generation {
                let update = schema_rs::engine::Engine::prepare_plugin_update(
                    engine.checkpoint(),
                    configured_plugins,
                )?;
                engine.install_plugin_update(update)?;
            }
            if engine.project().output != project.output {
                engine.apply(schema_rs::engine::Command::SetOutput {
                    config: project.output,
                })?;
            }
            let listener = tokio::net::TcpListener::bind(bind)
                .await
                .map_err(|e| e.to_string())?;
            let runtime = Runtime::start_engine_guarded(
                engine,
                Some(session_path),
                Some(root.join(".update-in-progress")),
            )?;
            let osc = args
                .iter()
                .position(|a| a == "--osc")
                .map(|i| {
                    args.get(i + 1)
                        .ok_or("--osc requires IP:port")
                        .and_then(|s| s.parse().map_err(|_| "Invalid OSC bind"))
                })
                .transpose()?;
            let _inputs =
                schema_rs::io::Services::start(runtime.controller.clone(), root.clone(), osc)?;
            println!("Schéma engine: http://{}\nTUI: schema-rs tui --server {}\nProject: {}\nArt-Net: {}",listener.local_addr().unwrap(),bind,root.display(),if is_live{"enabled"}else{"disabled (capture only)"});
            let app_state = AppState::new(runtime.controller.clone(), root, token);
            app_state.plugins.set_initial_watch(plugin_watch);
            let plugin_watcher = app_state.start_plugin_watcher()?;
            if desktop && !args.iter().any(|arg| arg == "--no-open") {
                let url = format!(
                    "http://{}/welcome",
                    listener.local_addr().map_err(|e| e.to_string())?
                );
                tokio::task::spawn_blocking(move || {
                    std::thread::sleep(std::time::Duration::from_millis(200));
                    if let Err(error) = schema_rs::desktop::open_browser(&url) {
                        eprintln!("Open {url} in your browser ({error})");
                    }
                });
            }
            axum::serve(
                listener,
                router(app_state).into_make_service_with_connect_info::<SocketAddr>(),
            )
            .with_graceful_shutdown(async {
                let _ = tokio::signal::ctrl_c().await;
            })
            .await
            .map_err(|e| e.to_string())?;
            drop(plugin_watcher);
            drop(_inputs);
            runtime.controller.persist_session().await?;
            drop(runtime);
            Ok(())
        }
        "plugins" => {
            let action = args.get(1).map(String::as_str).unwrap_or("list");
            let (method, path, body) = match action {
                "list" => ("GET", "/api/plugins", None),
                "load" => (
                    "POST",
                    "/api/plugins/load",
                    Some(
                        serde_json::json!({"folder":args.get(2).ok_or("plugins load requires a folder")?}),
                    ),
                ),
                "reload" => (
                    "POST",
                    "/api/plugins/reload",
                    Some(
                        serde_json::json!({"id":args.get(2).ok_or("plugins reload requires an id")?}),
                    ),
                ),
                "unload" => (
                    "POST",
                    "/api/plugins/unload",
                    Some(
                        serde_json::json!({"id":args.get(2).ok_or("plugins unload requires an id")?}),
                    ),
                ),
                "watch" => (
                    "POST",
                    "/api/plugins/watch",
                    Some(
                        serde_json::json!({"enabled":match args.get(2).map(String::as_str){Some("on")=>true,Some("off")=>false,_=>return Err("plugins watch requires on/off".into())}}),
                    ),
                ),
                _ => return Err("Use plugins list/load/reload/unload/watch".into()),
            };
            let data = schema_rs::tui::request(
                &option("--server", "127.0.0.1:8787"),
                method,
                path,
                body.as_ref(),
                token.as_deref(),
            )?;
            println!("{}", String::from_utf8_lossy(&data));
            Ok(())
        }
        "tui" if args.iter().any(|a| a == "--snapshot") => {
            let data = schema_rs::tui::request(
                &option("--server", "127.0.0.1:8787"),
                "GET",
                "/api/state",
                None,
                token.as_deref(),
            )?;
            let state: schema_rs::server::Published =
                serde_json::from_slice(&data).map_err(|e| e.to_string())?;
            let stack = state
                .project
                .content
                .values()
                .flatten()
                .flatten()
                .next()
                .ok_or("No stack")?;
            println!(
                "{}",
                schema_rs::tui::render_stack(&stack.root, stack.root.id, 3, 150, 45)
            );
            Ok(())
        }
        "tui" => schema_rs::tui::run(&option("--server", "127.0.0.1:8787"), token.as_deref()),
        "migrate" => {
            use std::io::{Read, Write};
            let kind: schema_rs::migrations::Kind = option("--format", "session").parse()?;
            let input = PathBuf::from(option("--input", ""));
            if input.as_os_str().is_empty() {
                return Err("migrate requires --input FILE".into());
            }
            let file = std::fs::File::open(&input).map_err(|e| e.to_string())?;
            let mut bytes = Vec::new();
            file.take(schema_rs::migrations::MAX_BYTES as u64 + 1)
                .read_to_end(&mut bytes)
                .map_err(|e| e.to_string())?;
            let migrated =
                schema_rs::migrations::validated(kind, schema_rs::migrations::parse(&bytes)?)?;
            let output = option("--output", "");
            if !output.is_empty() {
                let output = PathBuf::from(output);
                let mut options = std::fs::OpenOptions::new();
                options.write(true).create_new(true);
                #[cfg(unix)]
                {
                    use std::os::unix::fs::OpenOptionsExt;
                    options.mode(0o600);
                }
                let mut file = options
                    .open(&output)
                    .map_err(|e| format!("Migration output must be a new file: {e}"))?;
                let bytes =
                    serde_json::to_vec_pretty(&migrated.document).map_err(|e| e.to_string())?;
                file.write_all(&bytes).map_err(|e| e.to_string())?;
                file.sync_all().map_err(|e| e.to_string())?;
            }
            println!(
                "{}",
                serde_json::json!({"format":kind,"version":kind.current(),"report":migrated.report})
            );
            Ok(())
        }
        "init" => {
            let root = PathBuf::from(option("--project", "schema-project"));
            if root.join("registry/layouts/init.json").exists() {
                return Err("Project already initialized".into());
            }
            schema_rs::project::save(&root, &Project::demo(), "init")?;
            println!("Initialized {}", root.display());
            Ok(())
        }
        "validate" => {
            let path = args.get(1).ok_or("validate requires a ScenicScript file")?;
            let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
            let stack = schema_rs::scenic::parse(&text)?;
            println!("Valid: {}", stack.name);
            Ok(())
        }
        "capture" => {
            let mut e = schema_rs::engine::Engine::new(Project::demo())?;
            for _ in 0..120 {
                e.tick(1. / 60.);
            }
            println!(
                "{}",
                serde_json::to_string_pretty(&e.snapshot()).map_err(|e| e.to_string())?
            );
            Ok(())
        }
        _ => {
            println!("schema-rs desktop [--project DIR] [--bind 127.0.0.1:PORT] [--no-open]\nschema-rs serve [--project DIR] [--bind IP:PORT] [--artnet IP:PORT] [--resume-session FILE | --fresh-session]\nschema-rs tui [--server HOST:PORT]\nschema-rs plugins list|load FOLDER|reload ID|unload ID|watch on|off [--server HOST:PORT]\nschema-rs init [--project DIR]\nschema-rs validate FILE\nschema-rs capture\nLAN API authentication: SCHEMA_TOKEN environment variable");
            Ok(())
        }
    }
}
