use crate::storage_formats::{Content, Layout, Output};
use crate::{model::*, scenic};
use std::{
    fs,
    path::{Path, PathBuf},
};
pub(crate) fn filename(name: &str) -> Result<()> {
    if name.is_empty()
        || name.len() > 80
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err("Snapshot names must contain only letters, digits, - and _".into());
    }
    Ok(())
}
pub(crate) fn write(path: PathBuf, bytes: &[u8]) -> Result<()> {
    use std::{
        io::Write,
        sync::atomic::{AtomicU64, Ordering},
    };
    static SERIAL: AtomicU64 = AtomicU64::new(0);
    let parent = path.parent().ok_or("Project file has no parent")?;
    let temp = parent.join(format!(
        ".project-{}-{}-{}.tmp",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos(),
        SERIAL.fetch_add(1, Ordering::Relaxed)
    ));
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)
        .map_err(|e| e.to_string())?;
    let result = (|| {
        file.write_all(bytes).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        drop(file);
        fs::rename(&temp, &path).map_err(|e| e.to_string())?;
        #[cfg(unix)]
        fs::File::open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(|e| e.to_string())?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

pub fn initialize(root: &Path) -> Result<()> {
    for dir in [
        "registry/config/window",
        "registry/layouts",
        "registry/content",
        "registry/stacks",
        "registry/mapping",
        "registry/favorites",
        "resources",
        "plugins",
    ] {
        fs::create_dir_all(root.join(dir)).map_err(|e| e.to_string())?;
    }
    Ok(())
}
pub fn save(root: &Path, p: &Project, name: &str) -> Result<()> {
    filename(name)?;
    p.validate()?;
    initialize(root)?;
    for (path, kind) in [
        (
            format!("registry/layouts/{name}.json"),
            crate::migrations::Kind::Layout,
        ),
        (
            format!("registry/content/{name}.json"),
            crate::migrations::Kind::Content,
        ),
        (
            "registry/config/output.json".into(),
            crate::migrations::Kind::Output,
        ),
    ] {
        crate::migrations::backup_existing(&root.join(path), kind)?;
    }
    let layout = Layout {
        dmx_models: p.dmx_models.clone(),
        dimming: p.dimming.clone(),
        version: crate::migrations::LAYOUT_VERSION,
        name: p.name.clone(),
        fixtures: p.fixtures.clone(),
    };
    write(
        root.join(format!("registry/layouts/{name}.json")),
        &serde_json::to_vec_pretty(&layout).map_err(|e| e.to_string())?,
    )?;
    let entries: Vec<_> = p
        .content
        .iter()
        .chain(std::iter::once((&"__director".to_owned(), &p.director)))
        .flat_map(|(fixture, layers)| {
            layers
                .iter()
                .enumerate()
                .map(|(layer, stack)| ContentEntry {
                    fixture: fixture.clone(),
                    layer,
                    stack: stack.clone(),
                })
        })
        .collect();
    write(
        root.join(format!("registry/content/{name}.json")),
        &serde_json::to_vec_pretty(&Content {
            version: crate::migrations::CONTENT_VERSION,
            entries,
        })
        .map_err(|e| e.to_string())?,
    )?;
    write(
        root.join("registry/config/output.json"),
        &serde_json::to_vec_pretty(&Output {
            version: crate::migrations::OUTPUT_VERSION,
            output: p.output.clone(),
        })
        .map_err(|e| e.to_string())?,
    )?;
    for (fixture, layers) in p
        .content
        .iter()
        .chain(std::iter::once((&"__director".to_owned(), &p.director)))
    {
        for (layer, stack) in layers.iter().enumerate() {
            if let Some(stack) = stack {
                let safe: String = fixture
                    .chars()
                    .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
                    .collect();
                write(
                    root.join(format!("registry/stacks/{safe}-{layer}.scnq.yaml")),
                    scenic::encode(stack)?.as_bytes(),
                )?
            }
        }
    }
    Ok(())
}
pub fn load(root: &Path, name: &str) -> Result<Project> {
    filename(name)?;
    let layout: Layout = serde_json::from_slice(
        &fs::read(root.join(format!("registry/layouts/{name}.json"))).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let entries = read_content(root, name)?;
    let mut content = layout
        .fixtures
        .iter()
        .map(|f| (f.id.clone(), Vec::new()))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut director = Vec::new();
    for e in entries {
        if e.layer >= MAX_LAYERS {
            return Err("Invalid saved layer".into());
        }
        let layers = if e.fixture == "__director" {
            &mut director
        } else if let Some(layers) = content.get_mut(&e.fixture) {
            layers
        } else {
            continue;
        };
        layers.resize_with(layers.len().max(e.layer + 1), || None);
        layers[e.layer] = e.stack;
    }
    for layers in content.values_mut().chain(std::iter::once(&mut director)) {
        if layers.is_empty() {
            layers.resize(3, None);
        }
    }
    let output: Output = serde_json::from_slice(
        &fs::read(root.join("registry/config/output.json")).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let mut output = output.output;
    output.enabled = false;
    let p = Project {
        dmx_models: layout.dmx_models,
        dimming: layout.dimming,
        version: crate::migrations::PROJECT_VERSION,
        name: layout.name,
        fixtures: layout.fixtures,
        content,
        director,
        output,
    };
    p.validate()?;
    Ok(p)
}
pub fn read_content(root: &Path, name: &str) -> Result<Vec<ContentEntry>> {
    filename(name)?;
    let value = crate::migrations::parse(
        &fs::read(root.join(format!("registry/content/{name}.json"))).map_err(|e| e.to_string())?,
    )?;
    let document: Content = serde_json::from_value(value).map_err(|e| e.to_string())?;
    document.validate()?;
    Ok(document.entries)
}
pub fn save_content(root: &Path, entries: &[ContentEntry], name: &str) -> Result<()> {
    filename(name)?;
    crate::migrations::backup_existing(
        &root.join(format!("registry/content/{name}.json")),
        crate::migrations::Kind::Content,
    )?;
    initialize(root)?;
    let mut targets = std::collections::HashSet::new();
    for e in entries {
        if e.layer >= MAX_LAYERS || !targets.insert((&e.fixture, e.layer)) {
            return Err("Invalid or duplicate content target".into());
        }
        if let Some(s) = &e.stack {
            scenic::validate(s)?;
        }
    }
    write(
        root.join(format!("registry/content/{name}.json")),
        &serde_json::to_vec_pretty(&Content {
            version: crate::migrations::CONTENT_VERSION,
            entries: entries.to_vec(),
        })
        .map_err(|e| e.to_string())?,
    )
}
pub fn save_layout(root: &Path, p: &Project, name: &str) -> Result<()> {
    filename(name)?;
    crate::migrations::backup_existing(
        &root.join(format!("registry/layouts/{name}.json")),
        crate::migrations::Kind::Layout,
    )?;
    p.validate()?;
    initialize(root)?;
    let layout = Layout {
        dmx_models: p.dmx_models.clone(),
        dimming: p.dimming.clone(),
        version: crate::migrations::LAYOUT_VERSION,
        name: p.name.clone(),
        fixtures: p.fixtures.clone(),
    };
    write(
        root.join(format!("registry/layouts/{name}.json")),
        &serde_json::to_vec_pretty(&layout).map_err(|e| e.to_string())?,
    )
}

/// Lightweight content scope for selection UIs; no stacks or runtime state are published.
pub fn list_content(root: &Path) -> Result<serde_json::Value> {
    let folder = crate::project_library::guarded(root, "registry/content")?;
    let files = match fs::read_dir(folder) {
        Ok(files) => files,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(serde_json::json!({"contents":[]}))
        }
        Err(error) => return Err(error.to_string()),
    };
    let mut names = Vec::new();
    for entry in files {
        let entry = entry.map_err(|e| e.to_string())?;
        if !entry.file_type().map_err(|e| e.to_string())?.is_file() {
            continue;
        }
        let path = entry.path();
        if path.extension().is_none_or(|e| e != "json") {
            continue;
        }
        let Some(name) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        if filename(name).is_ok() {
            names.push(name.to_owned());
        }
    }
    names.sort();
    let contents: Vec<_> = names.into_iter().map(|name| {
        let result=(|| {
            let path=crate::project_library::guarded(root,&format!("registry/content/{name}.json"))?;
            if fs::metadata(path).map_err(|e|e.to_string())?.len()>16*1024*1024{return Err("Content exceeds 16 MiB".into());}
            read_content(root,&name)
        })();
        match result {
            Ok(entries)=>serde_json::json!({"name":name,"targets":entries.iter().map(|e|serde_json::json!({"fixture":e.fixture,"layer":e.layer})).collect::<Vec<_>>()}),
            Err(error)=>serde_json::json!({"name":name,"targets":[],"error":error}),
        }
    }).collect();
    Ok(serde_json::json!({"contents":contents}))
}
