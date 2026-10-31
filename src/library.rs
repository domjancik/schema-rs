//! Portable library records. Saving a transfer never changes an engine or extracts files.
use crate::{bundle::Bundle, engine::Engine, model::Result, storage_formats::Content};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Project,
    Content,
}
impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Project => "project",
            Self::Content => "content",
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Item {
    pub format: String,
    pub version: u32,
    pub kind: Kind,
    pub id: String,
    pub title: String,
    #[serde(with = "json_text")]
    pub bundle: Bundle,
    #[serde(with = "json_text")]
    pub content: Option<Content>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Entry {
    pub key: String,
    pub kind: Kind,
    pub id: String,
    pub title: String,
    pub origin: String,
    pub fixtures: usize,
    pub layers: usize,
    pub hash: String,
    pub bytes: usize,
}
pub fn valid_id(id: &str) -> Result<()> {
    if id.is_empty()
        || id.len() > 80
        || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err("Use 1–80 letters, digits, - or _ for the library ID".into());
    }
    Ok(())
}
impl Item {
    pub fn new(
        kind: Kind,
        id: String,
        title: String,
        bundle: Bundle,
        content: Option<Content>,
    ) -> Self {
        Self {
            format: "schema-library-item".into(),
            version: 1,
            kind,
            id,
            title,
            bundle,
            content,
        }
    }
    pub fn references_init(&self) -> bool {
        fn contains(value: &serde_json::Value) -> bool {
            match value {
                serde_json::Value::String(text) => text == "init",
                serde_json::Value::Array(values) => values.iter().any(contains),
                serde_json::Value::Object(values) => values.values().any(contains),
                _ => false,
            }
        }
        fn walk(n: &crate::model::Node) -> bool {
            let mut properties = crate::scenic::defaults(&n.behavior).unwrap_or_default();
            properties.extend(n.properties.clone());
            properties.values().any(contains)
                || n.children.iter().chain(n.drivers.values()).any(walk)
        }
        let roots: Vec<&crate::model::Node> = if let Some(content) = &self.content {
            content
                .entries
                .iter()
                .filter_map(|e| e.stack.as_ref().map(|s| &s.root))
                .collect()
        } else {
            self.bundle
                .project
                .content
                .values()
                .chain(std::iter::once(&self.bundle.project.director))
                .flatten()
                .flatten()
                .map(|s| &s.root)
                .collect()
        };
        let bundled_references = self.bundle.assets.stacks.values().any(|s| walk(&s.root))
            || self
                .bundle
                .assets
                .content
                .iter()
                .filter(|(name, _)| name.as_str() != "init")
                .any(|(_, entries)| {
                    entries
                        .iter()
                        .filter_map(|e| e.stack.as_ref())
                        .any(|s| walk(&s.root))
                });
        bundled_references
            || roots.into_iter().any(walk)
            || serde_json::to_value(&self.bundle.plugins).is_ok_and(|v| contains(&v))
    }
    pub fn validate(&self) -> Result<()> {
        if self.format != "schema-library-item" || self.version != 1 {
            return Err("Unsupported library item version; use a newer Schéma app. Existing data was retained".into());
        }
        valid_id(&self.id)?;
        self.entry()?;
        if self.title.trim().is_empty()
            || self.title.len() > 200
            || self.title.chars().any(char::is_control)
        {
            return Err("Library title must contain 1–200 printable characters".into());
        }
        if (self.kind == Kind::Content) != self.content.is_some() {
            return Err("Library kind does not match its content".into());
        }
        let session = Engine::new_with_plugins(
            crate::model::Project::demo(),
            std::sync::Arc::new(crate::plugins::Registry::default()),
        )?
        .checkpoint();
        let prepared = self.bundle.clone().prepare(session)?;
        crate::scenic::with_plugins(prepared.plugins.clone(), || {
            if let Some(content) = &self.content {
                content.validate()?;
            }
            Engine::from_session(prepared).map(|_| ())
        })
    }
    pub fn entry(&self) -> Result<Entry> {
        let bytes = serde_json::to_vec(self).map_err(|e| e.to_string())?;
        if bytes.len() > 64 * 1024 * 1024 {
            return Err("Library item exceeds 64 MiB".into());
        }
        Ok(Entry {
            key: format!("portable:{}:{}", self.kind.label(), self.id),
            kind: self.kind,
            id: self.id.clone(),
            title: self.title.clone(),
            origin: "Portable library".into(),
            fixtures: self.bundle.project.fixtures.len(),
            layers: self.content.as_ref().map_or(0, |c| c.entries.len()),
            hash: format!("{:x}", Sha256::digest(&bytes)),
            bytes: bytes.len(),
        })
    }
}
#[cfg(feature = "native")]
fn path(root: &std::path::Path, key: &str) -> Result<std::path::PathBuf> {
    let parts = key.split(':').collect::<Vec<_>>();
    if parts.len() != 3 || parts[0] != "portable" || !matches!(parts[1], "project" | "content") {
        return Err("Invalid portable library key".into());
    }
    valid_id(parts[2])?;
    crate::project_library::guarded(root, &format!("library/{}/{}.json", parts[1], parts[2]))
}
#[cfg(feature = "native")]
pub fn read(root: &std::path::Path, key: &str) -> Result<Item> {
    let path = path(root, key)?;
    if std::fs::metadata(&path).map_err(|e| e.to_string())?.len() > 64 * 1024 * 1024 {
        return Err("Library item exceeds 64 MiB".into());
    }
    let value = crate::migrations::parse(&std::fs::read(path).map_err(|e| e.to_string())?)?;
    let item: Item = serde_json::from_value(value).map_err(|e| e.to_string())?;
    item.validate()?;
    if item.entry()?.key != key {
        return Err("Library key mismatch".into());
    }
    Ok(item)
}
#[cfg(feature = "native")]
pub fn store(root: &std::path::Path, item: &Item, expected_hash: Option<&str>) -> Result<Entry> {
    item.validate()?;
    let entry = item.entry()?;
    let path = path(root, &entry.key)?;
    if path.exists() {
        let previous = read(root, &entry.key)?;
        if expected_hash != Some(previous.entry()?.hash.as_str()) {
            return Err("Revision conflict: library item exists or changed; review it and explicitly replace".into());
        }
        let serial = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos();
        let backup = crate::project_library::guarded(
            root,
            &format!(
                "library/backups/{}-{}-{serial}.json",
                item.kind.label(),
                item.id
            ),
        )?;
        std::fs::create_dir_all(backup.parent().ok_or("Invalid backup path")?)
            .map_err(|e| e.to_string())?;
        crate::project::write(backup, &std::fs::read(&path).map_err(|e| e.to_string())?)?;
    } else if expected_hash.is_some() {
        return Err("Revision conflict: library item no longer exists".into());
    }
    std::fs::create_dir_all(path.parent().ok_or("Invalid library path")?)
        .map_err(|e| e.to_string())?;
    crate::project::write(path, &serde_json::to_vec(item).map_err(|e| e.to_string())?)?;
    Ok(entry)
}
#[cfg(feature = "native")]
pub fn list(root: &std::path::Path) -> Result<Vec<Entry>> {
    let mut entries = Vec::new();
    for kind in ["project", "content"] {
        let dir = crate::project_library::guarded(root, &format!("library/{kind}"))?;
        if !dir.exists() {
            continue;
        }
        for f in std::fs::read_dir(dir)
            .map_err(|e| e.to_string())?
            .take(1024)
        {
            let f = f.map_err(|e| e.to_string())?;
            if f.path().extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            let id = f
                .path()
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            if let Ok(item) = read(root, &format!("portable:{kind}:{id}")) {
                entries.push(item.entry()?);
            }
        }
    }
    Ok(entries)
}

// Keep typed property numbers lossless across JavaScript relays (1.0 versus 1).
mod json_text {
    use serde::{de::DeserializeOwned, Deserialize, Deserializer, Serialize, Serializer};
    pub fn serialize<T: Serialize, S: Serializer>(
        value: &T,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&serde_json::to_string(value).map_err(serde::ser::Error::custom)?)
    }
    pub fn deserialize<'de, T: DeserializeOwned, D: Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<T, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        let value = if let Some(text) = value.as_str() {
            crate::migrations::parse(text.as_bytes()).map_err(serde::de::Error::custom)?
        } else {
            value
        };
        serde_json::from_value(value).map_err(serde::de::Error::custom)
    }
}
