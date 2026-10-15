import type { LanguageSupport } from "@codemirror/language";

/**
 * Lazily-loaded, WASM-free Typst language support for CodeMirror.
 *
 * We only pull syntax highlighting, indentation, folding and completion from
 * the Lezer parser. Diagnostics are intentionally *not* taken from the editor:
 * the Rust Typst engine is the single source of truth for compilation errors.
 */
let cached: Promise<LanguageSupport> | null = null;

export function loadTypstLanguage(): Promise<LanguageSupport> {
  if (!cached) {
    cached = import("codemirror-lang-typst/lezer").then((mod) => mod.typst_lezer());
  }
  return cached;
}
