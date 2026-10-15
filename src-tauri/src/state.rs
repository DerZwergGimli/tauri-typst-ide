use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use crate::error::{AppError, AppResult};

#[derive(Clone)]
pub struct ProjectState {
    pub root: PathBuf,
    pub main_file: Option<PathBuf>,
}

#[derive(Default)]
pub struct AppState {
    project: Mutex<Option<ProjectState>>,
    compile_gen: Arc<AtomicU64>,
}

impl AppState {
    pub fn set_project(&self, state: ProjectState) {
        *self.project.lock().unwrap() = Some(state);
    }

    pub fn clear_project(&self) {
        *self.project.lock().unwrap() = None;
        self.compile_gen.fetch_add(1, Ordering::SeqCst);
    }

    pub fn project(&self) -> AppResult<ProjectState> {
        self.project
            .lock()
            .unwrap()
            .clone()
            .ok_or(AppError::NoProject)
    }

    /// Start a new compilation, invalidating any in-flight one. Returns the
    /// generation token and a handle to check whether it is still current.
    pub fn begin_compile(&self) -> (u64, Arc<AtomicU64>) {
        let generation = self.compile_gen.fetch_add(1, Ordering::SeqCst) + 1;
        (generation, self.compile_gen.clone())
    }
}
