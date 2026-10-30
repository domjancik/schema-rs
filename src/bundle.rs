//! Portable authored data and bounded resources. Imports never extract files or load native code.
use crate::{
    inputs::Inputs,
    model::{Project, Result},
    plugins,
    session::Session,
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::Arc};
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Assets {
    pub samples: BTreeMap<String, Arc<crate::inputs::Audio>>,
    pub textures: BTreeMap<String, Arc<crate::inputs::Texture>>,
    pub stacks: BTreeMap<String, crate::model::Stack>,
    pub content: BTreeMap<String, Vec<crate::model::ContentEntry>>,
    pub sequences: BTreeMap<String, Vec<String>>,
}
impl Assets {
    pub fn from_inputs(i: &Inputs) -> Self {
        Self {
            samples: i.audio_samples.clone(),
            textures: i.textures.clone(),
            stacks: i.stacks.clone(),
            content: i.content.clone(),
            sequences: i.sequences.clone(),
        }
    }
}
crate::versioned_document! {crate::migrations::Kind::Bundle, BundleWire, "Bundle";
#[derive(Clone, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Bundle {
    pub version: u32,
    pub project: Project,
    pub plugins: plugins::State,
    pub assets: Assets,
    pub resources: BTreeMap<String, Vec<u8>>,
}
}
pub fn validate_resources(resources: &BTreeMap<String, Vec<u8>>) -> Result<()> {
    if resources.len() > 256 || resources.values().map(Vec::len).sum::<usize>() > 8 * 1024 * 1024 {
        return Err("Bundle resources exceed 256 files or 8 MiB".into());
    }
    for name in resources.keys() {
        if name.len() > 200
            || name.contains(['\\', ':'])
            || name.chars().any(char::is_control)
            || name
                .split('/')
                .any(|p| p.is_empty() || p == "." || p == "..")
        {
            return Err("Invalid portable resource path".into());
        }
    }
    Ok(())
}
pub fn merge_assets(inputs: &mut Inputs, assets: &Assets) -> Result<()> {
    macro_rules! merge {
        ($target:ident,$source:ident) => {
            for (name, value) in &assets.$source {
                if let Some(old) = inputs.$target.get(name) {
                    if serde_json::to_value(old).map_err(|e| e.to_string())?
                        != serde_json::to_value(value).map_err(|e| e.to_string())?
                    {
                        return Err(format!(
                            "Resource name conflict: {name}; rename before importing"
                        ));
                    }
                }
                inputs.$target.insert(name.clone(), value.clone());
            }
        };
    }
    merge!(audio_samples, samples);
    merge!(textures, textures);
    merge!(stacks, stacks);
    merge!(content, content);
    merge!(sequences, sequences);
    inputs.validate()
}
impl Bundle {
    pub fn from_session(s: &Session) -> Self {
        let mut project = (*s.project).clone();
        project.output.enabled = false;
        Self {
            version: crate::migrations::BUNDLE_VERSION,
            project,
            plugins: (*s.plugins.state).clone(),
            assets: Assets::from_inputs(&s.inputs),
            resources: (*s.resources).clone(),
        }
    }
    pub fn prepare(self, mut session: Session) -> Result<Session> {
        if self.version != crate::migrations::BUNDLE_VERSION {
            return Err("Unsupported portable bundle version".into());
        }
        validate_resources(&self.resources)?;
        let mut plugins = (*session.plugins.state).clone();
        for (id, package) in self.plugins.packages {
            if let Some(old) = plugins.packages.get(&id) {
                if old != &package {
                    return Err(format!("Component package conflict: {id}; import into a new project session or rename the package"));
                }
            }
            plugins.packages.insert(id, package);
        }
        plugins.generation = session
            .plugins
            .state
            .generation
            .checked_add(1)
            .ok_or("Plugin generation overflow")?;
        session.plugins = Arc::new(plugins::Registry::compile(plugins)?);
        crate::scenic::with_plugins(session.plugins.clone(), || {
            let mut inputs = (*session.inputs).clone();
            merge_assets(&mut inputs, &self.assets)?;
            let mut resources = (*session.resources).clone();
            for (name, bytes) in self.resources {
                if resources.get(&name).is_some_and(|old| old != &bytes) {
                    return Err(format!("Resource name conflict: {name}"));
                }
                inputs.files.insert(name.clone(), true);
                resources.insert(name, bytes);
            }
            validate_resources(&resources)?;
            let mut project = self.project;
            project.output.enabled = false;
            project.validate()?;
            session.undo.push(session.project.clone());
            if session.undo.len() > 128 {
                session.undo.remove(0);
            }
            session.redo.clear();
            session.states.clear();
            session.project = Arc::new(project);
            session.inputs = Arc::new(inputs);
            session.resources = Arc::new(resources);
            session.snapshot.revision += 1;
            session.snapshot.debug_focus = None;
            session.validate()?;
            Ok(session)
        })
    }
    #[cfg(feature = "native")]
    pub fn collect(&mut self, root: &std::path::Path) -> Result<()> {
        let directory = root.join("resources");
        if !directory.exists() {
            return Ok(());
        }
        if std::fs::symlink_metadata(&directory)
            .map_err(|e| e.to_string())?
            .file_type()
            .is_symlink()
        {
            return Err("Portable resources cannot be symbolic links".into());
        }
        let base = directory.canonicalize().map_err(|e| e.to_string())?;
        fn visit(bundle: &mut Bundle, base: &std::path::Path, dir: &std::path::Path) -> Result<()> {
            for entry in std::fs::read_dir(dir).map_err(|e| e.to_string())? {
                let entry = entry.map_err(|e| e.to_string())?;
                let path = entry.path();
                let meta = std::fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
                if meta.file_type().is_symlink() {
                    return Err("Portable resources cannot include symbolic links".into());
                }
                if !path
                    .canonicalize()
                    .map_err(|e| e.to_string())?
                    .starts_with(base)
                {
                    return Err("Resource escapes project directory".into());
                }
                if meta.is_dir() {
                    if path.components().count() - base.components().count() > 16 {
                        return Err("Resource directory is too deep".into());
                    }
                    visit(bundle, base, &path)?;
                } else if meta.is_file() {
                    if meta.len() > 8 * 1024 * 1024 {
                        return Err("Resource exceeds 8 MiB".into());
                    }
                    let name = path
                        .strip_prefix(base)
                        .map_err(|e| e.to_string())?
                        .to_string_lossy()
                        .replace('\\', "/");
                    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
                    if path
                        .extension()
                        .is_some_and(|e| e.eq_ignore_ascii_case("wav"))
                    {
                        bundle
                            .assets
                            .samples
                            .insert(name.clone(), Arc::new(crate::io::decode_wav(&bytes)?));
                    }
                    if bundle.resources.get(&name).is_some_and(|old| old != &bytes) {
                        return Err(format!("Resource name conflict: {name}; embedded and filesystem contents differ"));
                    }
                    bundle.resources.insert(name, bytes);
                    validate_resources(&bundle.resources)?;
                }
            }
            Ok(())
        }
        visit(self, &base, &base)
    }
}
