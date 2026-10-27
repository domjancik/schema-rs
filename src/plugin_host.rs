//! Project-scoped plugin IO/control service; compilation occurs off the frame thread.
use crate::{
    model::Result,
    plugins::{Manifest, Registry},
    server::Controller,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Configuration {
    version: u32,
    registry: Arc<Registry>,
    watch: bool,
}
pub fn load_configuration(root: &Path) -> Result<(Arc<Registry>, bool)> {
    let path = root.join("registry/config/plugins.json");
    if !path.exists() {
        return Ok((Arc::new(Registry::default()), true));
    }
    if path.metadata().map_err(|e| e.to_string())?.len() > 4 * 1024 * 1024 + 1024 {
        return Err("Plugin configuration exceeds 4 MiB".into());
    }
    let config: Configuration =
        serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    if config.version != 1 {
        return Err("Unsupported plugin configuration".into());
    }
    Ok((config.registry, config.watch))
}
fn manifest_path(root: &Path, folder: &str) -> Result<PathBuf> {
    if !crate::plugins::valid_name(folder) {
        return Err("Invalid plugin folder name".into());
    }
    let base = root
        .join("plugins")
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let path = base
        .join(folder)
        .join("plugin.json")
        .canonicalize()
        .map_err(|e| e.to_string())?;
    if !path.starts_with(base) {
        return Err("Plugin manifest escapes the project plugins directory".into());
    }
    if path.metadata().map_err(|e| e.to_string())?.len() > 1024 * 1024 {
        return Err("Plugin manifest exceeds 1 MiB".into());
    }
    Ok(path)
}
fn read_manifest(root: &Path, folder: &str) -> Result<String> {
    std::fs::read_to_string(manifest_path(root, folder)?).map_err(|e| e.to_string())
}
#[derive(Serialize)]
pub struct Status {
    pub generation: u64,
    pub watch: bool,
    pub packages: Vec<serde_json::Value>,
    pub available: Vec<String>,
    pub errors: BTreeMap<String, String>,
    pub persistence_error: Option<String>,
}
pub struct Manager {
    controller: Controller,
    root: PathBuf,
    lock: tokio::sync::Mutex<()>,
    watch: AtomicBool,
    errors: Mutex<BTreeMap<String, String>>,
    persistence_error: Mutex<Option<String>>,
}
impl Manager {
    pub fn new(controller: Controller, root: PathBuf) -> Self {
        Self {
            controller,
            root,
            lock: tokio::sync::Mutex::new(()),
            watch: AtomicBool::new(true),
            errors: Mutex::new(BTreeMap::new()),
            persistence_error: Mutex::new(None),
        }
    }
    pub fn set_initial_watch(&self, enabled: bool) {
        self.watch.store(enabled, Ordering::Relaxed);
    }
    fn error(&self, key: &str, error: Option<&str>) {
        let mut errors = self.errors.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(error) = error {
            if errors.len() >= 128 && !errors.contains_key(key) {
                if let Some(first) = errors.keys().next().cloned() {
                    errors.remove(&first);
                }
            }
            errors.insert(key.into(), error.chars().take(4096).collect());
        } else {
            errors.remove(key);
        }
    }
    async fn persist(&self) {
        let config = Configuration {
            version: 1,
            registry: self.controller.current_plugins(),
            watch: self.watch.load(Ordering::Relaxed),
        };
        let root = self.root.clone();
        let result = tokio::task::spawn_blocking(move || -> Result<()> {
            let path = root.join("registry/config/plugins.json");
            std::fs::create_dir_all(path.parent().ok_or("Invalid plugin config path")?)
                .map_err(|e| e.to_string())?;
            let temp = path.with_extension("json.tmp");
            let bytes = serde_json::to_vec(&config).map_err(|e| e.to_string())?;
            std::fs::write(&temp, bytes).map_err(|e| e.to_string())?;
            std::fs::rename(temp, path).map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())
        .and_then(|r| r);
        *self
            .persistence_error
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = result.err();
    }
    pub async fn load(&self, folder: &str) -> Result<u64> {
        let _guard = self.lock.lock().await;
        self.load_locked(folder).await
    }
    async fn reload_active(&self, id: &str, generation: u64) -> Result<u64> {
        let _guard = self.lock.lock().await;
        let registry = self.controller.current_plugins();
        if registry.state.generation != generation || !self.watch.load(Ordering::Relaxed) {
            return Err("Revision conflict during watched reload".into());
        }
        let package = registry
            .state
            .packages
            .get(id)
            .filter(|p| p.enabled)
            .ok_or("Plugin was unloaded")?;
        self.load_locked(&package.folder).await
    }
    async fn load_locked(&self, folder: &str) -> Result<u64> {
        let root = self.root.clone();
        let folder_owned = folder.to_owned();
        let registry = self.controller.current_plugins();
        let prepared = tokio::task::spawn_blocking(move || {
            let text = read_manifest(&root, &folder_owned)?;
            registry.load_json(&folder_owned, &text).map(Arc::new)
        })
        .await
        .map_err(|e| e.to_string())?;
        let result = match prepared {
            Ok(registry) => self.controller.replace_plugins(registry).await,
            Err(e) => Err(e),
        };
        match &result {
            Ok(_) => {
                self.error(folder, None);
                self.persist().await
            }
            Err(error) => self.error(folder, Some(error)),
        }
        result
    }
    pub async fn reload(&self, id: &str) -> Result<u64> {
        let registry = self.controller.current_plugins();
        let folder = registry
            .state
            .packages
            .get(id)
            .ok_or("Unknown plugin")?
            .folder
            .clone();
        self.load(&folder).await
    }
    pub async fn unload(&self, id: &str) -> Result<u64> {
        let _guard = self.lock.lock().await;
        let registry = Arc::new(self.controller.current_plugins().set_enabled(id, false)?);
        let result = self.controller.replace_plugins(registry).await;
        if result.is_ok() {
            self.persist().await;
        }
        result
    }
    pub async fn set_watch(&self, enabled: bool) -> Result<()> {
        let _guard = self.lock.lock().await;
        self.watch.store(enabled, Ordering::Relaxed);
        self.persist().await;
        Ok(())
    }
    pub async fn status(&self) -> Result<Status> {
        let root = self.root.clone();
        let available = tokio::task::spawn_blocking(move || {
            let mut names = vec![];
            if let Ok(entries) = std::fs::read_dir(root.join("plugins")) {
                for entry in entries.flatten().take(256) {
                    let name = entry.file_name().to_string_lossy().into_owned();
                    if crate::plugins::valid_name(&name) && entry.path().is_dir() {
                        names.push(name);
                    }
                }
            }
            names.sort();
            names
        })
        .await
        .map_err(|e| e.to_string())?;
        let registry = self.controller.current_plugins();
        let packages=registry.state.packages.iter().map(|(id,p)|serde_json::json!({"id":id,"folder":p.folder,"version":p.manifest.version,"enabled":p.enabled,"components":p.manifest.components.iter().map(|c|format!("Plugin.{id}.{}",c.name)).collect::<Vec<_>>()})).collect();
        Ok(Status {
            generation: registry.state.generation,
            watch: self.watch.load(Ordering::Relaxed),
            packages,
            available,
            errors: self
                .errors
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone(),
            persistence_error: self
                .persistence_error
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone(),
        })
    }
    pub fn start_watcher(self: &Arc<Self>) -> Result<Watcher> {
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = stop.clone();
        let manager = self.clone();
        let worker = std::thread::Builder::new()
            .name("schema-plugin-watcher".into())
            .spawn(move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("plugin watcher runtime");
                let mut fingerprints: BTreeMap<String, (u64, u8, bool)> = BTreeMap::new();
                while !stopping.load(Ordering::Relaxed) {
                    if manager.watch.load(Ordering::Relaxed) {
                        let registry = manager.controller.current_plugins();
                        for (id, p) in &registry.state.packages {
                            if !p.enabled {
                                continue;
                            }
                            let text = read_manifest(&manager.root, &p.folder);
                            use std::hash::{Hash, Hasher};
                            let mut hasher = std::collections::hash_map::DefaultHasher::new();
                            text.hash(&mut hasher);
                            let hash = hasher.finish();
                            let record = fingerprints.entry(id.clone()).or_insert((hash, 0, false));
                            if record.0 != hash {
                                *record = (hash, 0, false);
                            }
                            record.1 = record.1.saturating_add(1);
                            if record.1 < 2 || record.2 {
                                continue;
                            }
                            record.2 = true;
                            match text {
                                Ok(text) => {
                                    let same = serde_json::from_str::<Manifest>(&text)
                                        .is_ok_and(|m| m == p.manifest);
                                    if same {
                                        manager.error(&p.folder, None);
                                    }
                                    if !same {
                                        let result = runtime.block_on(
                                            manager.reload_active(id, registry.state.generation),
                                        );
                                        if result
                                            .as_ref()
                                            .is_err_and(|e| e.contains("Revision conflict"))
                                        {
                                            record.2 = false;
                                        }
                                    }
                                }
                                Err(error) => manager.error(&p.folder, Some(&error)),
                            }
                        }
                    }
                    std::thread::sleep(Duration::from_millis(250));
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(Watcher {
            stop,
            worker: Some(worker),
        })
    }
}
pub struct Watcher {
    stop: Arc<AtomicBool>,
    worker: Option<std::thread::JoinHandle<()>>,
}
impl Drop for Watcher {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
