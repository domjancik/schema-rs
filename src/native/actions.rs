use super::{flag, num, providers::Context, text, timed::State};
use crate::{
    engine::Command,
    inputs::Event,
    model::{Bank, ContentEntry, Node, Stack},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Action {
    Assign {
        target: String,
        group: bool,
        director: bool,
        stack: String,
        layer: usize,
        transition: String,
        duration: f64,
    },
    Content {
        name: String,
        group: String,
        transition: String,
        duration: f64,
    },
    Global {
        slot: usize,
        primary: bool,
        from: [f64; 4],
        color: [f64; 4],
        start: f64,
        duration: f64,
    },
    Memorize {
        name: String,
        stack: Stack,
    },
    LoadMemory {
        fixture: String,
        layer: usize,
        node: u64,
        name: String,
    },
    Adopt {
        fixture: String,
        layer: usize,
        node: u64,
        child: u64,
    },
    Pixels {
        fixture: String,
        positions: Vec<[f64; 3]>,
    },
}
pub(crate) fn supported(key: &str) -> bool {
    matches!(
        key,
        "Assign.Content"
            | "Assign.Director"
            | "Assign.Fixture"
            | "Assign.Group"
            | "Assign.Global.Color"
            | "Global.Set.Color"
            | "Global.Set.Primary.FromColor"
            | "Modify.Memorize"
            | "Transition.Fade"
    )
}
pub(crate) fn blend(a: &Bank, b: &Bank, t: f64) -> Bank {
    let mut out = if t < 1. { a.clone() } else { b.clone() };
    let f = |a: f64, b: f64| a + (b - a) * t;
    if let (Some(a), Some(b)) = (a.value, b.value) {
        out.value = Some(f(a, b));
    }
    if let (Some(a), Some(b)) = (a.duration, b.duration) {
        out.duration = Some(f(a, b));
    }
    if let (Some(a), Some(b)) = (a.speed, b.speed) {
        out.speed = Some(f(a, b));
    }
    if let (Some(a), Some(b)) = (a.position, b.position) {
        out.position = Some(std::array::from_fn(|i| f(a[i], b[i])));
    }
    if let (Some(a), Some(b)) = (a.rotation, b.rotation) {
        out.rotation = Some(std::array::from_fn(|i| f(a[i], b[i])));
    }
    if let (Some(a), Some(b)) = (&a.colors, &b.colors) {
        if !a.is_empty() && !b.is_empty() {
            let len = a.len().max(b.len()).min(512);
            out.colors = Some(
                (0..len)
                    .map(|i| std::array::from_fn(|ch| f(a[i % a.len()][ch], b[i % b.len()][ch])))
                    .collect(),
            );
        }
    }
    out
}
pub(crate) fn evaluate(
    node: &Node,
    p: &BTreeMap<String, Value>,
    inputs: &[Bank],
    bank: &mut Bank,
    state: &mut State,
    c: &mut Context<'_>,
    layer: usize,
) -> bool {
    let key = node.behavior.as_str();
    let n = |key, default| num(p, key, default);
    let owner = &c.fixture.id;
    match key {
        "Assign.Group" | "Assign.Fixture" | "Assign.Director" => {
            if flag(p, "Enabled", false) {
                let group = key == "Assign.Group";
                let director = key == "Assign.Director";
                let target = if group {
                    text(p, "Group", "Group")
                } else {
                    text(p, "Fixture", "Select Fixture")
                }
                .to_owned();
                let layer = if key == "Assign.Fixture" {
                    0
                } else {
                    n("Layer", 0.).max(0.) as usize
                };
                let action = Action::Assign {
                    target: target.clone(),
                    group,
                    director,
                    stack: text(p, "Stack", "").to_owned(),
                    layer,
                    transition: text(p, "Transition", "Transition.Fade").to_owned(),
                    duration: n("Duration", 2.).max(0.),
                };
                bank.actions.insert(
                    format!(
                        "{}/{target}/{layer}",
                        if group {
                            "0/group"
                        } else if director {
                            "2/director"
                        } else {
                            "1/fixture"
                        }
                    ),
                    action,
                );
            }
        }
        "Assign.Content" => {
            if flag(p, "Enabled", false) {
                let name = text(p, "Content", "Content").to_owned();
                let group = text(p, "Group", "").to_owned();
                bank.actions.insert(
                    format!("0/content/{name}/{group}"),
                    Action::Content {
                        name,
                        group,
                        transition: text(p, "Transition", "Transition.Fade").to_owned(),
                        duration: n("Duration", 2.).max(0.),
                    },
                );
            }
        }
        "Assign.Global.Color" | "Global.Set.Color" | "Global.Set.Primary.FromColor" => {
            let primary = key == "Global.Set.Primary.FromColor";
            let slot = n("Slot", 0.).max(0.) as usize;
            let color = bank
                .colors
                .as_ref()
                .and_then(|v| v.first())
                .copied()
                .unwrap_or_else(|| {
                    crate::scenic::color(text(
                        p,
                        if primary { "Color" } else { "Default" },
                        "#ffffff",
                    ))
                    .unwrap_or([1.; 4])
                });
            if state.colors.len() != 2 || state.colors[1] != color {
                let from = if primary {
                    c.host.primary
                } else {
                    c.host.colors.get(&slot).copied()
                }
                .unwrap_or([0., 0., 0., 1.]);
                state.colors = vec![from, color];
                state.start = Some(c.time);
            }
            bank.actions.insert(
                format!("3/global/{primary}/{slot}"),
                Action::Global {
                    slot,
                    primary,
                    from: state.colors[0],
                    color,
                    start: state.start.unwrap_or(c.time),
                    duration: n("Fade Duration", 1.).max(0.),
                },
            );
            if key == "Assign.Global.Color" {
                bank.colors = None;
            }
        }
        "Modify.Memorize" => {
            let save = flag(p, "Save", false);
            let load = flag(p, "Load", false);
            let name = format!("{}:{}", text(p, "Name", "Memory"), n("Slot", 0.) as i64);
            if save && !state.latch {
                if let Some(child) = node.children.first() {
                    bank.actions.insert(
                        format!("4/memory/save/{name}"),
                        Action::Memorize {
                            name: name.clone(),
                            stack: Stack {
                                enabled: true,
                                name: name.clone(),
                                root: child.clone(),
                            },
                        },
                    );
                }
            }
            if load && !state.playing {
                bank.actions.insert(
                    format!("5/memory/load/{owner}/{layer}/{}", node.id),
                    Action::LoadMemory {
                        fixture: owner.clone(),
                        layer,
                        node: node.id,
                        name,
                    },
                );
            }
            state.latch = save;
            state.playing = load;
        }
        "Transition.Fade" => {
            if inputs.len() >= 2 {
                let start = *state.start.get_or_insert(c.time);
                let duration = n("Duration", 2.).max(0.);
                let t = if duration == 0. {
                    1.
                } else {
                    ((c.time - start) / duration).clamp(0., 1.)
                };
                *bank = blend(&inputs[0], &inputs[1], t);
                bank.progress = Some(t);
                if t >= 1. {
                    bank.actions.insert(
                        format!("6/adopt/{owner}/{layer}/{}", node.id),
                        Action::Adopt {
                            fixture: owner.clone(),
                            layer,
                            node: node.id,
                            child: node.children[1].id,
                        },
                    );
                }
            }
        }
        _ => return false,
    }
    true
}
fn transitioned(
    previous: Option<&Stack>,
    mut next: Stack,
    transition: &str,
    duration: f64,
) -> Stack {
    if duration > 0. && transition == "Transition.Fade" {
        if let Some(previous) = previous {
            let mut root = Node {
                bypassed: false,
                id: 1,
                behavior: "Transition.Fade".into(),
                properties: BTreeMap::from([("Duration".into(), serde_json::json!(duration))]),
                children: vec![previous.root.clone(), next.root],
                drivers: BTreeMap::new(),
            };
            root.reidentify(&mut 1);
            next.root = root;
        }
    }
    next
}
pub(crate) struct Resolution {
    pub commands: Vec<Command>,
    pub internal: Vec<Command>,
    pub events: Vec<Event>,
    pub errors: Vec<String>,
}
pub(crate) fn resolve(frame: &crate::runtime::Published) -> Resolution {
    let mut result = Resolution {
        commands: vec![],
        internal: vec![],
        events: vec![],
        errors: vec![],
    };
    let mut targets: BTreeMap<(String, usize), (Option<Stack>, String, f64)> = BTreeMap::new();
    for action in frame.snapshot.actions.values() {
        match action {
            Action::Assign {
                target,
                group,
                director,
                stack,
                layer,
                transition,
                duration,
            } => {
                if *layer >= crate::model::MAX_LAYERS {
                    result.errors.push("Assignment layer must be 0..31".into());
                    continue;
                }
                if let Some(stack) = frame.host.stacks.get(stack) {
                    if *director {
                        targets.insert(
                            ("__director".into(), *layer),
                            (Some(stack.clone()), transition.clone(), *duration),
                        );
                    } else {
                        for fixture in &frame.project.fixtures {
                            if *group
                                && (target.is_empty() || fixture.groups.iter().any(|g| g == target))
                                || !*group && (fixture.id == *target || fixture.name == *target)
                            {
                                targets.insert(
                                    (fixture.id.clone(), *layer),
                                    (Some(stack.clone()), transition.clone(), *duration),
                                );
                            }
                        }
                    }
                } else {
                    result
                        .errors
                        .push(format!("Stack registry entry '{stack}' missing"));
                }
            }
            Action::Content {
                name,
                group,
                transition,
                duration,
            } => {
                if let Some(entries) = frame.host.content.get(name) {
                    for entry in entries {
                        if group.is_empty()
                            || frame.project.fixtures.iter().any(|f| {
                                f.id == entry.fixture && f.groups.iter().any(|g| g == group)
                            })
                        {
                            targets.insert(
                                (entry.fixture.clone(), entry.layer),
                                (entry.stack.clone(), transition.clone(), *duration),
                            );
                        }
                    }
                } else {
                    result
                        .errors
                        .push(format!("Content registry entry '{name}' missing"));
                }
            }
            Action::Global {
                slot,
                primary,
                from,
                color,
                start,
                duration,
            } => {
                let t = if *duration <= 0. {
                    1.
                } else {
                    ((frame.snapshot.time - start) / duration).clamp(0., 1.)
                };
                let color = std::array::from_fn(|i| from[i] + (color[i] - from[i]) * t);
                if *primary {
                    if frame.host.primary != Some(color) {
                        result.events.push(Event::Primary { color });
                    }
                } else if frame.host.colors.get(slot) != Some(&color) {
                    result.events.push(Event::Color { slot: *slot, color });
                }
            }
            Action::Memorize { name, stack } => result.events.push(Event::Stack {
                name: name.clone(),
                stack: stack.clone(),
            }),
            Action::LoadMemory {
                fixture,
                layer,
                node,
                name,
            } => {
                if let Some(stack) = frame.host.stacks.get(name) {
                    result.internal.push(Command::ReplaceChildren {
                        fixture: fixture.clone(),
                        layer: *layer,
                        node: *node,
                        children: vec![stack.root.clone()],
                    });
                } else {
                    result.errors.push(format!("Memory '{name}' missing"));
                }
            }
            Action::Adopt {
                fixture,
                layer,
                node,
                child,
            } => result.internal.push(Command::AdoptNode {
                fixture: fixture.clone(),
                layer: *layer,
                node: *node,
                child: *child,
            }),
            Action::Pixels { fixture, positions } => {
                if let Some(current) = frame.project.fixtures.iter().find(|f| f.id == *fixture) {
                    if current.pixel_positions != *positions {
                        let mut fixture = current.clone();
                        fixture.pixel_positions = positions.clone();
                        result.internal.push(Command::SetFixture { fixture });
                    }
                }
            }
        }
    }
    let mut entries = vec![];
    for ((fixture, layer), (stack, transition, duration)) in targets {
        let previous = if fixture == "__director" {
            frame.project.director.get(layer).and_then(Option::as_ref)
        } else {
            frame
                .project
                .content
                .get(&fixture)
                .and_then(|layers| layers.get(layer))
                .and_then(Option::as_ref)
        };
        if previous.map(|s| &s.name) == stack.as_ref().map(|s| &s.name) {
            continue;
        }
        let stack = stack.map(|next| transitioned(previous, next, &transition, duration));
        entries.push(ContentEntry {
            fixture,
            layer,
            stack,
        });
    }
    if !entries.is_empty() {
        result.commands.push(Command::ApplyContent { entries });
    }
    result
}
