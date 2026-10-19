use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Instant;

use serde::Serialize;
use specta::Type;
use tauri::State;
use typst::diag::{Severity, SourceDiagnostic, Warned};
use typst::{World, WorldExt, compile as typst_compile};
use typst_layout::PagedDocument;
use typst_pdf::{PdfOptions, pdf};

use crate::error::{AppError, AppResult, rel_from_root};
use crate::events::EmitExt;
use crate::state::AppState;
use crate::typst_engine::world::{ProjectWorld, shared_fonts};

#[derive(Debug, Clone, Serialize, Type)]
pub struct Diagnostic {
    pub severity: &'static str,
    pub message: String,
    pub path: Option<String>,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub offset: Option<u32>,
    pub length: Option<u32>,
    pub hints: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct CompileResult {
    pub success: bool,
    pub pdf_path: Option<String>,
    pub duration_ms: f64,
    pub diagnostics: Vec<Diagnostic>,
}

/// Internal task outcome: `None` means the compile was superseded and must be
/// discarded.
pub(crate) type TaskOutcome = Option<(bool, Option<PathBuf>, Vec<Diagnostic>)>;

fn diagnostic_severity(sev: Severity) -> &'static str {
    match sev {
        Severity::Error => "error",
        Severity::Warning => "warning",
    }
}

pub(crate) fn map_diagnostic(world: &ProjectWorld, d: &SourceDiagnostic) -> Diagnostic {
    let mut path: Option<String> = None;
    let mut line: Option<u32> = None;
    let mut column: Option<u32> = None;
    let mut offset: Option<u32> = None;
    let mut length: Option<u32> = None;

    // File + position
    if let Some(id) = d.span.id() {
        // path
        path = Some(world.name(id));
        // range
        if let Some(range) = world.range(d.span) {
            offset = u32::try_from(range.start).ok();
            length = u32::try_from(range.end.saturating_sub(range.start)).ok();
            // line/col
            if let Ok(src) = world.source(id)
                && let Some((l, c)) = src.lines().byte_to_line_column(range.start)
            {
                // 1-based for UI
                line = u32::try_from(l + 1).ok();
                column = u32::try_from(c + 1).ok();
            }
        }
    }

    let hints = d.hints.iter().map(|h| h.v.to_string()).collect();

    Diagnostic {
        severity: diagnostic_severity(d.severity),
        message: d.message.to_string(),
        path,
        line,
        column,
        offset,
        length,
        hints,
    }
}

pub(crate) fn map_diagnostics<'a>(
    world: &ProjectWorld,
    diags: impl Iterator<Item = &'a SourceDiagnostic>,
) -> Vec<Diagnostic> {
    diags.map(|e| map_diagnostic(world, e)).collect()
}

pub(crate) async fn execute_typst_compile(
    root: PathBuf,
    main: PathBuf,
    generation: u64,
    gen_handle: Arc<std::sync::atomic::AtomicU64>,
) -> AppResult<TaskOutcome> {
    let fonts = shared_fonts();

    tauri::async_runtime::spawn_blocking(move || -> Result<TaskOutcome, AppError> {
        let world = ProjectWorld::new(&root, &main, fonts)?;
        let Warned { output, warnings } = typst_compile::<PagedDocument>(&world);

        let stale = || gen_handle.load(Ordering::SeqCst) != generation;

        match output {
            Ok(doc) => {
                let mut diags: Vec<Diagnostic> =
                    warnings.iter().map(|w| map_diagnostic(&world, w)).collect();
                match pdf(&doc, &PdfOptions::default()) {
                    Ok(bytes) => {
                        if stale() {
                            return Ok(None);
                        }
                        let stem = main
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("document");
                        let pdf_path = main.with_file_name(format!("{stem}.pdf"));
                        std::fs::write(&pdf_path, &bytes)
                            .map_err(|e| AppError::io("write pdf", e))?;
                        Ok(Some((true, Some(pdf_path), diags)))
                    }
                    Err(errs) => {
                        diags.extend(map_diagnostics(&world, errs.iter()));
                        Ok(Some((false, None, diags)))
                    }
                }
            }
            Err(errs) => {
                let mut diags: Vec<Diagnostic> = map_diagnostics(&world, errs.iter());
                for w in &warnings {
                    diags.push(map_diagnostic(&world, w));
                }
                Ok(Some((false, None, diags)))
            }
        }
    })
    .await
    .map_err(|e| AppError::Other(format!("spawn_blocking: {e}")))?
}

#[tauri::command]
#[specta::specta]
pub async fn compile(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> AppResult<CompileResult> {
    let start = Instant::now();
    let project = state.project()?;
    // Ensure main file is set
    let main_abs = project
        .main_file
        .as_ref()
        .cloned()
        .ok_or_else(|| AppError::Other("no main file selected".into()))?;

    let root = project.root.clone();
    let main = main_abs.clone();

    let (generation, gen_handle) = state.begin_compile();
    let gen_in_task = gen_handle.clone();

    app.process_status("compile", "started", None);
    app.log("info", "compiling...");

    let result = execute_typst_compile(root.clone(), main.clone(), generation, gen_in_task).await?;

    // A newer compilation superseded this one; discard its result silently.
    let Some((success, pdf_path, diagnostics)) = result else {
        app.log_ctx(
            "info",
            format!("compile superseded after {}ms", start.elapsed().as_millis()),
            "compile",
        );
        return Ok(CompileResult {
            success: false,
            pdf_path: None,
            duration_ms: start.elapsed().as_millis() as f64,
            diagnostics: Vec::new(),
        });
    };

    let duration_ms = start.elapsed().as_millis() as f64;
    let pdf_path_str = pdf_path
        .as_ref()
        .and_then(|p| rel_from_root(&root, p).ok())
        .or_else(|| pdf_path.as_ref().map(|p| p.to_string_lossy().to_string()));

    if success {
        app.log("info", format!("compile succeeded in {duration_ms}ms"));
    } else {
        app.log("warn", format!("compile failed in {duration_ms}ms"));
    }

    let res = CompileResult {
        success,
        pdf_path: pdf_path_str,
        duration_ms,
        diagnostics,
    };
    app.process_status(
        "compile",
        if success { "finished" } else { "failed" },
        Some(duration_ms as u64),
    );
    Ok(res)
}
