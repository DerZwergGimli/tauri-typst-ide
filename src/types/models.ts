export type FileNodeKind = "file" | "dir";

export interface FileNode {
  name: string;
  path: string;
  kind: FileNodeKind;
  children?: FileNode[];
}

export interface ProjectInfo {
  root: string;
  name: string;
  main_file: string | null;
  typ_files: string[];
}

export interface FileContent {
  path: string;
  content: string;
  mtime_ms: number;
}

export interface FileContentBytes {
  path: string;
  content: number[];
  mtime_ms: number;
}

export interface FileMeta {
  path: string;
  mtime_ms: number;
}

export type DiagnosticSeverity = "error" | "warning";

export interface Diagnostic {
  severity: DiagnosticSeverity;
  message: string;
  path: string | null;
  line: number | null;
  column: number | null;
  offset: number | null;
  length: number | null;
  hints: string[];
}

export interface CompileResult {
  success: boolean;
  pdf_path: string | null;
  duration_ms: number;
  diagnostics: Diagnostic[];
}

export type LogLevel = "info" | "warn" | "error";

export interface LogEntry {
  timestamp: string;
  level: LogLevel;
  message: string;
  context?: string;
}

/** Payload emitted by the Rust `log` event (no timestamp; frontend stamps it). */
export interface LogPayload {
  level: LogLevel;
  message: string;
  context?: string;
}

export type ProcessName = "compile";
export type ProcessStatusKind = "started" | "finished" | "failed";

export interface ProcessStatusEvent {
  process: ProcessName;
  status: ProcessStatusKind;
  duration_ms?: number;
}

export type FsChangeKind = "created" | "modified" | "removed" | "renamed";

export interface FsChangedEvent {
  kind: FsChangeKind;
  paths: string[];
}

export interface RecentProject {
  path: string;
  name: string;
  last_opened: string;
}
