//! Bounded, ephemeral runtime event journal. Never serializes commands, input payloads or credentials.
use crate::{clock::Instant, model::Snapshot};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, VecDeque};
const CAPACITY: usize = 512;
#[derive(Clone, Debug, Serialize)]
pub struct Entry {
    pub id: u64,
    pub elapsed_ms: u64,
    pub level: &'static str,
    pub source: &'static str,
    pub message: String,
}
#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Query {
    pub after: u64,
    pub limit: usize,
}
#[derive(Serialize)]
pub struct Page {
    pub generation: String,
    pub entries: Vec<Entry>,
    pub next: u64,
    pub latest: u64,
    pub truncated: bool,
    pub reset: bool,
}
pub struct Journal {
    started: Instant,
    entries: VecDeque<Entry>,
    sequence: u64,
    diagnostics: BTreeSet<String>,
    output: Option<String>,
    controls: Option<(bool, bool)>,
    revision: Option<u64>,
}
impl Default for Journal {
    fn default() -> Self {
        Self::new()
    }
}
impl Journal {
    pub fn new() -> Self {
        let mut log = Self {
            started: Instant::now(),
            entries: VecDeque::new(),
            sequence: 0,
            diagnostics: BTreeSet::new(),
            output: None,
            controls: None,
            revision: None,
        };
        log.push("info", "system", "Schéma engine started");
        log
    }
    fn push(&mut self, level: &'static str, source: &'static str, message: &str) {
        self.sequence += 1;
        if self.entries.len() == CAPACITY {
            self.entries.pop_front();
        }
        self.entries.push_back(Entry {
            id: self.sequence,
            elapsed_ms: self.started.elapsed().as_millis().min(u64::MAX as u128) as u64,
            level,
            source,
            message: message
                .chars()
                .take(1024)
                .map(|c| if c.is_control() { ' ' } else { c })
                .collect(),
        });
    }
    pub fn record(&mut self, level: &'static str, source: &'static str, message: &str) {
        self.push(level, source, message);
    }
    pub fn observe(&mut self, snapshot: &Snapshot) {
        let diagnostics: BTreeSet<String> = snapshot
            .diagnostics
            .iter()
            .take(128)
            .map(|m| m.chars().take(1024).collect())
            .collect();
        for message in diagnostics
            .difference(&self.diagnostics)
            .cloned()
            .collect::<Vec<_>>()
        {
            self.push("warning", "evaluation", &message);
        }
        for message in self
            .diagnostics
            .difference(&diagnostics)
            .cloned()
            .collect::<Vec<_>>()
        {
            self.push(
                "info",
                "evaluation",
                &format!("Diagnostic cleared: {message}"),
            );
        }
        self.diagnostics = diagnostics;
        if self.output.as_ref() != Some(&snapshot.output_status) {
            let lower = snapshot.output_status.to_ascii_lowercase();
            let level = if lower.contains("error") || lower.contains("failed") {
                "error"
            } else {
                "info"
            };
            if !snapshot.output_status.trim().is_empty() {
                self.push(level, "output", &snapshot.output_status);
            }
            self.output = Some(snapshot.output_status.clone());
        }
        let controls = (snapshot.paused, snapshot.blackout);
        if let Some(previous) = self.controls {
            if previous.0 != controls.0 {
                self.push(
                    "info",
                    "engine",
                    if controls.0 {
                        "Playback paused"
                    } else {
                        "Playback resumed"
                    },
                );
            }
            if previous.1 != controls.1 {
                self.push(
                    "info",
                    "output",
                    if controls.1 {
                        "Blackout enabled"
                    } else {
                        "Blackout disabled"
                    },
                );
            }
        }
        self.controls = Some(controls);
        if self
            .revision
            .is_some_and(|revision| revision != snapshot.revision)
        {
            self.push(
                "info",
                "project",
                &format!("Project revision {}", snapshot.revision),
            );
        }
        self.revision = Some(snapshot.revision);
    }
    pub fn read(&self, query: Query) -> Page {
        let reset = query.after > self.sequence;
        let after = if reset { 0 } else { query.after };
        let limit = if query.limit == 0 {
            200
        } else {
            query.limit.min(200)
        };
        let entries: Vec<_> = self
            .entries
            .iter()
            .filter(|entry| entry.id > after)
            .take(limit)
            .cloned()
            .collect();
        Page {
            generation: format!("{:?}", self.started),
            next: entries.last().map_or(after, |entry| entry.id),
            latest: self.sequence,
            truncated: after > 0
                && self
                    .entries
                    .front()
                    .is_some_and(|entry| after < entry.id.saturating_sub(1)),
            reset,
            entries,
        }
    }
}
