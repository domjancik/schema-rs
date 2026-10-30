//! Named, immutable project generations with an atomic manifest as the save boundary.
use crate::storage_formats::Manifest;
pub use crate::storage_formats::ProjectEntry as Entry;
use crate::{
    model::{Project, Result},
    project,
};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};
// User IDs never become arbitrary paths. Reject links and existing escaped ancestors too.
pub(crate) fn guarded(root: &Path, relative: &str) -> Result<PathBuf> {
    let mut path = root.to_path_buf();
    let base = root.canonicalize().ok();
    for component in Path::new(relative).components() {
        if !matches!(component, std::path::Component::Normal(_)) {
            return Err("Invalid project path".into());
        }
        path.push(component);
        match fs::symlink_metadata(&path) {
            Ok(meta) => {
                if meta.file_type().is_symlink() {
                    return Err("Project paths cannot contain symbolic links".into());
                }
                if let Some(base) = &base {
                    if !path
                        .canonicalize()
                        .map_err(|e| e.to_string())?
                        .starts_with(base)
                    {
                        return Err("Project path escapes workspace".into());
                    }
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(path)
}
fn manifest(root: &Path, id: &str) -> Result<Manifest> {
    project::filename(id)?;
    let path = guarded(root, &format!("projects/{id}/project.json"))?;
    if fs::metadata(&path).map_err(|e| e.to_string())?.len() > 1024 * 1024 {
        return Err("Project manifest is too large".into());
    }
    let m: Manifest = serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    project::filename(&m.generation)?;
    if m.version != crate::migrations::MANIFEST_VERSION || m.entry.id != id || m.entry.legacy {
        return Err("Invalid saved project manifest".into());
    }
    Ok(m)
}
pub fn list(root: &Path) -> Result<Vec<Entry>> {
    let mut entries = std::collections::BTreeMap::new();
    let directory = guarded(root, "projects")?;
    if directory.exists() {
        for child in fs::read_dir(directory)
            .map_err(|e| e.to_string())?
            .take(1024)
        {
            let child = child.map_err(|e| e.to_string())?;
            let id = child.file_name().to_string_lossy().into_owned();
            if let Ok(m) = manifest(root, &id) {
                entries.insert(id, m.entry);
            }
        }
    }
    let layouts = guarded(root, "registry/layouts")?;
    if layouts.exists() {
        for child in fs::read_dir(layouts).map_err(|e| e.to_string())?.take(1024) {
            let child = child.map_err(|e| e.to_string())?;
            let path = child.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            let id = path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            if project::filename(&id).is_err() || entries.contains_key(&id) {
                continue;
            }
            let content = guarded(root, &format!("registry/content/{id}.json"))?;
            if content.is_file() {
                entries.insert(
                    id.clone(),
                    Entry {
                        id: id.clone(),
                        title: id,
                        fixtures: 0,
                        saved_revision: 0,
                        modified_unix_ms: 0,
                        legacy: true,
                    },
                );
            }
        }
    }
    let mut result: Vec<_> = entries.into_values().collect();
    result.sort_by_key(|e| (e.title.to_lowercase(), e.id.clone()));
    Ok(result)
}
pub fn store(
    root: &Path,
    p: &Project,
    id: &str,
    title: &str,
    overwrite: bool,
    revision: u64,
) -> Result<Entry> {
    project::filename(id)?;
    if title.trim().is_empty() || title.len() > 200 || title.chars().any(char::is_control) {
        return Err("Project title must contain 1–200 printable characters".into());
    }
    let pointer = guarded(root, &format!("projects/{id}/project.json"))?;
    let legacy = guarded(root, &format!("registry/layouts/{id}.json"))?;
    if !overwrite && (pointer.exists() || legacy.exists()) {
        return Err("Project already exists; choose another ID or explicitly overwrite it".into());
    }
    if pointer.exists() {
        crate::migrations::backup_existing(&pointer, crate::migrations::Kind::ProjectManifest)?;
    }
    let mut saved = p.clone();
    saved.name = title.trim().into();
    saved.validate()?;
    static SERIAL: AtomicU64 = AtomicU64::new(0);
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_millis() as u64;
    let generation = format!(
        "{ms}-{}-{}",
        std::process::id(),
        SERIAL.fetch_add(1, Ordering::Relaxed)
    );
    let location = guarded(root, &format!("projects/{id}/revisions/{generation}"))?;
    fs::create_dir_all(&location).map_err(|e| e.to_string())?;
    project::save(&location, &saved, "init")?;
    let entry = Entry {
        id: id.into(),
        title: saved.name.clone(),
        fixtures: saved.fixtures.len(),
        saved_revision: revision,
        modified_unix_ms: ms,
        legacy: false,
    };
    let m = Manifest {
        version: crate::migrations::MANIFEST_VERSION,
        generation,
        entry: entry.clone(),
    };
    project::write(
        pointer,
        &serde_json::to_vec_pretty(&m).map_err(|e| e.to_string())?,
    )?;
    Ok(entry)
}
pub fn load(root: &Path, id: &str) -> Result<Project> {
    project::filename(id)?;
    let pointer = guarded(root, &format!("projects/{id}/project.json"))?;
    let (directory, name) = if pointer.exists() {
        let m = manifest(root, id)?;
        (
            guarded(root, &format!("projects/{id}/revisions/{}", m.generation))?,
            "init",
        )
    } else {
        (root.to_path_buf(), id)
    };
    for path in [
        format!("registry/layouts/{name}.json"),
        format!("registry/content/{name}.json"),
        "registry/config/output.json".into(),
    ] {
        let file = guarded(&directory, &path)?;
        if fs::metadata(file).map_err(|e| e.to_string())?.len() > 16 * 1024 * 1024 {
            return Err("Saved project file is too large".into());
        }
    }
    project::load(&directory, name)
}
