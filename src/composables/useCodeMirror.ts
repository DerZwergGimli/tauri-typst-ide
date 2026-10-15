import { EditorView, keymap, lineNumbers, highlightActiveLine, highlightActiveLineGutter } from "@codemirror/view";
import { EditorState, Compartment, EditorSelection, type Extension } from "@codemirror/state";
import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
import { bracketMatching, indentOnInput } from "@codemirror/language";
import { highlightSelectionMatches } from "@codemirror/search";
import { closeBrackets } from "@codemirror/autocomplete";

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

  const saveKeymap = keymap.of([
    {
      key: "Mod-s",
      preventDefault: true,
      run: () => {
        options.onSave?.();
        return true;
      },
    },
  ]);

  const theme = EditorView.theme({
    "&": {
      height: "100%",
      fontSize: "13.5px",
    },
    ".cm-scroller": {
      fontFamily:
        "'Jet Mono', 'Fira Code', 'Cascadia Code', Menlo, monospace",
      lineHeight: "1.6",
      overflow: "auto",
    },
    ".cm-content": {
      paddingBottom: "40vh",
    },
    "&.cm-focused": {
      outline: "none",
    },
  });

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
        theme,
        languageCompartment.of(options.extraExtensions?.() ?? []),
        readOnlyCompartment.of([
          EditorState.readOnly.of(options.readOnly ?? false),
          EditorView.editable.of(!(options.readOnly ?? false)),
        ]),
        EditorView.updateListener.of((update) => {
          if (update.docChanged) {
            options.onChange?.(update.state.doc.toString());
          }
        }),
      ],
    }),
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
    view.destroy();
  }

  return {
    view,
    setValue,
    getValue,
    setReadOnly,
    setLanguageExtensions,
    jumpTo,
    focus,
    destroy,
  };
}
