use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::Ordering;
use std::time::Instant;

use tauri::State;

use crate::commands::compile::{CompileResult, Diagnostic, execute_typst_compile};
use crate::error::{AppError, AppResult, rel_from_root};
use crate::events::EmitExt;
use crate::state::AppState;

#[derive(Debug, Clone)]
enum StepKind {
    Ps1,
    Sh,
    Bash,
}

#[derive(Debug, Clone)]
struct PipelineStep {
    order: u32,
    rel_path: String,
    abs_path: PathBuf,
    kind: StepKind,
}

fn parse_step(path: &Path, root: &Path) -> Option<PipelineStep> {
    let name = path.file_name()?.to_str()?;
    let prefix_len = name.chars().take_while(|c| c.is_ascii_digit()).count();
    if prefix_len == 0 {
        return None;
    }
    let order = name[..prefix_len].parse::<u32>().ok()?;
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    let kind = match ext.as_str() {
        "ps1" => StepKind::Ps1,
        "sh" => StepKind::Sh,
        "bash" => StepKind::Bash,
        _ => return None,
    };
    Some(PipelineStep {
        order,
        rel_path: rel_from_root(root, path).ok()?,
        abs_path: path.to_path_buf(),
        kind,
    })
}

fn discover_steps(root: &Path) -> AppResult<Vec<PipelineStep>> {
    let pipeline_dir = root.join("pipeline");
    if !pipeline_dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut steps = Vec::new();
    let entries = std::fs::read_dir(&pipeline_dir)
        .map_err(|e| AppError::io(pipeline_dir.display().to_string(), e))?;
    for entry in entries {
        let entry = entry.map_err(|e| AppError::io(pipeline_dir.display().to_string(), e))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if let Some(step) = parse_step(&path, root) {
            steps.push(step);
        }
    }

    steps.sort_by(|a, b| {
        a.order
            .cmp(&b.order)
            .then_with(|| a.rel_path.to_lowercase().cmp(&b.rel_path.to_lowercase()))
    });
    Ok(steps)
}

fn count_typst_directives(script: &str) -> usize {
    script
        .lines()
        .filter(|line| line.to_ascii_lowercase().contains("@typst compile"))
        .count()
}

fn make_command(kind: &StepKind, script: &Path) -> AppResult<Command> {
    match kind {
        StepKind::Ps1 => {
            if !cfg!(target_os = "windows") {
                return Err(AppError::Other(format!(
                    "incompatible script type for this platform: {}",
                    script.display()
                )));
            }
            let mut cmd = Command::new("powershell.exe");
            cmd.arg("-NoProfile")
                .arg("-ExecutionPolicy")
                .arg("Bypass")
                .arg("-File")
                .arg(script);
            Ok(cmd)
        }
        StepKind::Sh | StepKind::Bash => {
            if cfg!(target_os = "windows") {
                return Err(AppError::Other(format!(
                    "incompatible script type for this platform: {}",
                    script.display()
                )));
            }
            let mut cmd = Command::new("bash");
            cmd.arg(script);
            Ok(cmd)
        }
    }
}

fn output_tail(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    const MAX: usize = 800;
    let chars: Vec<char> = text.chars().collect();
    let start = chars.len().saturating_sub(MAX);
    chars[start..]
        .iter()
        .collect::<String>()
        .trim()
        .to_string()
}

fn make_script_failure(rel_path: &str, message: String) -> Diagnostic {
    Diagnostic {
        severity: "error",
        message,
        path: Some(rel_path.to_string()),
        line: None,
        column: None,
        offset: None,
        length: None,
        hints: Vec::new(),
    }
}

async fn run_script_step(step: &PipelineStep, root: PathBuf) -> AppResult<(String, String)> {
    let mut command = make_command(&step.kind, &step.abs_path)?;
    command.current_dir(root);

    let output = tauri::async_runtime::spawn_blocking(move || {
        command
            .output()
            .map_err(|e| AppError::io("execute pipeline step", e))
    })
    .await
    .map_err(|e| AppError::Other(format!("spawn_blocking: {e}")))??;

    let stdout = output_tail(&output.stdout);
    let stderr = output_tail(&output.stderr);

    if !output.status.success() {
        let code = output
            .status
            .code()
            .map_or_else(|| "unknown".to_string(), |c| c.to_string());
        return Err(AppError::Other(format!(
            "pipeline step failed: {} (exit code {code})",
            step.rel_path
        )));
    }

    Ok((stdout, stderr))
}

#[tauri::command]
#[specta::specta]
pub async fn run_build_pipeline(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> AppResult<CompileResult> {
    let start = Instant::now();
    let project = state.project()?;
    let root = project.root.clone();
    let main = project
        .main_file
        .clone()
        .ok_or_else(|| AppError::Other("no main file selected".into()))?;

    let (generation, gen_handle) = state.begin_compile();

    app.process_status("compile", "started", None);
    app.log("info", "running build pipeline...");

    let steps = discover_steps(&root)?;
    if steps.is_empty() {
        app.log("info", "no pipeline steps found; running typst compile");
        let result = execute_typst_compile(root.clone(), main, generation, gen_handle).await?;
        let Some((success, pdf_path, diagnostics)) = result else {
            app.log_ctx(
                "info",
                format!(
                    "pipeline superseded after {}ms",
                    start.elapsed().as_millis()
                ),
                "pipeline",
            );
            return Ok(CompileResult {
                success: false,
                pdf_path: None,
                duration_ms: start.elapsed().as_millis() as f64,
                diagnostics: Vec::new(),
            });
        };

        let duration_ms = start.elapsed().as_millis() as f64;
        let pdf_path = pdf_path
            .as_ref()
            .and_then(|p| rel_from_root(&root, p).ok())
            .or_else(|| pdf_path.as_ref().map(|p| p.to_string_lossy().to_string()));

        app.process_status(
            "compile",
            if success { "finished" } else { "failed" },
            Some(duration_ms as u64),
        );

        return Ok(CompileResult {
            success,
            pdf_path,
            duration_ms,
            diagnostics,
        });
    }

    let mut latest_pdf: Option<String> = None;
    let mut diagnostics: Vec<Diagnostic> = Vec::new();

    for step in steps {
        if gen_handle.load(Ordering::SeqCst) != generation {
            app.log_ctx(
                "info",
                format!(
                    "pipeline superseded after {}ms",
                    start.elapsed().as_millis()
                ),
                "pipeline",
            );
            return Ok(CompileResult {
                success: false,
                pdf_path: None,
                duration_ms: start.elapsed().as_millis() as f64,
                diagnostics: Vec::new(),
            });
        }

        app.log_ctx("info", format!("step: {}", step.rel_path), "pipeline");

        let script = std::fs::read_to_string(&step.abs_path)
            .map_err(|e| AppError::io(format!("read {}", step.rel_path), e))?;
        let typst_directives = count_typst_directives(&script);

        let (stdout, stderr) = match run_script_step(&step, root.clone()).await {
            Ok(out) => out,
            Err(e) => {
                let duration_ms = start.elapsed().as_millis() as f64;
                let diag = make_script_failure(&step.rel_path, e.to_string());
                app.log_ctx("error", diag.message.clone(), "pipeline");
                app.process_status("compile", "failed", Some(duration_ms as u64));
                return Ok(CompileResult {
                    success: false,
                    pdf_path: latest_pdf,
                    duration_ms,
                    diagnostics: vec![diag],
                });
            }
        };

        if !stdout.is_empty() {
            app.log_ctx("info", format!("stdout {}: {stdout}", step.rel_path), "pipeline");
        }
        if !stderr.is_empty() {
            app.log_ctx("warn", format!("stderr {}: {stderr}", step.rel_path), "pipeline");
        }

        for idx in 0..typst_directives {
            app.log_ctx(
                "info",
                format!(
                    "typst compile directive {} in {}",
                    idx + 1,
                    step.rel_path
                ),
                "pipeline",
            );
            let outcome = execute_typst_compile(
                root.clone(),
                main.clone(),
                generation,
                gen_handle.clone(),
            )
            .await?;

            let Some((success, pdf_path, compile_diags)) = outcome else {
                app.log_ctx(
                    "info",
                    format!(
                        "pipeline superseded after {}ms",
                        start.elapsed().as_millis()
                    ),
                    "pipeline",
                );
                return Ok(CompileResult {
                    success: false,
                    pdf_path: latest_pdf,
                    duration_ms: start.elapsed().as_millis() as f64,
                    diagnostics: Vec::new(),
                });
            };

            diagnostics = compile_diags;
            if let Some(path) = pdf_path {
                latest_pdf = rel_from_root(&root, &path)
                    .ok()
                    .or_else(|| Some(path.to_string_lossy().to_string()));
            }

            if !success {
                let duration_ms = start.elapsed().as_millis() as f64;
                app.process_status("compile", "failed", Some(duration_ms as u64));
                app.log_ctx("warn", format!("pipeline failed in {duration_ms}ms"), "pipeline");
                return Ok(CompileResult {
                    success: false,
                    pdf_path: latest_pdf,
                    duration_ms,
                    diagnostics,
                });
            }
        }
    }

    let duration_ms = start.elapsed().as_millis() as f64;
    app.process_status("compile", "finished", Some(duration_ms as u64));
    app.log_ctx("info", format!("pipeline succeeded in {duration_ms}ms"), "pipeline");

    Ok(CompileResult {
        success: true,
        pdf_path: latest_pdf,
        duration_ms,
        diagnostics,
    })
}
