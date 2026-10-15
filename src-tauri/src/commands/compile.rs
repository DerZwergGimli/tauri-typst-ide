use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::time::Instant;

use serde::Serialize;
use tauri::State;
use tauri_plugin_opener::OpenerExt;
use typst::{compile as typst_compile, World, WorldExt};
use typst::diag::{Severity, SourceDiagnostic, Warned};
use typst_layout::PagedDocument;
use typst_pdf::{pdf, PdfOptions};

use crate::error::{AppError, AppResult};
use crate::events::EmitExt;
use crate::state::AppState;
use crate::typst_engine::world::{ProjectWorld, shared_fonts};

#[derive(Debug, Clone, Serialize)]
pub struct Diagnostic {
    pub severity: &'static str,
    pub message: String,
    pub path: Option<String>,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub offset: Option<usize>,
    pub length: Option<usize>,
    pub hints: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CompileResult {
    pub success: bool,
    pub pdf_path: Option<String>,
    pub duration_ms: u64,
    pub diagnostics: Vec<Diagnostic>,
}

/// Internal task outcome: `None` means the compile was superseded and must be
/// discarded.
type TaskOutcome = Option<(bool, Option<PathBuf>, Vec<Diagnostic>)>;

fn diagnostic_severity(sev: Severity) -> &'static str {
    match sev {
        Severity::Error => "error",
        Severity::Warning => "warning",
    }
}

fn map_diagnostic(
    world: &ProjectWorld,
    d: &SourceDiagnostic,
) -> Diagnostic {
    let mut path: Option<String> = None;
    let mut line: Option<usize> = None;
    let mut column: Option<usize> = None;
    let mut offset: Option<usize> = None;
    let mut length: Option<usize> = None;

    // File + position
    if let Some(id) = d.span.id() {
        // path
        path = Some(world.name(id));
        // range
        if let Some(range) = world.range(d.span) {
            offset = Some(range.start);
            length = Some(range.end.saturating_sub(range.start));
            // line/col
            if let Ok(src) = world.source(id)
                && let Some((l, c)) = src.lines().byte_to_line_column(range.start)
            {
                // 1-based for UI
                line = Some(l + 1);
                column = Some(c + 1);
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

fn map_diagnostics<'a>(
    world: &ProjectWorld,
    diags: impl Iterator<Item = &'a SourceDiagnostic>,
) -> Vec<Diagnostic> {
    diags.map(|e| map_diagnostic(world, e)).collect()
}

#[tauri::command]
pub async fn compile(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> AppResult<CompileResult> {
    let start = Instant::now();
    let project = state.project()?;
    let fonts = shared_fonts();

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

    let result = tauri::async_runtime::spawn_blocking(
        move || -> Result<TaskOutcome, AppError> {
            let world = ProjectWorld::new(&root, &main, fonts)?;
            let Warned { output, warnings } = typst_compile::<PagedDocument>(&world);

            let stale = || gen_in_task.load(Ordering::SeqCst) != generation;

            match output {
                Ok(doc) => {
                    let mut diags: Vec<Diagnostic> = warnings
                        .iter()
                        .map(|w| map_diagnostic(&world, w))
                        .collect();
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
        },
    )
    .await
    .map_err(|e| AppError::Other(format!("spawn_blocking: {e}")))??;

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
            duration_ms: start.elapsed().as_millis() as u64,
            diagnostics: Vec::new(),
        });
    };

    let duration_ms = start.elapsed().as_millis() as u64;
    let pdf_path_str = pdf_path.as_ref().map(|p| p.to_string_lossy().to_string());

    if success && pdf_path.is_some() {
        app.log("info", format!("compile succeeded in {duration_ms}ms"));
        if let Some(p) = &pdf_path {
            let _ = app
                .opener()
                .open_path(p.to_string_lossy().to_string(), None::<&str>);
        }
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
        Some(duration_ms),
    );
    Ok(res)
}
