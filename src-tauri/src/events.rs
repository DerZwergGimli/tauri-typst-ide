use serde::Serialize;
use tauri::Emitter;

#[derive(Debug, Clone, Serialize)]
pub struct LogPayload {
    pub level: &'static str,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProcessStatusPayload {
    pub process: &'static str,
    pub status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

pub trait EmitExt {
    fn log(&self, level: &'static str, message: impl Into<String>);
    #[allow(dead_code)]
    fn log_ctx(
        &self,
        level: &'static str,
        message: impl Into<String>,
        context: impl Into<String>,
    );
    fn process_status(
        &self,
        process: &'static str,
        status: &'static str,
        duration_ms: Option<u64>,
    );
}

impl EmitExt for tauri::AppHandle {
    fn log(&self, level: &'static str, message: impl Into<String>) {
        let _ = self.emit(
            "log",
            LogPayload {
                level,
                message: message.into(),
                context: None,
            },
        );
    }

    fn log_ctx(
        &self,
        level: &'static str,
        message: impl Into<String>,
        context: impl Into<String>,
    ) {
        let _ = self.emit(
            "log",
            LogPayload {
                level,
                message: message.into(),
                context: Some(context.into()),
            },
        );
    }

    fn process_status(
        &self,
        process: &'static str,
        status: &'static str,
        duration_ms: Option<u64>,
    ) {
        let _ = self.emit(
            "process-status",
            ProcessStatusPayload {
                process,
                status,
                duration_ms,
            },
        );
    }
}
