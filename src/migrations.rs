//! Ordered, lossless storage upgrades. Run before typed decoding; never rewrite a source on load.
use crate::model::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
pub const PROJECT_VERSION: u32 = 2;
pub const SESSION_VERSION: u32 = 2;
pub const BUNDLE_VERSION: u32 = 2;
pub const LAYOUT_VERSION: u32 = 2;
pub const CONTENT_VERSION: u32 = 2;
pub const OUTPUT_VERSION: u32 = 2;
pub const MANIFEST_VERSION: u32 = 2;
pub const MAX_BYTES: usize = 64 * 1024 * 1024;
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Project,
    Session,
    Bundle,
    Layout,
    Content,
    Output,
    #[serde(rename = "manifest")]
    ProjectManifest,
}
impl Kind {
    pub fn current(self) -> u32 {
        match self {
            Self::Project => PROJECT_VERSION,
            Self::Session => SESSION_VERSION,
            Self::Bundle => BUNDLE_VERSION,
            Self::Layout => LAYOUT_VERSION,
            Self::Content => CONTENT_VERSION,
            Self::Output => OUTPUT_VERSION,
            Self::ProjectManifest => MANIFEST_VERSION,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Project => "project",
            Self::Session => "session",
            Self::Bundle => "bundle",
            Self::Layout => "layout",
            Self::Content => "content",
            Self::Output => "output",
            Self::ProjectManifest => "manifest",
        }
    }
}
impl std::str::FromStr for Kind {
    type Err = String;
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "project" => Ok(Self::Project),
            "session" => Ok(Self::Session),
            "bundle" => Ok(Self::Bundle),
            "layout" => Ok(Self::Layout),
            "content" => Ok(Self::Content),
            "output" => Ok(Self::Output),
            "manifest" => Ok(Self::ProjectManifest),
            _ => {
                Err("Expected project, session, bundle, layout, content, output or manifest".into())
            }
        }
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct Step {
    pub format: Kind,
    pub path: String,
    pub from: u32,
    pub to: u32,
    pub description: &'static str,
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct Report {
    pub steps: Vec<Step>,
}
impl Report {
    pub fn changed(&self) -> bool {
        !self.steps.is_empty()
    }
}
pub struct Migrated {
    pub document: Value,
    pub report: Report,
}
/// A single field declaration supplies both the public model and its private wire decoder.
/// All Deserialize entry points, including nested models and serde_json::from_value, migrate.
#[macro_export]
macro_rules! versioned_document{
 ($kind:expr,$wire:ident,$remote:literal;$(#[$attr:meta])* $vis:vis struct $name:ident {$($fields:tt)*})=>{
  $(#[$attr])* $vis struct $name {$($fields)*}
  #[derive(serde::Deserialize)]
  #[serde(remote=$remote,deny_unknown_fields)]
  struct $wire {$($fields)*}
  impl<'de> serde::Deserialize<'de> for $name{
   fn deserialize<D:serde::Deserializer<'de>>(deserializer:D)->std::result::Result<Self,D::Error>{
    let value=$crate::migrations::deserialize_value(deserializer)?;
    let migrated=$crate::migrations::migrate($kind,value).map_err(serde::de::Error::custom)?;
    $wire::deserialize(migrated.document).map_err(serde::de::Error::custom)
   }
  }
 };
}
pub fn parse(bytes: &[u8]) -> Result<Value> {
    if bytes.len() > MAX_BYTES {
        return Err("Storage document exceeds 64 MiB".into());
    }
    let (mut depth, mut quoted, mut escaped) = (0usize, false, false);
    for &byte in bytes {
        if quoted {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                quoted = false;
            }
        } else {
            match byte {
                b'"' => quoted = true,
                b'{' | b'[' => {
                    depth += 1;
                    if depth > 160 {
                        return Err("Storage JSON nesting exceeds 160 levels".into());
                    }
                }
                b'}' | b']' => depth = depth.saturating_sub(1),
                _ => {}
            }
        }
    }
    let mut parser = serde_json::Deserializer::from_slice(bytes);
    parser.disable_recursion_limit();
    let value = deserialize_value(&mut parser).map_err(|e| format!("Storage format: {e}"))?;
    parser.end().map_err(|e| format!("Storage format: {e}"))?;
    Ok(value)
}
pub fn version(kind: Kind, value: &Value, path: &str) -> Result<u32> {
    if kind == Kind::Content && value.is_array() {
        return Ok(1);
    }
    if kind == Kind::Output && value.is_object() && value.get("version").is_none() {
        return Ok(1);
    }
    let number = value
        .get("version")
        .and_then(Value::as_u64)
        .ok_or_else(|| {
            format!(
                "{} at {path}: version must be an unsigned integer",
                kind.name()
            )
        })?;
    let version = u32::try_from(number)
        .map_err(|_| format!("{} at {path}: unsupported version {number}", kind.name()))?;
    if version == 0 || version > kind.current() {
        return Err(format!(
            "{} at {path}: unsupported {}version {version}; this engine supports 1..{}",
            kind.name(),
            if version > kind.current() {
                "future "
            } else {
                ""
            },
            kind.current()
        ));
    }
    Ok(version)
}
pub fn migrate(kind: Kind, mut document: Value) -> Result<Migrated> {
    let mut pending = vec![(&document, 0usize)];
    while let Some((value, depth)) = pending.pop() {
        if depth > 160 {
            return Err("Storage nesting exceeds 160 levels".into());
        }
        match value {
            Value::Array(values) => pending.extend(values.iter().map(|v| (v, depth + 1))),
            Value::Object(values) => pending.extend(values.values().map(|v| (v, depth + 1))),
            _ => {}
        }
    }
    let mut report = Report::default();
    upgrade(kind, &mut document, "$", &mut report)?;
    Ok(Migrated { document, report })
}
fn object<'a>(value: &'a mut Value, path: &str) -> Result<&'a mut Map<String, Value>> {
    value
        .as_object_mut()
        .ok_or_else(|| format!("Expected an object at {path}"))
}
fn node(value: &mut Value, path: &str) -> Result<()> {
    let map = object(value, path)?;
    map.entry("bypassed").or_insert(json!(false));
    map.entry("properties").or_insert(json!({}));
    map.entry("children").or_insert(json!([]));
    map.entry("drivers").or_insert(json!({}));
    if let Some(children) = map.get_mut("children").and_then(Value::as_array_mut) {
        for child in children {
            node(child, path)?;
        }
    }
    if let Some(drivers) = map.get_mut("drivers").and_then(Value::as_object_mut) {
        for child in drivers.values_mut() {
            node(child, path)?;
        }
    }
    Ok(())
}
fn stack(value: &mut Value, path: &str) -> Result<()> {
    if value.is_null() {
        return Ok(());
    }
    let map = object(value, path)?;
    map.entry("enabled").or_insert(json!(true));
    if let Some(root) = map.get_mut("root") {
        node(root, path)?;
    }
    Ok(())
}
fn layers(value: &mut Value, path: &str) -> Result<()> {
    if let Some(layers) = value.as_array_mut() {
        for layer in layers {
            stack(layer, path)?;
        }
    }
    Ok(())
}
fn fixture_defaults(value: &mut Value, path: &str) -> Result<()> {
    let map = object(value, path)?;
    for (key, value) in [
        ("dmx_model", Value::Null),
        ("setup_channel", json!(0)),
        ("rotation", json!([0, 0, 0])),
        ("pixel_positions", json!([[0, 0, 0]])),
        ("pixel_reverse", json!(false)),
    ] {
        map.entry(key).or_insert(value);
    }
    Ok(())
}
fn v1_to_v2(kind: Kind, value: &mut Value, path: &str) -> Result<()> {
    if kind == Kind::Content && value.is_array() {
        *value = json!({"version":1,"entries":std::mem::take(value)});
    }
    if kind == Kind::Output && value.get("version").is_none() {
        *value = json!({"version":1,"output":std::mem::take(value)});
    }
    let map = object(value, path)?;
    match kind {
        Kind::Project | Kind::Layout => {
            map.entry("dmx_models").or_insert(
                serde_json::to_value(crate::output_design::default_models())
                    .map_err(|e| e.to_string())?,
            );
            map.entry("dimming").or_insert(
                serde_json::to_value(crate::output_design::DimmingTimeline::default())
                    .map_err(|e| e.to_string())?,
            );
            if let Some(fixtures) = map.get_mut("fixtures").and_then(Value::as_array_mut) {
                for fixture in fixtures {
                    fixture_defaults(fixture, path)?;
                }
            }
            if kind == Kind::Project {
                map.entry("director").or_insert(json!([null, null, null]));
                if let Some(director) = map.get_mut("director") {
                    layers(director, path)?;
                }
                if let Some(content) = map.get_mut("content").and_then(Value::as_object_mut) {
                    for value in content.values_mut() {
                        layers(value, path)?;
                    }
                }
            }
        }
        Kind::Session => {
            map.entry("resources").or_insert(json!({}));
            map.entry("checkpoints").or_insert(json!({}));
            map.entry("inputs").or_insert(json!({}));
            map.entry("plugins")
                .or_insert(json!({"generation":0,"packages":{}}));
        }
        Kind::Content => {
            if let Some(entries) = map.get_mut("entries").and_then(Value::as_array_mut) {
                for entry in entries {
                    if let Some(value) = entry.get_mut("stack") {
                        stack(value, path)?;
                    }
                }
            }
        }
        Kind::Bundle | Kind::Output | Kind::ProjectManifest => {}
    }
    map.insert("version".into(), json!(2));
    Ok(())
}
struct Migration {
    kind: Kind,
    from: u32,
    to: u32,
    apply: fn(Kind, &mut Value, &str) -> Result<()>,
    description: &'static str,
}
const MIGRATIONS: &[Migration] = &[
    Migration {
        kind: Kind::Project,
        from: 1,
        to: 2,
        apply: v1_to_v2,
        description: "Freeze v1 fixture, layer and block defaults",
    },
    Migration {
        kind: Kind::Session,
        from: 1,
        to: 2,
        apply: v1_to_v2,
        description: "Version recovery metadata and migrate all retained projects",
    },
    Migration {
        kind: Kind::Bundle,
        from: 1,
        to: 2,
        apply: v1_to_v2,
        description: "Version portable assets and embedded project",
    },
    Migration {
        kind: Kind::Layout,
        from: 1,
        to: 2,
        apply: v1_to_v2,
        description: "Freeze legacy layout defaults",
    },
    Migration {
        kind: Kind::Content,
        from: 1,
        to: 2,
        apply: v1_to_v2,
        description: "Wrap legacy content entries in a versioned document",
    },
    Migration {
        kind: Kind::Output,
        from: 1,
        to: 2,
        apply: v1_to_v2,
        description: "Wrap legacy transport settings in a versioned document",
    },
    Migration {
        kind: Kind::ProjectManifest,
        from: 1,
        to: 2,
        apply: v1_to_v2,
        description: "Version immutable project-generation manifest",
    },
];
fn upgrade(kind: Kind, value: &mut Value, path: &str, report: &mut Report) -> Result<()> {
    let mut current = version(kind, value, path)?;
    while current < kind.current() {
        let step = MIGRATIONS
            .iter()
            .find(|step| step.kind == kind && step.from == current)
            .ok_or_else(|| format!("No {} migration from version {current}", kind.name()))?;
        (step.apply)(kind, value, path)?;
        if version(kind, value, path)? != step.to {
            return Err("Migration did not advance to its declared version".into());
        }
        report.steps.push(Step {
            format: kind,
            path: path.into(),
            from: step.from,
            to: step.to,
            description: step.description,
        });
        current = step.to;
    }
    match kind {
        Kind::Session => {
            let map = object(value, path)?;
            if let Some(project) = map.get_mut("project") {
                upgrade(Kind::Project, project, &format!("{path}.project"), report)?;
            }
            for field in ["undo", "redo"] {
                if let Some(values) = map.get_mut(field).and_then(Value::as_array_mut) {
                    for (i, project) in values.iter_mut().enumerate() {
                        upgrade(
                            Kind::Project,
                            project,
                            &format!("{path}.{field}[{i}]"),
                            report,
                        )?;
                    }
                }
            }
            if let Some(values) = map.get_mut("checkpoints").and_then(Value::as_object_mut) {
                for (name, project) in values {
                    upgrade(
                        Kind::Project,
                        project,
                        &format!("{path}.checkpoints[{name:?}]"),
                        report,
                    )?;
                }
            }
        }
        Kind::Bundle => {
            if let Some(project) = value.get_mut("project") {
                upgrade(Kind::Project, project, &format!("{path}.project"), report)?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// Validate the whole migrated document before a caller persists or installs it.
pub fn validated(kind: Kind, document: Value) -> Result<Migrated> {
    let mut migrated = migrate(kind, document)?;
    migrated.document = match kind {
        Kind::Project => {
            let project: crate::model::Project =
                serde_json::from_value(migrated.document).map_err(|e| e.to_string())?;
            project.validate()?;
            serde_json::to_value(project)
        }
        Kind::Session => {
            let session: crate::session::Session =
                serde_json::from_value(migrated.document).map_err(|e| e.to_string())?;
            session.validate()?;
            let _ = crate::engine::Engine::from_session(session.clone())?;
            serde_json::to_value(session)
        }
        Kind::Bundle => {
            let bundle: crate::bundle::Bundle =
                serde_json::from_value(migrated.document).map_err(|e| e.to_string())?;
            let registry =
                std::sync::Arc::new(crate::plugins::Registry::compile(bundle.plugins.clone())?);
            crate::scenic::with_plugins(registry, || {
                bundle.project.validate()?;
                crate::bundle::validate_resources(&bundle.resources)?;
                crate::bundle::merge_assets(&mut crate::inputs::Inputs::default(), &bundle.assets)
            })?;
            serde_json::to_value(bundle)
        }
        Kind::Layout => {
            let doc: crate::storage_formats::Layout =
                serde_json::from_value(migrated.document).map_err(|e| e.to_string())?;
            doc.validate()?;
            serde_json::to_value(doc)
        }
        Kind::Content => {
            let doc: crate::storage_formats::Content =
                serde_json::from_value(migrated.document).map_err(|e| e.to_string())?;
            doc.validate()?;
            serde_json::to_value(doc)
        }
        Kind::Output => {
            let doc: crate::storage_formats::Output =
                serde_json::from_value(migrated.document).map_err(|e| e.to_string())?;
            doc.validate()?;
            serde_json::to_value(doc)
        }
        Kind::ProjectManifest => {
            let doc: crate::storage_formats::Manifest =
                serde_json::from_value(migrated.document).map_err(|e| e.to_string())?;
            doc.validate()?;
            serde_json::to_value(doc)
        }
    }
    .map_err(|e| e.to_string())?;
    Ok(migrated)
}
/// Preserve exact source bytes before the first successful replacement. Never touch a source on load.
#[cfg(feature = "native")]
pub fn backup_existing(path: &std::path::Path, kind: Kind) -> Result<Option<std::path::PathBuf>> {
    use std::{
        fs,
        io::{Read, Write},
    };
    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.to_string()),
    };
    if file.metadata().map_err(|e| e.to_string())?.len() > MAX_BYTES as u64 {
        return Err("Existing storage document exceeds 64 MiB; refusing overwrite".into());
    }
    let mut original = Vec::new();
    file.take(MAX_BYTES as u64 + 1)
        .read_to_end(&mut original)
        .map_err(|e| e.to_string())?;
    let migrated = validated(kind, parse(&original)?)?;
    if !migrated.report.changed() {
        return Ok(None);
    }
    let directory = path
        .parent()
        .ok_or("Storage path has no parent")?
        .join(".migration-backups");
    if let Ok(meta) = fs::symlink_metadata(&directory) {
        if meta.file_type().is_symlink() || !meta.is_dir() {
            return Err("Migration backup directory must be a real directory".into());
        }
    }
    let mut builder = fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(&directory).map_err(|e| e.to_string())?;
    let filename = path
        .file_name()
        .ok_or("Storage filename missing")?
        .to_string_lossy();
    for index in 0..1000 {
        let backup = directory.join(format!(
            "{filename}.v{}-to-v{}.{}.json",
            migrated.report.steps[0].from,
            kind.current(),
            index
        ));
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        match options.open(&backup) {
            Ok(mut file) => {
                file.write_all(&original).map_err(|e| e.to_string())?;
                file.sync_all().map_err(|e| e.to_string())?;
                #[cfg(unix)]
                {
                    fs::File::open(&directory)
                        .and_then(|file| file.sync_all())
                        .map_err(|e| e.to_string())?;
                    fs::File::open(path.parent().unwrap())
                        .and_then(|file| file.sync_all())
                        .map_err(|e| e.to_string())?;
                }
                return Ok(Some(backup));
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                let meta = fs::symlink_metadata(&backup).map_err(|e| e.to_string())?;
                if meta.file_type().is_symlink() {
                    return Err("Migration backup cannot be a symbolic link".into());
                }
                if meta.len() == original.len() as u64
                    && fs::read(&backup).map_err(|e| e.to_string())? == original
                {
                    return Ok(Some(backup));
                }
            }
            Err(error) => return Err(error.to_string()),
        }
    }
    Err("Migration backup limit reached; no original was overwritten".into())
}

pub fn versions() -> Value {
    json!({"project":PROJECT_VERSION,"session":SESSION_VERSION,"bundle":BUNDLE_VERSION,"layout":LAYOUT_VERSION,"content":CONTENT_VERSION,"output":OUTPUT_VERSION,"manifest":MANIFEST_VERSION})
}

/// JSON map duplicate detection is required before Value would discard earlier fields.
pub fn deserialize_value<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Value, D::Error> {
    struct Document(Value);
    impl<'de> serde::Deserialize<'de> for Document {
        fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
            deserialize_value(d).map(Self)
        }
    }
    struct Visitor;
    impl<'de> serde::de::Visitor<'de> for Visitor {
        type Value = Value;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("JSON with unique object keys")
        }
        fn visit_unit<E: serde::de::Error>(self) -> std::result::Result<Value, E> {
            Ok(Value::Null)
        }
        fn visit_none<E: serde::de::Error>(self) -> std::result::Result<Value, E> {
            Ok(Value::Null)
        }
        fn visit_bool<E: serde::de::Error>(self, v: bool) -> std::result::Result<Value, E> {
            Ok(Value::Bool(v))
        }
        fn visit_i64<E: serde::de::Error>(self, v: i64) -> std::result::Result<Value, E> {
            Ok(Value::Number(v.into()))
        }
        fn visit_u64<E: serde::de::Error>(self, v: u64) -> std::result::Result<Value, E> {
            Ok(Value::Number(v.into()))
        }
        fn visit_f64<E: serde::de::Error>(self, v: f64) -> std::result::Result<Value, E> {
            serde_json::Number::from_f64(v)
                .map(Value::Number)
                .ok_or_else(|| E::custom("Non-finite storage number"))
        }
        fn visit_str<E: serde::de::Error>(self, v: &str) -> std::result::Result<Value, E> {
            Ok(Value::String(v.into()))
        }
        fn visit_string<E: serde::de::Error>(self, v: String) -> std::result::Result<Value, E> {
            Ok(Value::String(v))
        }
        fn visit_seq<A: serde::de::SeqAccess<'de>>(
            self,
            mut access: A,
        ) -> std::result::Result<Value, A::Error> {
            let mut values = Vec::new();
            while let Some(Document(value)) = access.next_element()? {
                values.push(value);
            }
            Ok(Value::Array(values))
        }
        fn visit_map<A: serde::de::MapAccess<'de>>(
            self,
            mut access: A,
        ) -> std::result::Result<Value, A::Error> {
            let mut values = Map::new();
            while let Some((key, Document(value))) = access.next_entry::<String, Document>()? {
                if values.insert(key.clone(), value).is_some() {
                    return Err(serde::de::Error::custom(format!(
                        "Duplicate storage key {key:?}"
                    )));
                }
            }
            Ok(Value::Object(values))
        }
    }
    deserializer.deserialize_any(Visitor)
}
pub fn optional_value<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> std::result::Result<Option<Value>, D::Error> {
    deserialize_value(d).map(Some)
}
