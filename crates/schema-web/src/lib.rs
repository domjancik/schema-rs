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
    pub fn restore(&mut self, checkpoint: &str) -> Result<String, JsValue> {
        let (session, report) =
            EngineSession::decode_session_with_report(checkpoint.as_bytes()).map_err(error)?;
        let report = serde_json::to_string(&report).map_err(error)?;
        self.session = session;
        Ok(report)
    }
    pub fn checkpoint(&self) -> Result<String, JsValue> {
        String::from_utf8(self.session.encode_session().map_err(error)?).map_err(error)
    }
    pub fn advance(&mut self, dt: f64) -> Result<String, JsValue> {
        serde_json::to_string(&self.session.advance(dt).map_err(error)?.compact()).map_err(error)
    }
    pub fn request(&mut self, path: &str, body: &str) -> Result<String, JsValue> {
        let body: Value = schema_rs::migrations::parse(body.as_bytes()).map_err(error)?;
        let expected = body.get("expected_revision").and_then(Value::as_u64);
        if body
            .get("expected_revision")
            .is_some_and(|v| !v.is_null() && v.as_u64().is_none())
        {
            return Err(error("Expected revision must be an unsigned integer"));
        }
        let value = match path {
            "/api/project/raw" => {
                return self
                    .session
                    .storage_snapshot(schema_rs::migrations::Kind::Project, None, vec![], None)
                    .map_err(error)
            }
            "/api/storage/snapshot" => {
                let kind = serde_json::from_value(body["format"].clone()).map_err(error)?;
                let title = body.get("title").and_then(Value::as_str).map(str::to_owned);
                let fixtures = body
                    .get("fixtures")
                    .and_then(Value::as_array)
                    .map(|a| {
                        a.iter()
                            .filter_map(Value::as_str)
                            .map(str::to_owned)
                            .collect()
                    })
                    .unwrap_or_default();
                let layer = body
                    .get("layer")
                    .and_then(Value::as_u64)
                    .map(|i| i as usize);
                return self
                    .session
                    .storage_snapshot(kind, title, fixtures, layer)
                    .map_err(error);
            }
            "/api/projects/import" => {
                json!({"revision":self.session.command(Command::ImportProject{json:body["json"].as_str().ok_or_else(||error("Project JSON required"))?.into()},expected).map_err(error)?})
            }
            "/api/resources/content" => {
                self.session
                    .content_resource(
                        body["name"]
                            .as_str()
                            .ok_or_else(|| error("Content name required"))?
                            .into(),
                        body["text"]
                            .as_str()
                            .ok_or_else(|| error("Content text required"))?,
                    )
                    .map_err(error)?;
                json!({"ok":true})
            }
            "/api/content/load" => {
                json!({"revision":self.session.load_content_text(body["text"].as_str().ok_or_else(||error("Content text required"))?,body.get("group").and_then(Value::as_str).map(str::to_owned),expected).map_err(error)?})
            }
            "/api/projects/compose" => {
                json!({"revision":self.session.compose_project(body.get("layout").and_then(Value::as_str).map(str::to_owned),body.get("content").and_then(Value::as_str).map(str::to_owned),expected).map_err(error)?})
            }
            "/api/storage/versions" => schema_rs::migrations::versions(),
            "/api/storage/migrate" => {
                let kind: schema_rs::migrations::Kind =
                    serde_json::from_value(body["format"].clone()).map_err(error)?;
                let document = match (
                    body.get("document"),
                    body.get("text").and_then(Value::as_str),
                ) {
                    (Some(document), None) => document.clone(),
                    (None, Some(text)) => {
                        schema_rs::migrations::parse(text.as_bytes()).map_err(error)?
                    }
                    _ => return Err(error("Supply exactly one document or text")),
                };
                let result = self
                    .session
                    .migrate_document(kind, document)
                    .map_err(error)?;
                json!({"document":result.document,"report":result.report})
            }
            "/api/history" => self.session.history(),
            "/api/projects/bundle" => {
                if body.is_null() {
                    serde_json::to_value(self.session.bundle()).map_err(error)?
                } else {
                    let value = match (body.get("bundle"), body.get("text").and_then(Value::as_str))
                    {
                        (Some(value), None) => value.clone(),
                        (None, Some(text)) => {
                            schema_rs::migrations::parse(text.as_bytes()).map_err(error)?
                        }
                        _ => return Err(error("Supply exactly one bundle or text")),
                    };
                    let bundle = serde_json::from_value(value).map_err(error)?;
                    json!({"revision":self.session.import_bundle(bundle,expected).map_err(error)?})
                }
            }
            "/api/logs" => serde_json::to_value(self.session.logs(if body.is_null() {
                schema_rs::logs::Query::default()
            } else {
                serde_json::from_value(body).map_err(error)?
            }))
            .map_err(error)?,
            "/api/inspection" => serde_json::to_value(
                self.session
                    .inspection(serde_json::from_value(body["targets"].clone()).map_err(error)?)
                    .map_err(error)?,
            )
            .map_err(error)?,
            "/api/state/compact" => {
                serde_json::to_value(self.session.publication().compact()).map_err(error)?
            }
            "/api/dimming/clock" => serde_json::to_value(if body.is_null() {
                self.session.publication().dimming_clock
            } else {
                self.session
                    .set_dimming_clock(serde_json::from_value(body).map_err(error)?)
                    .map_err(error)?
            })
            .map_err(error)?,
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
