//! Versioned split-project documents, shared by the native and browser hosts.
use crate::model::{ContentEntry, Fixture, OutputConfig, Project, Result};
use serde::Serialize;
use std::collections::BTreeMap;
crate::versioned_document! {crate::migrations::Kind::Layout,LayoutWire,"Layout";
#[derive(Clone,Serialize)]
#[serde(deny_unknown_fields)]
pub struct Layout{
 #[serde(default="crate::output_design::default_models")]
 pub dmx_models:BTreeMap<String,crate::output_design::ModelDefinition>,
 #[serde(default)]pub dimming:crate::output_design::DimmingTimeline,
 pub version:u32,pub name:String,pub fixtures:Vec<Fixture>,
}}
crate::versioned_document! {crate::migrations::Kind::Content,ContentWire,"Content";
#[derive(Clone,Serialize)]
#[serde(deny_unknown_fields)]
pub struct Content{pub version:u32,pub entries:Vec<ContentEntry>}}
crate::versioned_document! {crate::migrations::Kind::Output,OutputWire,"Output";
#[derive(Clone,Serialize)]
#[serde(deny_unknown_fields)]
pub struct Output{pub version:u32,pub output:OutputConfig}}
#[derive(Clone, Debug, Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectEntry {
    pub id: String,
    pub title: String,
    pub fixtures: usize,
    pub saved_revision: u64,
    pub modified_unix_ms: u64,
    pub legacy: bool,
}
crate::versioned_document! {crate::migrations::Kind::ProjectManifest,ManifestWire,"Manifest";
#[derive(Clone,Serialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest{pub version:u32,pub generation:String,pub entry:ProjectEntry}}
impl Layout {
    pub fn validate(&self) -> Result<()> {
        let mut project = Project::demo();
        project.version = crate::migrations::PROJECT_VERSION;
        project.name = self.name.clone();
        project.fixtures = self.fixtures.clone();
        project.dmx_models = self.dmx_models.clone();
        project.dimming = self.dimming.clone();
        project.content = project
            .fixtures
            .iter()
            .map(|f| (f.id.clone(), vec![None, None, None]))
            .collect();
        project.director = vec![None, None, None];
        project.validate()
    }
}
impl Content {
    pub fn validate(&self) -> Result<()> {
        let mut targets = std::collections::HashSet::new();
        for entry in &self.entries {
            if entry.layer >= crate::model::MAX_LAYERS
                || !targets.insert((&entry.fixture, entry.layer))
            {
                return Err("Invalid or duplicate content target".into());
            }
            if let Some(stack) = &entry.stack {
                crate::scenic::validate(stack)?;
            }
        }
        Ok(())
    }
}
impl Output {
    pub fn validate(&self) -> Result<()> {
        let mut project = Project::demo();
        project.output = self.output.clone();
        project.validate()
    }
}
impl Manifest {
    pub fn validate(&self) -> Result<()> {
        let slug = |text: &str| max_slug(text);
        if !slug(&self.entry.id)
            || !slug(&self.generation)
            || self.entry.legacy
            || self.entry.title.trim().is_empty()
            || self.entry.title.len() > 200
            || self.entry.title.chars().any(char::is_control)
            || self.entry.fixtures > 512
        {
            return Err("Invalid saved project manifest".into());
        }
        Ok(())
    }
}
fn max_slug(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 80
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
}
