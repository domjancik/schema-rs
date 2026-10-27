//! Versioned, isolated native graph component packages. No filesystem access on evaluation.
use crate::model::{Node, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct State {
    pub generation: u64,
    pub packages: BTreeMap<String, Package>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Package {
    pub folder: String,
    pub enabled: bool,
    pub manifest: Manifest,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub api_version: u32,
    pub id: String,
    pub version: String,
    pub components: Vec<Component>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Component {
    pub name: String,
    pub description: String,
    #[serde(default = "category")]
    pub category: String,
    #[serde(default = "domain")]
    pub domain: String,
    #[serde(default)]
    pub defaults: BTreeMap<String, Value>,
    pub graph: Value,
}
fn category() -> String {
    "Plugin".into()
}
fn domain() -> String {
    "Value".into()
}
pub type Bindings = BTreeMap<u64, BTreeMap<String, String>>;
#[derive(Clone)]
pub struct Compiled {
    pub key: String,
    pub enabled: bool,
    pub root: Arc<Node>,
    pub bindings: Arc<Bindings>,
}
#[derive(Clone, Default)]
pub struct Registry {
    pub state: Arc<State>,
    compiled: BTreeMap<String, Compiled>,
}
impl Serialize for Registry {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        self.state.serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for Registry {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        Self::compile(State::deserialize(d)?).map_err(serde::de::Error::custom)
    }
}
pub fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}
impl Registry {
    pub fn resource_names(&self, project: &crate::model::Project) -> BTreeSet<String> {
        fn visit(
            node: &Node,
            registry: &Registry,
            bindings: Option<&Bindings>,
            params: &BTreeMap<String, Value>,
            names: &mut BTreeSet<String>,
            budget: &mut usize,
        ) {
            if *budget == 0 {
                return;
            }
            *budget -= 1;
            let mut props = node.properties.clone();
            if let Some(values) = bindings.and_then(|b| b.get(&node.id)) {
                for (key, name) in values {
                    if let Some(value) = params.get(name) {
                        props.insert(key.clone(), value.clone());
                    }
                }
            }
            for key in ["File", "Resource"] {
                if let Some(name) = props.get(key).and_then(Value::as_str) {
                    names.insert(name.into());
                }
            }
            for child in node.children.iter().chain(node.drivers.values()) {
                visit(child, registry, bindings, params, names, budget);
            }
            if let Some(component) = registry.component(&node.behavior).filter(|c| c.enabled) {
                let mut parameters = registry.defaults(&node.behavior).unwrap_or_default();
                parameters.extend(props);
                visit(
                    &component.root,
                    registry,
                    Some(&component.bindings),
                    &parameters,
                    names,
                    budget,
                );
            }
        }
        let mut names = BTreeSet::new();
        for stack in project
            .content
            .values()
            .chain(std::iter::once(&project.director))
            .flatten()
            .flatten()
        {
            visit(
                &stack.root,
                self,
                None,
                &BTreeMap::new(),
                &mut names,
                &mut 16384,
            );
        }
        names
    }
    pub fn component(&self, key: &str) -> Option<&Compiled> {
        self.compiled.get(key)
    }
    pub fn components(&self) -> impl Iterator<Item = &Compiled> {
        self.compiled.values()
    }
    pub fn defaults(&self, key: &str) -> Option<BTreeMap<String, Value>> {
        let (id, name) = key.strip_prefix("Plugin.")?.split_once('.')?;
        self.state
            .packages
            .get(id)?
            .manifest
            .components
            .iter()
            .find(|component| component.name == name)
            .map(|component| component.defaults.clone())
    }
    pub fn load_json(&self, folder: &str, text: &str) -> Result<Self> {
        if !valid_name(folder) || text.len() > 1024 * 1024 {
            return Err("Invalid plugin folder or manifest exceeds 1 MiB".into());
        }
        let manifest: Manifest =
            serde_json::from_str(text).map_err(|e| format!("Plugin manifest: {e}"))?;
        let mut state = (*self.state).clone();
        if state
            .packages
            .values()
            .any(|p| p.folder == folder && p.manifest.id != manifest.id)
            || state
                .packages
                .get(&manifest.id)
                .is_some_and(|p| p.folder != folder)
        {
            return Err(
                "Plugin ID/folder identity changed or collides with another package".into(),
            );
        }
        state.generation = state
            .generation
            .checked_add(1)
            .ok_or("Plugin generation limit")?;
        state.packages.insert(
            manifest.id.clone(),
            Package {
                folder: folder.into(),
                enabled: true,
                manifest,
            },
        );
        Self::compile(state)
    }
    pub fn set_enabled(&self, id: &str, enabled: bool) -> Result<Self> {
        let mut state = (*self.state).clone();
        state.packages.get_mut(id).ok_or("Unknown plugin")?.enabled = enabled;
        state.generation = state
            .generation
            .checked_add(1)
            .ok_or("Plugin generation limit")?;
        Self::compile(state)
    }
    pub fn catalog(&self) -> Vec<Value> {
        let mut rows = vec![];
        for p in self.state.packages.values().filter(|p| p.enabled) {
            for c in &p.manifest.components {
                let key = format!("Plugin.{}.{}", p.manifest.id, c.name);
                rows.push(json!({"key":key,"name":c.name,"category":c.category,"domain":c.domain,"summary":c.description,"description":c.description,"defaults":c.defaults,"extension":true,"plugin":p.manifest.id,"plugin_version":p.manifest.version,"keywords":[p.manifest.id,c.name,"component","plugin"],"icon":"icons/plugin-component.svg","inputs":"Incoming child channels via Plugin.Input","output":c.domain}));
            }
        }
        rows
    }
    pub fn compile(state: State) -> Result<Self> {
        if state.packages.len() > 64
            || serde_json::to_vec(&state).map_err(|e| e.to_string())?.len() > 4 * 1024 * 1024
        {
            return Err("Plugin registry exceeds package/4 MiB budget".into());
        }
        let provisional = Arc::new(Self {
            state: Arc::new(state),
            compiled: BTreeMap::new(),
        });
        let mut compiled = BTreeMap::new();
        let mut count = 0;
        for (id, package) in &provisional.state.packages {
            let m = &package.manifest;
            if id != &m.id
                || !valid_name(id)
                || !valid_name(&package.folder)
                || m.api_version != 1
                || m.version.is_empty()
                || m.version.len() > 80
                || m.components.is_empty()
            {
                return Err("Invalid plugin identity, API version or empty components".into());
            }
            for c in &m.components {
                count += 1;
                if count > 256
                    || !valid_name(&c.name)
                    || c.description.is_empty()
                    || c.description.len() > 2048
                    || c.category.len() > 80
                    || c.domain.len() > 80
                    || c.defaults.len() > 64
                {
                    return Err("Invalid component metadata or component limit".into());
                }
                for (k, v) in &c.defaults {
                    if k.is_empty()
                        || k.len() > 80
                        || !(v.is_boolean()
                            || v.as_f64().is_some_and(|v| v.is_finite() && v.abs() <= 1e9)
                            || v.as_str().is_some_and(|s| s.len() <= 4096)
                            || crate::scenic::vector(v).is_ok())
                    {
                        return Err(format!("Invalid plugin parameter {k}"));
                    }
                }
                let mut graph = c.graph.clone();
                resolve(&mut graph, &c.defaults)?;
                let text = serde_json::to_string(&json!({"name":c.name,"stack":graph}))
                    .map_err(|e| e.to_string())?;
                let stack = crate::scenic::with_plugins(provisional.clone(), || {
                    crate::scenic::parse(&text)
                })?;
                fn structural(n: &Node) -> bool {
                    matches!(n.behavior.as_str(), "Transition.Fade" | "Modify.Memorize")
                        || n.children.iter().chain(n.drivers.values()).any(structural)
                }
                if structural(&stack.root) {
                    return Err("Structural node operations belong in authored stacks, not component templates".into());
                }
                let mut bindings = Bindings::new();
                bind(&c.graph, &stack.root, &c.defaults, &mut bindings)?;
                let key = format!("Plugin.{id}.{}", c.name);
                if compiled
                    .insert(
                        key.clone(),
                        Compiled {
                            key,
                            enabled: package.enabled,
                            root: Arc::new(stack.root),
                            bindings: Arc::new(bindings),
                        },
                    )
                    .is_some()
                {
                    return Err("Duplicate component key".into());
                }
            }
        }
        let registry = Self {
            state: provisional.state.clone(),
            compiled,
        };
        fn expanded(
            key: &str,
            r: &Registry,
            visiting: &mut BTreeSet<String>,
            depth: usize,
        ) -> Result<usize> {
            if depth > 8 || !visiting.insert(key.into()) {
                return Err("Plugin component cycle or nesting exceeds 8".into());
            }
            let c = r.component(key).ok_or("Missing component dependency")?;
            fn nodes(
                n: &Node,
                r: &Registry,
                visiting: &mut BTreeSet<String>,
                depth: usize,
                active: bool,
            ) -> Result<usize> {
                let mut size = 1;
                if n.behavior.starts_with("Plugin.") && n.behavior != "Plugin.Input" {
                    let dependency = r
                        .component(&n.behavior)
                        .ok_or("Missing plugin component dependency")?;
                    if active && !dependency.enabled {
                        return Err(format!(
                            "Enabled component depends on unloaded {}",
                            n.behavior
                        ));
                    }
                    size += expanded(&n.behavior, r, visiting, depth + 1)?;
                }
                for child in n.children.iter().chain(n.drivers.values()) {
                    size += nodes(child, r, visiting, depth, active)?;
                    if size > 4096 {
                        return Err("Expanded plugin graph exceeds 4096 nodes".into());
                    }
                }
                if size > 4096 {
                    return Err("Expanded plugin graph exceeds 4096 nodes".into());
                }
                Ok(size)
            }
            let size = nodes(&c.root, r, visiting, depth, c.enabled)?;
            visiting.remove(key);
            Ok(size)
        }
        for key in registry.compiled.keys() {
            expanded(key, &registry, &mut BTreeSet::new(), 0)?;
        }
        Ok(registry)
    }
}
fn resolve(value: &mut Value, parameters: &BTreeMap<String, Value>) -> Result<()> {
    match value {
        Value::String(text) if text.starts_with("$$") => *text = text[1..].into(),
        Value::String(text) if text.starts_with('$') => {
            *value = parameters
                .get(&text[1..])
                .cloned()
                .ok_or_else(|| format!("Unknown component parameter {text}"))?;
        }
        Value::Array(items) => {
            for value in items {
                resolve(value, parameters)?;
            }
        }
        Value::Object(map) => {
            for value in map.values_mut() {
                resolve(value, parameters)?;
            }
        }
        _ => {}
    }
    Ok(())
}
fn bind(
    raw: &Value,
    node: &Node,
    parameters: &BTreeMap<String, Value>,
    bindings: &mut Bindings,
) -> Result<()> {
    if raw.is_string() {
        return Ok(());
    }
    let fields = raw.as_object().and_then(|m| m.values().next());
    if let Some(fields) = fields.and_then(Value::as_object) {
        for (key, value) in fields {
            if key == "in" {
                for (raw, child) in value
                    .as_array()
                    .ok_or("Expected component inputs")?
                    .iter()
                    .zip(&node.children)
                {
                    bind(raw, child, parameters, bindings)?;
                }
            } else if let Some(text) = value
                .as_str()
                .filter(|s| s.starts_with('$') && !s.starts_with("$$"))
            {
                if key == "Code" || key == "Expression" {
                    return Err("Component code cannot be a dynamically bound parameter; use params in a static Script.Value".into());
                }
                if !parameters.contains_key(&text[1..]) {
                    return Err("Unknown component binding".into());
                }
                bindings
                    .entry(node.id)
                    .or_default()
                    .insert(key.clone(), text[1..].into());
            } else if let Some(driver) = node.drivers.get(key) {
                bind(value, driver, parameters, bindings)?;
            } else if value.is_array() {
                fn placeholder(v: &Value) -> bool {
                    match v {
                        Value::String(s) => s.starts_with('$') && !s.starts_with("$$"),
                        Value::Array(a) => a.iter().any(placeholder),
                        _ => false,
                    }
                }
                if placeholder(value) {
                    return Err("Bind a complete vector property using $Name; partial vector bindings are unsupported".into());
                }
            }
        }
    }
    Ok(())
}
