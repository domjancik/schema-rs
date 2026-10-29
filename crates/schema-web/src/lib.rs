//! Browser binding only: the runtime has no JS, storage or transport dependencies.
use schema_rs::{engine::Command, model::Project, runtime::EngineSession};
use serde_json::{json, Value};
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
pub struct BrowserEngine {
    session: EngineSession,
}
fn error(message: impl ToString) -> JsValue {
    JsValue::from_str(&message.to_string())
}
#[wasm_bindgen]
impl BrowserEngine {
    #[wasm_bindgen(constructor)]
    pub fn new(project: Option<String>) -> Result<BrowserEngine, JsValue> {
        let mut project: Project = match project {
            Some(text) => serde_json::from_str(&text).map_err(error)?,
            None => Project::demo(),
        };
        project.output.enabled = false;
        Ok(Self {
            session: EngineSession::new(project).map_err(error)?,
        })
    }
    pub fn restore(&mut self, checkpoint: &str) -> Result<(), JsValue> {
        self.session = EngineSession::decode_session(checkpoint.as_bytes()).map_err(error)?;
        Ok(())
    }
    pub fn checkpoint(&self) -> Result<String, JsValue> {
        String::from_utf8(self.session.encode_session().map_err(error)?).map_err(error)
    }
    pub fn advance(&mut self, dt: f64) -> Result<String, JsValue> {
        serde_json::to_string(&self.session.advance(dt).map_err(error)?).map_err(error)
    }
    pub fn request(&mut self, path: &str, body: &str) -> Result<String, JsValue> {
        let body: Value = serde_json::from_str(body).map_err(error)?;
        let expected = body.get("expected_revision").and_then(Value::as_u64);
        if body
            .get("expected_revision")
            .is_some_and(|v| !v.is_null() && v.as_u64().is_none())
        {
            return Err(error("Expected revision must be an unsigned integer"));
        }
        let value = match path {
            "/api/state" => serde_json::to_value(self.session.publication()).map_err(error)?,
            "/api/catalog" => json!(self.session.catalog()),
            "/api/command" => {
                let mut command: Command =
                    serde_json::from_value(body["command"].clone()).map_err(error)?;
                browser_command(&mut command).map_err(error)?;
                json!({"revision": self.session.command(command, expected).map_err(error)?})
            }
            "/api/inputs" => {
                let event = serde_json::from_value(body).map_err(error)?;
                self.session
                    .command(Command::Input { event }, None)
                    .map_err(error)?;
                json!({"ok":true})
            }
            "/api/import" => json!({"revision":self.session.import_stack(
                body["fixture"].as_str().ok_or_else(||error("Fixture required"))?.into(),
                body["layer"].as_u64().ok_or_else(||error("Layer required"))? as usize,
                body["yaml"].as_str().ok_or_else(||error("ScenicScript required"))?,expected).map_err(error)?}),
            "/api/session" => return self.checkpoint(),
            "/api/plugins" => {
                let frame = self.session.publication();
                let packages = frame.plugins.state.packages.iter().map(|(id,p)|json!({"id":id,"folder":p.folder,"version":p.manifest.version,"enabled":p.enabled,"components":p.manifest.components.iter().map(|c|&c.name).collect::<Vec<_>>()})).collect::<Vec<_>>();
                json!({"watch":false,"packages":packages,"available":[],"errors":{"browser":"Native folder loading and file watching are unavailable. Embedded session components evaluate normally."}})
            }
            "/api/output/devices" => {
                json!({"open_dmx":false,"devices":[],"error":"Browser host has no physical outputs; computed scene values remain available."})
            }
            "/api/capabilities" => {
                json!({"execution":"browser-worker","network_output":false,"usb_output":false,"filesystem":false,"session_serialization":true,"scene_editing":true})
            }
            _ if path.starts_with("/api/export/") => {
                let parts = path
                    .trim_start_matches("/api/export/")
                    .rsplit_once('/')
                    .ok_or_else(|| error("Invalid stack address"))?;
                let layer: usize = parts.1.parse().map_err(error)?;
                let frame = self.session.publication();
                let layers = if parts.0 == "__director" {
                    &frame.project.director
                } else {
                    frame
                        .project
                        .content
                        .get(parts.0)
                        .ok_or_else(|| error("Unknown fixture"))?
                };
                let stack = layers
                    .get(layer)
                    .and_then(Option::as_ref)
                    .ok_or_else(|| error("Empty layer"))?;
                return schema_rs::scenic::with_plugins(frame.plugins, || {
                    schema_rs::scenic::encode(stack)
                })
                .map_err(error);
            }
            _ => return Err(error(format!("This browser host does not support {path}"))),
        };
        serde_json::to_string(&value).map_err(error)
    }
}

fn browser_command(command: &mut Command) -> Result<(), String> {
    match command {
        Command::SetOutput { config } if config.enabled => {
            return Err("Physical output is unavailable in the browser host".into())
        }
        Command::ReplaceProject { project } => project.output.enabled = false,
        Command::Batch { commands } => {
            for command in commands {
                browser_command(command)?;
            }
        }
        _ => {}
    }
    Ok(())
}
