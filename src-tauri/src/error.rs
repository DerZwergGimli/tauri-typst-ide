use serde::Serialize;
use std::path::PathBuf;

use specta::Type;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Serialize, Type)]
#[serde(tag = "kind", content = "message", rename_all = "camelCase")]
pub enum AppError {
    NoProject,
    NotAFile(String),
    NotADirectory(String),
    NotFound(String),
    AlreadyExists(String),
    InvalidPath(String),
    OutsideProject(String),
    Io { context: String, message: String },
    Other(String),
}

impl AppError {
    pub fn io(context: impl Into<String>, err: std::io::Error) -> Self {
        AppError::Io {
            context: context.into(),
            message: err.to_string(),
        }
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::NoProject => write!(f, "no project is open"),
            AppError::NotAFile(p) => write!(f, "not a file: {p}"),
            AppError::NotADirectory(p) => write!(f, "not a directory: {p}"),
            AppError::NotFound(p) => write!(f, "not found: {p}"),
            AppError::AlreadyExists(p) => write!(f, "already exists: {p}"),
            AppError::InvalidPath(p) => write!(f, "invalid path: {p}"),
            AppError::OutsideProject(p) => write!(f, "path escapes project root: {p}"),
            AppError::Io { context, message } => write!(f, "{context}: {message}"),
            AppError::Other(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for AppError {}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::Io {
            context: "io".into(),
            message: err.to_string(),
        }
    }
}

/// Resolve a root-relative path against the project root, rejecting
/// absolute paths, `..` components, and symlink escapes.
pub fn resolve_in_root(root: &std::path::Path, rel: &str) -> AppResult<PathBuf> {
    use std::path::{Component, Path};

    if rel.is_empty() {
        return Ok(root.to_path_buf());
    }
    let rel_path = Path::new(rel);
    if rel_path.is_absolute()
        || rel_path
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err(AppError::InvalidPath(rel.to_string()));
    }

    let joined = root.join(rel_path);

    // Block symlink escapes: canonicalize the deepest existing ancestor
    // and confirm it stays inside the (canonicalized) root.
    let canon_root = root
        .canonicalize()
        .map_err(|e| AppError::io("resolve project root", e))?;
    let mut exist = joined.clone();
    while !exist.exists() {
        if !exist.pop() {
            break;
        }
    }
    let canon_exist = exist
        .canonicalize()
        .map_err(|e| AppError::io("resolve path", e))?;
    if !canon_exist.starts_with(&canon_root) {
        return Err(AppError::OutsideProject(rel.to_string()));
    }
    Ok(joined)
}

/// Convert an absolute path inside the root to a root-relative, `/`-separated path.
pub fn rel_from_root(root: &std::path::Path, abs: &std::path::Path) -> AppResult<String> {
    let rel = abs
        .strip_prefix(root)
        .map_err(|_| AppError::OutsideProject(abs.display().to_string()))?;
    Ok(rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/"))
}
