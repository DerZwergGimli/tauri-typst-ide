import { LanguageDescription, LanguageSupport } from "@codemirror/language";
import { languages as builtin } from "@codemirror/language-data";

let cached: Promise<LanguageSupport> | null = null;

export function loadTypstLanguage(): Promise<LanguageSupport> {
  if (!cached) {
    cached = import("codemirror-lang-typst/lezer").then((mod) =>
      mod.typst_lezer(),
    );
  }
  return cached;
}

const typst = LanguageDescription.of({
  name: "Typst",
  extensions: ["typ"],
  // Swap in whatever your current Typst extension is
  load: async () => {
    const { typst_lezer } = await import("codemirror-lang-typst/lezer");
    return typst_lezer(); // must return a LanguageSupport
  },
});

// Custom entries first so they win over built-ins on conflicts
export const languages: LanguageDescription[] = [typst, ...builtin];

export function findLanguage(filename: string): LanguageDescription | null {
  console.log(LanguageDescription.matchFilename(languages, filename));
  return LanguageDescription.matchFilename(languages, filename);
}
