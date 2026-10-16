use std::path::Path;
use std::time::UNIX_EPOCH;

use serde::Serialize;
use tauri::State;

use crate::error::{rel_from_root, resolve_in_root, AppError, AppResult};
use crate::state::AppState;

use specta::Type;

#[derive(Debug, Clone, Serialize, Type)]
pub struct FileNode {
    pub name: String,
    pub path: String,
    pub kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<FileNode>>,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct FileContent {
    pub path: String,
    pub content: String,
    pub mtime_ms: f64,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct FileMeta {
    pub path: String,
    pub mtime_ms: f64,
}

fn mtime_ms(path: &Path) -> AppResult<f64> {
    let meta = std::fs::metadata(path).map_err(|e| AppError::io(path.display().to_string(), e))?;
    let mtime = meta
        .modified()
        .map_err(|e| AppError::io(path.display().to_string(), e))?;
    Ok(mtime
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as f64)
        .unwrap_or(0.0))
}

fn read_tree_at(root: &Path, dir: &Path, depth: usize) -> AppResult<Vec<FileNode>> {
    if depth > 32 {
        return Ok(vec![]);
    }
    let entries = std::fs::read_dir(dir).map_err(|e| AppError::io(dir.display().to_string(), e))?;
    let mut nodes: Vec<FileNode> = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| AppError::io(dir.display().to_string(), e))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') || name == "node_modules" || name == "target" {
            continue;
        }
        let path = entry.path();
        let ty = entry
            .file_type()
            .map_err(|e| AppError::io(path.display().to_string(), e))?;
        let rel = rel_from_root(root, &path)?;
        if ty.is_dir() {
            nodes.push(FileNode {
                name,
                path: rel,
                kind: "dir",
                children: Some(read_tree_at(root, &path, depth + 1)?),
            });
        } else if ty.is_file() {
            nodes.push(FileNode {
                name,
                path: rel,
                kind: "file",
                children: None,
            });
        }
    }
    nodes.sort_by(|a, b| match (a.kind, b.kind) {
        ("dir", "file") => std::cmp::Ordering::Less,
        ("file", "dir") => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });
    Ok(nodes)
}

#[tauri::command]
pub fn read_dir_tree(state: State<'_, AppState>) -> AppResult<FileNode> {
    let project = state.project()?;
    let children = read_tree_at(&project.root, &project.root, 0)?;
    Ok(FileNode {
        name: project
            .root
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| project.root.to_string_lossy().into_owned()),
        path: String::new(),
        kind: "dir",
        children: Some(children),
    })
}

#[tauri::command]
#[specta::specta]
pub fn read_file(state: State<'_, AppState>, rel_path: String) -> AppResult<FileContent> {
    let project = state.project()?;
    let abs = resolve_in_root(&project.root, &rel_path)?;
    if !abs.is_file() {
        return Err(AppError::NotAFile(rel_path.clone()));
    }
    let content = std::fs::read_to_string(&abs)
        .map_err(|e| AppError::Io {
            context: format!("read {rel_path}"),
            message: e.to_string(),
        })?;
    Ok(FileContent {
        path: rel_path,
        content,
        mtime_ms: mtime_ms(&abs)?,
    })
}



#[tauri::command]
pub fn write_file(
    state: State<'_, AppState>,
    rel_path: String,
    content: String,
) -> AppResult<FileMeta> {
    let project = state.project()?;
    let abs = resolve_in_root(&project.root, &rel_path)?;
    if abs.exists() && !abs.is_file() {
        return Err(AppError::NotAFile(rel_path.clone()));
    }
    std::fs::write(&abs, content).map_err(|e| AppError::Io {
        context: format!("write {rel_path}"),
        message: e.to_string(),
    })?;
    Ok(FileMeta {
        path: rel_path,
        mtime_ms: mtime_ms(&abs)?,
    })
}

#[tauri::command]
pub fn file_mtime(state: State<'_, AppState>, rel_path: String) -> AppResult<Option<FileMeta>> {
    let project = state.project()?;
    let abs = resolve_in_root(&project.root, &rel_path)?;
    if !abs.is_file() {
        return Ok(None);
    }
    Ok(Some(FileMeta {
        path: rel_path,
        mtime_ms: mtime_ms(&abs)?,
    }))
}

#[tauri::command]
pub fn create_entry(
    state: State<'_, AppState>,
    rel_path: String,
    kind: String,
) -> AppResult<()> {
    let project = state.project()?;
    let abs = resolve_in_root(&project.root, &rel_path)?;
    if abs.exists() {
        return Err(AppError::AlreadyExists(rel_path));
    }
    match kind.as_str() {
        "file" => {
            if let Some(parent) = abs.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| AppError::io(parent.display().to_string(), e))?;
            }
            std::fs::write(&abs, "").map_err(|e| AppError::io(rel_path, e))?;
            Ok(())
        }
        "dir" | "folder" => {
            std::fs::create_dir_all(&abs).map_err(|e| AppError::io(rel_path, e))?;
            Ok(())
        }
        other => Err(AppError::InvalidPath(other.to_string())),
    }
}

#[tauri::command]
pub fn rename_entry(
    state: State<'_, AppState>,
    old_path: String,
    new_path: String,
) -> AppResult<()> {
    let project = state.project()?;
    let old_abs = resolve_in_root(&project.root, &old_path)?;
    let new_abs = resolve_in_root(&project.root, &new_path)?;
    if !old_abs.exists() {
        return Err(AppError::NotFound(old_path));
    }
    if new_abs.exists() {
        return Err(AppError::AlreadyExists(new_path));
    }
    if let Some(parent) = new_abs.parent() {
        std::fs::create_dir_all(parent).map_err(|e| AppError::io(parent.display().to_string(), e))?;
    }
    std::fs::rename(&old_abs, &new_abs).map_err(|e| AppError::Io {
        context: format!("rename {old_path} -> {new_path}"),
        message: e.to_string(),
    })
}

#[tauri::command]
pub fn delete_entry(state: State<'_, AppState>, rel_path: String) -> AppResult<()> {
    let project = state.project()?;
    if rel_path.is_empty() {
        return Err(AppError::InvalidPath(rel_path));
    }
    let abs = resolve_in_root(&project.root, &rel_path)?;
    if !abs.exists() {
        return Err(AppError::NotFound(rel_path));
    }
    if abs.is_dir() {
        std::fs::remove_dir_all(&abs).map_err(|e| AppError::Io {
            context: format!("delete {rel_path}"),
            message: e.to_string(),
        })
    } else {
        std::fs::remove_file(&abs).map_err(|e| AppError::Io {
            context: format!("delete {rel_path}"),
            message: e.to_string(),
        })
    }
}
