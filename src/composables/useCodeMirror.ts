import {
  EditorView,
  keymap,
  lineNumbers,
  highlightActiveLine,
  highlightActiveLineGutter,
} from "@codemirror/view";
import {
  EditorState,
  Compartment,
  EditorSelection,
  type Extension,
} from "@codemirror/state";
import {
  defaultKeymap,
  history,
  historyKeymap,
  indentWithTab,
} from "@codemirror/commands";
import {
  bracketMatching,
  HighlightStyle,
  indentOnInput,
  syntaxHighlighting,
} from "@codemirror/language";
import { highlightSelectionMatches } from "@codemirror/search";
import { closeBrackets } from "@codemirror/autocomplete";
import { tags as t } from "@lezer/highlight";
import { findLanguage } from "@/lib/languages.ts";

const c = (name: string) => `var(--${name})`;
const tint = (name: string, pct: number) =>
  `color-mix(in oklab, ${c(name)} ${pct}%, transparent)`;

const shadcnTheme = EditorView.theme({
  "&": {
    height: "100%",
    fontSize: "13.5px",
    color: c("foreground"),
    backgroundColor: c("background"),
  },
  "&.cm-focused": { outline: "none" },
  ".cm-scroller": {
    fontFamily: "'Jet Mono', 'Fira Code', 'Cascadia Code', Menlo, monospace",
    lineHeight: "1.6",
    overflow: "auto",
  },
  ".cm-content": {
    caretColor: c("foreground"),
    paddingBottom: "40vh",
  },
  ".cm-cursor, .cm-dropCursor": { borderLeftColor: c("foreground") },

  // Selection (selector mirrors CodeMirror's base theme so it wins on specificity)
  "&.cm-focused > .cm-scroller > .cm-selectionLayer .cm-selectionBackground, .cm-selectionBackground, .cm-content ::selection":
    { backgroundColor: tint("primary", 22) },
  ".cm-selectionMatch": { backgroundColor: tint("primary", 12) },

  // Gutters & active line (translucent so selection shows through)
  ".cm-gutters": {
    backgroundColor: c("background"),
    color: c("muted-foreground"),
    borderRight: `1px solid ${c("border")}`,
  },
  ".cm-activeLine": { backgroundColor: tint("muted", 60) },
  ".cm-activeLineGutter": {
    backgroundColor: tint("muted", 60),
    color: c("foreground"),
  },

  "&.cm-focused .cm-matchingBracket": {
    backgroundColor: tint("primary", 18),
    outline: `1px solid ${tint("primary", 40)}`,
  },

  // Popups/panels match shadcn popovers
  ".cm-tooltip": {
    backgroundColor: c("popover"),
    color: c("popover-foreground"),
    border: `1px solid ${c("border")}`,
    borderRadius: "calc(var(--radius) - 2px)",
  },
  ".cm-tooltip-autocomplete > ul > li[aria-selected]": {
    backgroundColor: c("accent"),
    color: c("accent-foreground"),
  },
  ".cm-panels": {
    backgroundColor: c("background"),
    color: c("foreground"),
    borderColor: c("border"),
  },
});

const shadcnHighlight = HighlightStyle.define([
  { tag: [t.keyword, t.modifier, t.operatorKeyword], color: c("cm-keyword") },
  { tag: [t.string, t.special(t.string)], color: c("cm-string") },
  { tag: [t.number, t.bool, t.null, t.atom], color: c("cm-number") },
  {
    tag: [t.comment, t.lineComment, t.blockComment],
    color: c("cm-comment"),
    fontStyle: "italic",
  },
  {
    tag: [t.function(t.variableName), t.function(t.propertyName)],
    color: c("cm-function"),
  },
  { tag: [t.heading], color: c("foreground"), fontWeight: "600" },
  { tag: t.emphasis, fontStyle: "italic" },
  { tag: t.strong, fontWeight: "600" },
  { tag: [t.link, t.url], color: c("primary"), textDecoration: "underline" },
  { tag: t.invalid, color: c("destructive") },
]);

const isDarkNow = () => document.documentElement.classList.contains("dark");

export interface UseCodeMirrorOptions {
  parent: HTMLElement;
  doc: string;
  readOnly?: boolean;
  onChange?: (value: string) => void;
  onSave?: () => void;
  /** Extra language/theme extensions (e.g. Typst support). */
  extraExtensions?: () => Extension[];
}

export function useCodeMirror(options: UseCodeMirrorOptions) {
  const readOnlyCompartment = new Compartment();
  const languageCompartment = new Compartment();
  const darkCompartment = new Compartment();

  const saveKeymap = keymap.of([
    {
      key: "Mod-s",
      preventDefault: true,
      run: () => (options.onSave?.(), true),
    },
  ]);

  const view = new EditorView({
    parent: options.parent,
    state: EditorState.create({
      doc: options.doc,
      extensions: [
        lineNumbers(),
        highlightActiveLineGutter(),
        highlightActiveLine(),
        history(),
        indentOnInput(),
        bracketMatching(),
        closeBrackets(),
        highlightSelectionMatches(),
        saveKeymap,
        keymap.of([...defaultKeymap, ...historyKeymap, indentWithTab]),
        shadcnTheme,
        syntaxHighlighting(shadcnHighlight),
        darkCompartment.of(EditorView.darkTheme.of(isDarkNow())),
        languageCompartment.of(options.extraExtensions?.() ?? []),
        readOnlyCompartment.of([
          EditorState.readOnly.of(options.readOnly ?? false),
          EditorView.editable.of(!(options.readOnly ?? false)),
        ]),
        EditorView.updateListener.of((update) => {
          if (update.docChanged)
            options.onChange?.(update.state.doc.toString());
        }),
      ],
    }),
  });

  let lastDark = isDarkNow();
  const observer = new MutationObserver(() => {
    const dark = isDarkNow();
    if (dark === lastDark) return;
    lastDark = dark;
    view.dispatch({
      effects: darkCompartment.reconfigure(EditorView.darkTheme.of(dark)),
    });
  });
  observer.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ["class"],
  });

  function setValue(value: string) {
    const current = view.state.doc.toString();
    if (current === value) return;
    view.dispatch({
      changes: { from: 0, to: current.length, insert: value },
    });
  }

  function getValue(): string {
    return view.state.doc.toString();
  }

  function setReadOnly(readOnly: boolean) {
    view.dispatch({
      effects: readOnlyCompartment.reconfigure([
        EditorState.readOnly.of(readOnly),
        EditorView.editable.of(!readOnly),
      ]),
    });
  }

  function setLanguageExtensions(extensions: Extension[]) {
    view.dispatch({
      effects: languageCompartment.reconfigure(extensions),
    });
  }

  let languageRequest = 0;

  async function setLanguageForFile(filename: string) {
    const request = ++languageRequest;
    const desc = findLanguage(filename);

    if (!desc) {
      view.dispatch({ effects: languageCompartment.reconfigure([]) });
      return;
    }

    // Already loaded? Apply synchronously, no flash of unhighlighted text
    const support = desc.support ?? (await desc.load());

    // A newer file was opened while this one was loading — drop the stale result
    if (request !== languageRequest) return;

    view.dispatch({ effects: languageCompartment.reconfigure(support) });
  }

  function jumpTo(offset: number, length: number) {
    const max = view.state.doc.length;
    const from = Math.min(Math.max(offset, 0), max);
    const to = Math.min(Math.max(from + length, from), max);
    view.dispatch({
      selection: EditorSelection.range(from, to),
      scrollIntoView: true,
    });
    view.focus();
  }

  function focus() {
    view.focus();
  }

  function destroy() {
    observer.disconnect();
    view.destroy();
  }

  return {
    view,
    setValue,
    getValue,
    setReadOnly,
    setLanguageExtensions,
    setLanguageForFile,
    jumpTo,
    focus,
    destroy,
  };
}
