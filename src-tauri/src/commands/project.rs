use std::path::{Path, PathBuf};

use serde::Serialize;
use specta::Type;
use tauri::State;
use tauri_plugin_store::StoreExt;

use crate::error::{AppError, AppResult, rel_from_root, resolve_in_root};
use crate::state::{AppState, ProjectState};
use crate::watcher::{WatcherState, watch_project};

#[derive(Debug, Clone, Serialize, Type)]
pub struct ProjectInfo {
    pub root: String,
    pub name: String,
    pub main_file: Option<String>,
    pub typ_files: Vec<String>,
}

const STARTER_MAIN: &str = r#"= Starter document

Write Typst here. Press the compile button (or Ctrl+Enter) to build a PDF.
"#;

fn collect_typ_files(root: &Path) -> AppResult<Vec<String>> {
    let mut files = Vec::new();
    collect_typ_files_inner(root, root, &mut files, 0)?;
    files.sort();
    Ok(files)
}

fn collect_typ_files_inner(
    root: &Path,
    dir: &Path,
    out: &mut Vec<String>,
    depth: usize,
) -> AppResult<()> {
    if depth > 16 {
        return Ok(());
    }
    let entries = std::fs::read_dir(dir).map_err(|e| AppError::io(dir.display().to_string(), e))?;
    for entry in entries {
        let entry = entry.map_err(|e| AppError::io(dir.display().to_string(), e))?;
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with('.') || name == "node_modules" || name == "target" {
            continue;
        }
        let ty = entry
            .file_type()
            .map_err(|e| AppError::io(path.display().to_string(), e))?;
        if ty.is_dir() {
            collect_typ_files_inner(root, &path, out, depth + 1)?;
        } else if ty.is_file() && path.extension().is_some_and(|e| e == "typ") {
            out.push(rel_from_root(root, &path)?);
        }
    }
    Ok(())
}

fn pick_main_file(root: &Path, preferred: Option<String>) -> Option<PathBuf> {
    if let Some(pref) = preferred {
        let p = root.join(&pref);
        if p.is_file() {
            return Some(p);
        }
    }
    let main = root.join("main.typ");
    if main.is_file() {
        return Some(main);
    }
    collect_typ_files(root)
        .ok()
        .and_then(|f| f.first().map(|rel| root.join(rel)))
}

fn build_project_info(root: &Path, main_file: Option<&Path>) -> AppResult<ProjectInfo> {
    Ok(ProjectInfo {
        root: root.to_string_lossy().into_owned(),
        name: root
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| root.to_string_lossy().into_owned()),
        main_file: main_file.map(|m| rel_from_root(root, m)).transpose()?,
        typ_files: collect_typ_files(root)?,
    })
}

#[tauri::command]
#[specta::specta]
pub fn open_project(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    watcher: State<'_, WatcherState>,
    path: String,
) -> AppResult<ProjectInfo> {
    let root = PathBuf::from(&path);
    if !root.is_dir() {
        return Err(AppError::NotADirectory(path));
    }
    let root = root
        .canonicalize()
        .map_err(|e| AppError::io("canonicalize project root", e))?;

    let preferred = last_main_file(&app, &root);
    let main = pick_main_file(&root, preferred);
    let info = build_project_info(&root, main.as_deref())?;

    state.set_project(ProjectState {
        root: root.clone(),
        main_file: main,
    });
    // Best-effort: a watcher failure should not prevent opening the project.
    let _ = watch_project(&app, &watcher, &root);
    Ok(info)
}

#[tauri::command]
#[specta::specta]
pub fn create_project(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    watcher: State<'_, WatcherState>,
    parent: String,
    name: String,
) -> AppResult<ProjectInfo> {
    let name = name.trim();
    if name.is_empty() || name.contains('/') || name.contains('\\') {
        return Err(AppError::InvalidPath(name.to_string()));
    }
    let parent = PathBuf::from(&parent);
    if !parent.is_dir() {
        return Err(AppError::NotADirectory(parent.display().to_string()));
    }
    let root = parent.join(name);
    if root.exists() {
        return Err(AppError::AlreadyExists(root.display().to_string()));
    }
    std::fs::create_dir_all(&root).map_err(|e| AppError::io("create project dir", e))?;
    let main_path = root.join("main.typ");
    std::fs::write(&main_path, STARTER_MAIN)
        .map_err(|e| AppError::io("write starter main.typ", e))?;

    let root = root
        .canonicalize()
        .map_err(|e| AppError::io("canonicalize project root", e))?;
    let info = build_project_info(&root, Some(&main_path))?;
    state.set_project(ProjectState {
        root: root.clone(),
        main_file: Some(main_path),
    });
    remember_main_file(&app, &root, "main.typ");
    let _ = watch_project(&app, &watcher, &root);
    Ok(info)
}

#[tauri::command]
#[specta::specta]
pub fn close_project(
    state: State<'_, AppState>,
    watcher: State<'_, WatcherState>,
) -> AppResult<()> {
    state.clear_project();
    crate::watcher::unwatch(&watcher);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn get_project_info(state: State<'_, AppState>) -> AppResult<Option<ProjectInfo>> {
    match state.project() {
        Ok(p) => Ok(Some(build_project_info(&p.root, p.main_file.as_deref())?)),
        Err(AppError::NoProject) => Ok(None),
        Err(e) => Err(e),
    }
}

#[tauri::command]
#[specta::specta]
pub fn set_main_file(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    rel_path: String,
) -> AppResult<()> {
    let mut project = state.project()?;
    let abs = resolve_in_root(&project.root, &rel_path)?;
    if !abs.is_file() {
        return Err(AppError::NotAFile(rel_path));
    }
    if abs.extension().is_none_or(|e| e != "typ") {
        return Err(AppError::InvalidPath(rel_path));
    }
    project.main_file = Some(abs);
    state.set_project(project.clone());
    remember_main_file(&app, &project.root, &rel_path);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn path_exists(path: String) -> bool {
    Path::new(&path).exists()
}

// --- per-project settings (main file memory) via the store plugin ---

fn settings_key(root: &Path) -> String {
    format!("project:{}", root.to_string_lossy())
}

fn remember_main_file(app: &tauri::AppHandle, root: &Path, rel: &str) {
    if let Ok(store) = app.store("app-settings.json") {
        let key = settings_key(root);
        let mut entry = serde_json::Map::new();
        entry.insert("mainFile".into(), serde_json::Value::String(rel.into()));
        store.set(key, serde_json::Value::Object(entry));
    }
}

fn last_main_file(app: &tauri::AppHandle, root: &Path) -> Option<String> {
    let store = app.store("app-settings.json").ok()?;
    let val = store.get(settings_key(root))?;
    val.get("mainFile")?.as_str().map(|s| s.to_string())
}
