//! Same-origin, authenticated library storage and a bounded, LAN-only peer bridge.
use crate::{
    library::{self, Entry, Item, Kind},
    server::{ApiError, ApiResult, AppState},
};
use axum::{
    extract::{DefaultBodyLimit, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    net::{IpAddr, SocketAddr},
    time::Duration,
};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Key {
    key: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Import {
    pub item: Item,
    #[serde(default)]
    pub expected_hash: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Load {
    key: String,
    expected_revision: u64,
}
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/library", get(index))
        .route("/api/library/export", post(export))
        .route("/api/library/import", post(import))
        .route("/api/library/load", post(load))
        .route("/api/library/publish", post(publish))
        .route("/api/library/peer", post(peer))
        .layer(DefaultBodyLimit::max(64 * 1024 * 1024))
}
async fn make(state: AppState, key: String) -> crate::model::Result<Item> {
    let session = state.controller.checkpoint().await?;
    let root = state.root;
    tokio::task::spawn_blocking(move || {
        crate::scenic::with_plugins(session.plugins.clone(), || {
            if key.starts_with("portable:") {
                return library::read(&root, &key);
            }
            let mut bundle = crate::bundle::Bundle::from_session(&session);
            bundle.collect(&root)?;
            let item = if key == "current" {
                Item::new(
                    Kind::Project,
                    "current-project".into(),
                    bundle.project.name.clone(),
                    bundle,
                    None,
                )
            } else if let Some(id) = key.strip_prefix("project:") {
                library::valid_id(id)?;
                bundle.project = crate::project_library::load(&root, id)?;
                Item::new(
                    Kind::Project,
                    id.into(),
                    bundle.project.name.clone(),
                    bundle,
                    None,
                )
            } else if let Some(id) = key.strip_prefix("content:") {
                library::valid_id(id)?;
                let content = crate::storage_formats::Content {
                    version: crate::migrations::CONTENT_VERSION,
                    entries: crate::project::read_content(&root, id)?,
                };
                Item::new(Kind::Content, id.into(), id.into(), bundle, Some(content))
            } else {
                return Err("Unknown library entry".into());
            };
            item.validate()?;
            Ok(item)
        })
    })
    .await
    .map_err(|e| e.to_string())?
}
async fn index(State(state): State<AppState>) -> ApiResult<Json<Value>> {
    let _lock = state.io_lock.lock().await;
    let frame = state.controller.current();
    let root = state.root.clone();
    let name = frame.project.name.clone();
    let mut entries = vec![Entry {
        key: "current".into(),
        kind: Kind::Project,
        id: "current-project".into(),
        title: name.clone(),
        origin: "Running project".into(),
        fixtures: frame.project.fixtures.len(),
        layers: 0,
        hash: String::new(),
        bytes: 0,
    }];
    let other = tokio::task::spawn_blocking(move || {
        crate::scenic::with_plugins(frame.plugins.clone(), || {
            let mut out = library::list(&root)?;
            for p in crate::project_library::list(&root)? {
                out.push(Entry {
                    key: format!("project:{}", p.id),
                    kind: Kind::Project,
                    id: p.id,
                    title: p.title,
                    origin: "Saved projects".into(),
                    fixtures: p.fixtures,
                    layers: 0,
                    hash: String::new(),
                    bytes: 0,
                });
            }
            let contents = crate::project::list_content(&root)?;
            if let Some(items) = contents["contents"].as_array() {
                for c in items {
                    if c.get("error").is_some() {
                        continue;
                    }
                    if let Some(id) = c["name"].as_str() {
                        out.push(Entry {
                            key: format!("content:{id}"),
                            kind: Kind::Content,
                            id: id.into(),
                            title: id.into(),
                            origin: "Content presets".into(),
                            fixtures: 0,
                            layers: c["targets"].as_array().map_or(0, Vec::len),
                            hash: String::new(),
                            bytes: 0,
                        });
                    }
                }
            }
            Ok::<_, String>(out)
        })
    })
    .await
    .map_err(|e| e.to_string())??;
    entries.extend(other);
    Ok(Json(
        json!({"protocol":1,"name":name,"entries":entries,"direct_transfer":true}),
    ))
}
async fn export(State(state): State<AppState>, Json(body): Json<Key>) -> ApiResult<Json<Value>> {
    let _lock = state.io_lock.lock().await;
    Ok(Json(
        serde_json::to_value(make(state.clone(), body.key).await?).map_err(|e| e.to_string())?,
    ))
}
async fn import(State(state): State<AppState>, Json(body): Json<Import>) -> ApiResult<Json<Value>> {
    let _lock = state.io_lock.lock().await;
    let root = state.root.clone();
    let entry = tokio::task::spawn_blocking(move || {
        library::store(&root, &body.item, body.expected_hash.as_deref())
    })
    .await
    .map_err(|e| e.to_string())??;
    Ok(Json(json!({"entry":entry,"stored_only":true})))
}
async fn publish(State(state): State<AppState>, Json(body): Json<Load>) -> ApiResult<Json<Value>> {
    let _lock = state.io_lock.lock().await;
    let frame = state.controller.current();
    if frame.snapshot.revision != body.expected_revision {
        return Err("Revision conflict: refresh before publishing"
            .to_owned()
            .into());
    }
    let item = make(state.clone(), body.key).await?;
    let name = item.id;
    let content = item
        .content
        .ok_or("Only content presets can be published".to_owned())?;
    let root = state.root.clone();
    let id = name.clone();
    tokio::task::spawn_blocking(move||crate::scenic::with_plugins(frame.plugins.clone(),||{
        content.validate()?;
        let path=crate::project_library::guarded(&root,&format!("registry/content/{id}.json"))?;
        if path.exists(){let previous=crate::project::read_content(&root,&id)?;if serde_json::to_value(previous).map_err(|e|e.to_string())?!=serde_json::to_value(&content.entries).map_err(|e|e.to_string())?{return Err("Content library name conflict; save a copy with a different ID before publishing".into());}}
        crate::project::save_content(&root,&content.entries,&id)
    })).await.map_err(|e|e.to_string())??;
    Ok(Json(json!({"published":true,"name":name})))
}
async fn load(State(state): State<AppState>, Json(body): Json<Load>) -> ApiResult<Json<Value>> {
    let _lock = state.io_lock.lock().await;
    let item = make(state.clone(), body.key).await?;
    let revision = state
        .controller
        .import_library(item, Some(body.expected_revision))
        .await?;
    Ok(Json(json!({"revision":revision})))
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Peer {
    pub origin: String,
    #[serde(default)]
    pub token: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Operation {
    List,
    Export,
    Import,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PeerRequest {
    peer: Peer,
    operation: Operation,
    key: Option<String>,
    item: Option<Item>,
    expected_hash: Option<String>,
}
pub fn allowed_address(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(ip) => ip.is_loopback() || ip.is_private(),
        IpAddr::V6(ip) => {
            ip.is_loopback()
                || (ip.segments()[0] & 0xfe00) == 0xfc00
                || (ip.segments()[0] & 0xffc0) == 0xfe80
        }
    }
}
impl Peer {
    pub fn url(&self) -> crate::model::Result<reqwest::Url> {
        let url =
            reqwest::Url::parse(&self.origin).map_err(|_| "Enter an HTTP(S) server origin")?;
        if !matches!(url.scheme(), "http" | "https")
            || !url.username().is_empty()
            || url.password().is_some()
            || url.path() != "/"
            || url.query().is_some()
            || url.fragment().is_some()
            || url.host_str().is_none()
            || self.token.len() > 1200
            || self.token.chars().any(char::is_control)
        {
            return Err("Use an HTTP(S) server origin without path, credentials or query".into());
        }
        Ok(url)
    }
    async fn request(&self, path: &str, body: Option<Value>) -> crate::model::Result<Value> {
        let url = self.url()?;
        let host = url.host_str().ok_or("Missing peer hostname")?;
        let port = url.port_or_known_default().ok_or("Missing peer port")?;
        let addresses: Vec<SocketAddr> = tokio::time::timeout(
            Duration::from_secs(5),
            tokio::net::lookup_host((host.trim_matches(['[', ']']), port)),
        )
        .await
        .map_err(|_| "Server address lookup timed out")?
        .map_err(|_| "Unable to resolve the server address")?
        .collect();
        if addresses.is_empty()
            || addresses.len() > 16
            || addresses.iter().any(|a| !allowed_address(a.ip()))
        {
            return Err("Direct transfers support private LAN and loopback servers only. Use a file for other addresses".into());
        }
        let client = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(90))
            .resolve_to_addrs(host, &addresses)
            .build()
            .map_err(|_| "Unable to create peer connection")?;
        let address = url.join(path).map_err(|_| "Invalid peer endpoint")?;
        let mut request = if body.is_some() {
            client.post(address)
        } else {
            client.get(address)
        };
        request = request
            .header("origin", url.origin().ascii_serialization())
            .header("x-schema-token", &self.token);
        if let Some(body) = body {
            request = request.json(&body);
        }
        let mut response = request
            .send()
            .await
            .map_err(|_| "Peer connection failed. Check address, network and TLS certificate")?;
        if !response.status().is_success() {
            return Err(format!(
                "Peer returned HTTP {}. Check its access key and update Schéma on both servers",
                response.status().as_u16()
            ));
        }
        if response
            .content_length()
            .is_some_and(|s| s > 64 * 1024 * 1024)
        {
            return Err("Peer response exceeds 64 MiB".into());
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| "Peer transfer interrupted")?
        {
            if bytes.len() + chunk.len() > 64 * 1024 * 1024 {
                return Err("Peer response exceeds 64 MiB".into());
            }
            bytes.extend(chunk);
        }
        serde_json::from_slice(&bytes)
            .map_err(|_| "Peer returned an incompatible response. Update both servers".to_owned())
    }
}
async fn peer(Json(body): Json<PeerRequest>) -> ApiResult<Json<Value>> {
    let (path, data) = match body.operation {
        Operation::List => ("/api/library", None),
        Operation::Export => (
            "/api/library/export",
            Some(json!({"key":body.key.ok_or("Choose a source item".to_string())?})),
        ),
        Operation::Import => (
            "/api/library/import",
            Some(
                json!({"item":body.item.ok_or("Choose a source item".to_string())?,"expected_hash":body.expected_hash}),
            ),
        ),
    };
    let value = body.peer.request(path, data).await.map_err(ApiError)?;
    Ok(Json(value))
}
