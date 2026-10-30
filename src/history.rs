//! Read-only presentation of the engine-owned undo/redo timeline.
use crate::{model::Project, session::Session};
use serde_json::{json, Value};
fn describe(a: &Project, b: &Project) -> String {
    if a.name != b.name {
        return format!("Project: {}", b.name);
    }
    if a.fixtures.len() != b.fixtures.len() {
        return "Add or remove fixtures".into();
    }
    if a.output != b.output {
        return "Output settings".into();
    }
    if a.dimming != b.dimming {
        return "Daily dimming".into();
    }
    if a.dmx_models != b.dmx_models {
        return "DMX models".into();
    }
    for (old, new) in a.fixtures.iter().zip(&b.fixtures) {
        if old.position != new.position {
            return format!("Move {}", new.name);
        }
        if old != new {
            return format!("Edit {}", new.name);
        }
    }
    for fixture in &b.fixtures {
        if a.content.get(&fixture.id) != b.content.get(&fixture.id) {
            return format!("Edit program · {}", fixture.name);
        }
    }
    if a.director != b.director {
        return "Edit Director".into();
    }
    "Project edit".into()
}
pub fn view(session: &Session) -> Value {
    let mut projects = session.undo.clone();
    let current = projects.len();
    projects.push(session.project.clone());
    projects.extend(session.redo.iter().rev().cloned());
    let entries=projects.iter().enumerate().map(|(i,p)|json!({"offset":i as i32-current as i32,"current":i==current,"label":if i==0{"Initial retained state".into()}else{describe(&projects[i-1],p)},"fixtures":p.fixtures.len()})).collect::<Vec<_>>();
    json!({"revision":session.snapshot.revision,"entries":entries,"checkpoints":session.checkpoints.iter().map(|(name,p)|json!({"name":name,"project":p.name,"fixtures":p.fixtures.len()})).collect::<Vec<_>>()})
}
