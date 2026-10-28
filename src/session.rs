use crate::{
    engine::NodeState,
    model::{Project, Result, Snapshot},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::{self, File},
    io::{BufWriter, Read, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
};
const MAX_BYTES: u64 = 64 * 1024 * 1024;
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Session {
    #[serde(default)]
    pub plugins: Arc<crate::plugins::Registry>,
    #[serde(default)]
    pub inputs: std::sync::Arc<crate::inputs::Inputs>,
    pub version: u32,
    pub project: Arc<Project>,
    pub snapshot: Snapshot,
    pub undo: Vec<Arc<Project>>,
    pub redo: Vec<Arc<Project>>,
    pub(crate) states: HashMap<String, NodeState>,
}
impl Session {
    pub fn disarm_output(&mut self) {
        Arc::make_mut(&mut self.project).output.enabled = false;
        for project in self.undo.iter_mut().chain(&mut self.redo) {
            Arc::make_mut(project).output.enabled = false;
        }
    }

    pub fn validate(&self) -> Result<()> {
        crate::scenic::with_plugins(self.plugins.clone(), || self.validate_inner())
    }
    fn validate_inner(&self) -> Result<()> {
        if self.version != 1 {
            return Err("Unsupported session version".into());
        }
        if self.undo.len() > 128 || self.redo.len() > 128 || self.undo.len() + self.redo.len() > 128
        {
            return Err("Session history exceeds 128 transactions".into());
        }
        if !self.snapshot.time.is_finite()
            || self.snapshot.time < 0.
            || self.snapshot.revision == u64::MAX
            || self.snapshot.frame == u64::MAX
        {
            return Err("Invalid session clock/revision".into());
        }
        self.inputs.validate()?;
        self.project.validate()?;
        for p in self.undo.iter().chain(&self.redo) {
            p.validate()?;
        }
        if self.states.len() > (self.project.fixtures.len() + 1) * crate::model::MAX_LAYERS * 4096 {
            return Err("Session runtime state exceeds node limit".into());
        }
        let mut keys = std::collections::HashMap::new();
        fn walk<'a>(
            n: &'a crate::model::Node,
            prefix: &str,
            keys: &mut std::collections::HashMap<String, &'a crate::model::Node>,
            plugins: &'a crate::plugins::Registry,
            prefixes: &std::collections::HashSet<String>,
        ) {
            keys.insert(format!("{prefix}/{}", n.id), n);
            for child in n.children.iter().chain(n.drivers.values()) {
                walk(child, prefix, keys, plugins, prefixes);
            }
            if let Some(component) = plugins.component(&n.behavior) {
                let virtual_prefix = format!("{prefix}/{}@plugin/{}", n.id, component.key);
                if prefixes.contains(&virtual_prefix) {
                    walk(&component.root, &virtual_prefix, keys, plugins, prefixes);
                }
            }
        }
        let mut prefixes = std::collections::HashSet::new();
        for key in self.states.keys() {
            for (index, _) in key.match_indices("@plugin/") {
                let start = index + 8;
                if let Some(end) = key[start..].find('/') {
                    prefixes.insert(key[..start + end].to_owned());
                }
            }
        }
        let director_key = "__director".to_owned();
        for (fixture, layers) in self
            .project
            .content
            .iter()
            .chain(std::iter::once((&director_key, &self.project.director)))
        {
            for (layer, stack) in layers.iter().enumerate() {
                if let Some(stack) = stack {
                    walk(
                        &stack.root,
                        &format!("{fixture}/{layer}"),
                        &mut keys,
                        &self.plugins,
                        &prefixes,
                    );
                }
            }
        }
        if self.states.keys().any(|key| !keys.contains_key(key)) {
            return Err("Session runtime state references a missing node".into());
        }
        for (key, state) in &self.states {
            state.validate_for(keys[key], self.snapshot.time)?;
        }
        Ok(())
    }
}
pub fn load(path: &Path) -> Result<Session> {
    let file = File::open(path).map_err(|e| e.to_string())?;
    if file.metadata().map_err(|e| e.to_string())?.len() > MAX_BYTES {
        return Err("Session exceeds 64 MiB".into());
    }
    let mut bytes = Vec::new();
    file.take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err("Session exceeds 64 MiB".into());
    }
    // Model depth is bounded at 64; account for each node's object/children-array
    // plus session/project wrappers before disabling serde's smaller generic limit.
    let (mut depth, mut quoted, mut escaped) = (0_u32, false, false);
    for byte in &bytes {
        if quoted {
            if escaped {
                escaped = false;
            } else if *byte == b'\\' {
                escaped = true;
            } else if *byte == b'"' {
                quoted = false;
            }
        } else {
            match byte {
                b'"' => quoted = true,
                b'{' | b'[' => {
                    depth += 1;
                    if depth > 160 {
                        return Err("Session JSON nesting exceeds 160 levels".into());
                    }
                }
                b'}' | b']' => depth = depth.saturating_sub(1),
                _ => {}
            }
        }
    }
    let mut parser = serde_json::Deserializer::from_slice(&bytes);
    parser.disable_recursion_limit();
    let session = Session::deserialize(&mut parser).map_err(|e| format!("Session format: {e}"))?;
    parser.end().map_err(|e| format!("Session format: {e}"))?;
    session.validate()?;
    Ok(session)
}
#[derive(Clone, Default, Serialize)]
pub struct Status {
    pub enabled: bool,
    pub saved_revision: Option<u64>,
    pub saved_frame: Option<u64>,
    pub error: Option<String>,
}
struct State {
    status: Status,
}
pub struct Store {
    path: PathBuf,
    state: Mutex<State>,
}
impl Store {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            state: Mutex::new(State {
                status: Status {
                    enabled: true,
                    ..Status::default()
                },
            }),
        }
    }
    pub fn status(&self) -> Status {
        self.state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .status
            .clone()
    }
    pub fn save(&self, session: &Session) -> Result<()> {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if state
            .status
            .saved_revision
            .zip(state.status.saved_frame)
            .is_some_and(|saved| saved > (session.snapshot.revision, session.snapshot.frame))
        {
            return Ok(());
        }
        let result = session.validate().and_then(|_| self.write(session));
        state.status.error = result.as_ref().err().cloned();
        if result.is_ok() {
            state.status.saved_revision = Some(session.snapshot.revision);
            state.status.saved_frame = Some(session.snapshot.frame);
        }
        result
    }
    fn write(&self, session: &Session) -> Result<()> {
        let parent = self.path.parent().ok_or("Session path has no parent")?;
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let temp = parent.join(format!(
            ".session-{}-{}.tmp",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        struct Limited<W> {
            writer: W,
            bytes: u64,
        }
        impl<W: Write> Write for Limited<W> {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                if self.bytes + buf.len() as u64 > MAX_BYTES {
                    return Err(std::io::Error::other("Session exceeds 64 MiB"));
                }
                let n = self.writer.write(buf)?;
                self.bytes += n as u64;
                Ok(n)
            }
            fn flush(&mut self) -> std::io::Result<()> {
                self.writer.flush()
            }
        }
        let result = (|| {
            let file = File::create(&temp).map_err(|e| e.to_string())?;
            let mut writer = BufWriter::new(Limited {
                writer: file,
                bytes: 0,
            });
            serde_json::to_writer(&mut writer, session).map_err(|e| e.to_string())?;
            writer.flush().map_err(|e| e.to_string())?;
            writer
                .get_ref()
                .writer
                .sync_all()
                .map_err(|e| e.to_string())?;
            drop(writer);
            fs::rename(&temp, &self.path).map_err(|e| e.to_string())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temp);
        }
        result
    }
}
