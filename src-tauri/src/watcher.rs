use std::path::Path;
use std::sync::Mutex;
use std::time::Duration;

use notify::event::EventKind;
use notify::RecursiveMode;
use notify_debouncer_full::{
    new_debouncer, DebounceEventResult, DebouncedEvent, Debouncer, RecommendedCache,
};
use notify::RecommendedWatcher;
use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::error::rel_from_root;

/// Debounced recursive filesystem watcher for a project root.
pub type ProjectWatcher = Debouncer<RecommendedWatcher, RecommendedCache>;

#[derive(Debug, Clone, Serialize)]
pub struct FsChangedPayload {
    pub kind: &'static str,
    pub paths: Vec<String>,
}

/// Managed state slot that owns the watcher for the current project. Dropping
/// the previous watcher (by replacing it) stops watching automatically.
#[derive(Default)]
pub struct WatcherState(pub Mutex<Option<ProjectWatcher>>);

fn kind_str(kind: &EventKind) -> Option<&'static str> {
    match kind {
        EventKind::Create(_) => Some("created"),
        EventKind::Remove(_) => Some("removed"),
        // Access events fire on every open/read — noise we must ignore.
        EventKind::Access(_) => None,
        EventKind::Modify(_) => Some("modified"),
        EventKind::Any | EventKind::Other => Some("modified"),
    }
}

/// Start (or restart) watching `root`. Ignored paths (dotfiles, `target`,
/// `node_modules`) are filtered out, and events are coalesced into a single
/// `fs-changed` emission every ~250ms.
pub fn watch_project(
    app: &AppHandle,
    watcher_state: &WatcherState,
    root: &Path,
) -> Result<(), notify::Error> {
    let root = std::sync::Arc::new(root.to_path_buf());
    let app = app.clone();
    let root_for_handler = root.clone();

    let mut debouncer = new_debouncer(
        Duration::from_millis(250),
        None,
        move |result: DebounceEventResult| {
            let Ok(events) = result else { return };
            emit_changes(&app, &root_for_handler, events);
        },
    )?;

    debouncer.watch(root.as_ref(), RecursiveMode::Recursive)?;

    let mut slot = watcher_state.0.lock().unwrap();
    *slot = Some(debouncer);
    Ok(())
}

/// Stop watching the current project (if any).
pub fn unwatch(watcher_state: &WatcherState) {
    *watcher_state.0.lock().unwrap() = None;
}

fn is_ignored(rel: &str) -> bool {
    rel.split('/').any(|segment| {
        segment.starts_with('.') || segment == "target" || segment == "node_modules"
    })
}

fn emit_changes(app: &AppHandle, root: &Path, events: Vec<DebouncedEvent>) {
    // Coalesce by kind, deduplicating paths.
    let mut created: Vec<String> = Vec::new();
    let mut removed: Vec<String> = Vec::new();
    let mut modified: Vec<String> = Vec::new();

    for event in events {
        let Some(kind) = kind_str(&event.kind) else {
            continue;
        };
        for path in &event.paths {
            let Ok(rel) = rel_from_root(root, path) else {
                continue;
            };
            if is_ignored(&rel) {
                continue;
            }
            let bucket = match kind {
                "created" => &mut created,
                "removed" => &mut removed,
                _ => &mut modified,
            };
            if !bucket.contains(&rel) {
                bucket.push(rel);
            }
        }
    }

    for (kind, paths) in [
        ("created", created),
        ("removed", removed),
        ("modified", modified),
    ] {
        if paths.is_empty() {
            continue;
        }
        let _ = app.emit("fs-changed", FsChangedPayload { kind, paths });
    }
}
