// Taken from https://github.com/prisma/text-editors/blob/main/src/extensions/typescript/index.ts
// which is licensed under Apache 2.0. With additional modifications.
import {
  autocompletion,
  completeFromList,
  CompletionContext,
  type CompletionResult,
} from '@codemirror/autocomplete';
import { javascript } from '@codemirror/lang-javascript';
import { type Diagnostic, linter, setDiagnostics as cmSetDiagnostics } from '@codemirror/lint';
import {
  EditorState,
  type Extension,
  StateEffect,
  StateField,
  type TransactionSpec,
} from '@codemirror/state';
import { hoverTooltip, type Tooltip } from '@codemirror/tooltip';
import { EditorView } from '@codemirror/view';
import { throttle } from 'lodash-es';
import { DiagnosticCategory, displayPartsToString, flattenDiagnosticMessageText } from 'typescript';
import { onChangeCallback } from '../change-callback';
import { log } from './log';
import { type FileMap, TypescriptProject } from './project';

export { TypescriptProject };
export type { FileMap };
export type WrapCodeFn = () => { prefix?: string; suffix?: string };

/**
 * This file exports an extension that makes Typescript language services work. This includes:
 *
 * 1. A StateField that holds an instance of a `TypescriptProject` (used to communicate with tsserver)
 * 2. A StateField that stores ranges for lint diagostics (used to cancel hover tooltips if a lint diagnistic is also present at the position)
 * 3. A `javascript` extension, that provides syntax highlighting and other simple JS features.
 * 4. An `autocomplete` extension that provides tsserver-backed completions, powered by the `completionSource` function
 * 5. A `linter` extension that provides tsserver-backed type errors, powered by the `lintDiagnostics` function
 * 6. A `hoverTooltip` extension that provides tsserver-backed type information on hover, powered by the `hoverTooltip` function
 * 7. An `updateListener` (facet) extension, that ensures that the editor's view is kept in sync with tsserver's view of the file
 * 8. A StateEffect that lets a consumer inject custom types into the `TypescriptProject`
 *
 * The "correct" way to read this file is from bottom to top.
 */

export interface TsStateField {
  project: TypescriptProject;
  prefix: string;
  suffix: string;
}

/**
 * A State field that represents the Typescript project that is currently "open" in the EditorView
 */
const tsStateField = StateField.define<TsStateField>({
  create(state) {
    return {
      project: new TypescriptProject(state.sliceDoc(0)),
      prefix: '',
      suffix: '',
    };
  },

  update(ts, transaction) {
    // For all transactions that run, this state field's value will only "change" if a `injectTypesEffect` StateEffect is attached to the transaction
    transaction.effects.forEach((e) => {
      if (e.is(injectTypesEffect)) {
        ts.project.injectTypes(e.value);
      }
    });

    return ts;
  },

  compare() {
    // There must never be two instances of this state field
    return true;
  },
});

/**
 * A CompletionSource that returns completions to show at the current cursor position (via tsserver)
 */
const completionSource = async (ctx: CompletionContext): Promise<CompletionResult | null> => {
  let { state, pos } = ctx;
  const ts = state.field(tsStateField);

  pos += ts.prefix.length;

  try {
    const completions = (await ts.project.lang()).getCompletionsAtPosition(
      ts.project.entrypoint,
      pos,
      {}
    );
    if (!completions) {
      log('Unable to get completions', { pos });
      return null;
    }

    let beforeDot = ctx.matchBefore(/\./);
    if (beforeDot) {
      // Force explicit to true if the user just typed a period, so that autocompletion
      // will show up.
      ctx = new CompletionContext(ctx.state, pos, true);
    }

    return completeFromList(
      completions.entries.map((c, i) => ({
        type: c.kind,
        label: c.name,
        boost: 1 / Number(c.sortText),
      }))
    )(ctx);
  } catch (e) {
    log('Unable to get completions', { pos, error: e });
    return null;
  }
};

/**
 * A LintSource that returns lint diagnostics across the current editor view (via tsserver)
 */
const lintDiagnostics = async (state: EditorState): Promise<Diagnostic[]> => {
  const ts = state.field(tsStateField);
  const diagnostics = (await ts.project.lang()).getSemanticDiagnostics(ts.project.entrypoint);

  return diagnostics
    .filter((d) => d.start !== undefined && d.length !== undefined)
    .map((d) => {
      let severity: 'info' | 'warning' | 'error' = 'info';
      if (d.category === DiagnosticCategory.Error) {
        severity = 'error';
      } else if (d.category === DiagnosticCategory.Warning) {
        severity = 'warning';
      }

      let start = (d.start || 0) - ts.prefix.length;

      return {
        from: start, // `!` is fine because of the `.filter()` before the `.map()`
        to: start + d.length!, // `!` is fine because of the `.filter()` before the `.map()`
        severity,
        message: flattenDiagnosticMessageText(d.messageText, '\n', 0),
      };
    });
};

/**
 * A HoverTooltipSource that returns a Tooltip to show at a given cursor position (via tsserver)
 */
const hoverTooltipSource = async (state: EditorState, pos: number): Promise<Tooltip | null> => {
  const ts = state.field(tsStateField);

  const quickInfo = (await ts.project.lang()).getQuickInfoAtPosition(
    ts.project.entrypoint,
    pos + ts.prefix.length
  );
  if (!quickInfo) {
    return null;
  }

  return {
    pos,
    create() {
      const dom = document.createElement('div');
      dom.setAttribute('class', 'cm-quickinfo-tooltip');
      dom.textContent =
        displayPartsToString(quickInfo.displayParts) +
        (quickInfo.documentation?.length
          ? '\n' + displayPartsToString(quickInfo.documentation)
          : '');

      return {
        dom,
      };
    },
    above: false, // HACK: This makes it so lint errors show up on TOP of this, so BOTH quickInfo and lint tooltips don't show up at the same time
  };
};

/**
 * A TransactionSpec that can be dispatched to add new types to the underlying tsserver instance
 */
const injectTypesEffect = StateEffect.define<FileMap>();
export function injectTypes(types: FileMap): TransactionSpec {
  return {
    effects: [injectTypesEffect.of(types)],
  };
}

/**
 * A TransactionSpec that can be dispatched to force re-calculation of lint diagnostics
 */
export async function setDiagnostics(state: EditorState): Promise<TransactionSpec> {
  const diagnostics = await lintDiagnostics(state);
  return cmSetDiagnostics(state, diagnostics);
}

/**
 * A (throttled) function that updates the view of the currently open "file" on TSServer
 */
const updateTSFileThrottled = throttle((view: EditorView) => {
  const ts = view.state.field(tsStateField);
  // Update tsserver's view of this file
  let code = view.state.sliceDoc(0);
  code = `${ts.prefix}${code}${ts.suffix}`;

  // Don't `await` because we do not want to block
  ts.project.env().then((env) => env.updateFile(ts.project.entrypoint, code || ' ')); // tsserver deletes the file if the text content is empty; we can't let that happen
}, 100);

/** Export a function that will build & return an Extension
 * @param wrapCode a function provides a prefix and suffix in which the code will be wrapped.
 */
export function typescript(wrapCode?: WrapCodeFn): Extension {
  return [
    tsStateField.init((state) => {
      let wrapper = wrapCode?.() ?? {};

      let code = state.sliceDoc(0);
      let prefix = wrapper.prefix ?? '';
      let suffix = wrapper.suffix ?? '';

      code = `${prefix}${code}${suffix}`;

      return {
        project: new TypescriptProject(code),
        prefix,
        suffix,
      };
    }),
    javascript({ typescript: true, jsx: false }),
    autocompletion({
      activateOnTyping: true,
      maxRenderedOptions: 30,
      override: [completionSource],
    }),
    linter((view) => lintDiagnostics(view.state)),
    hoverTooltip((view, pos) => hoverTooltipSource(view.state, pos), {
      hideOnChange: true,
    }),
    EditorView.updateListener.of(({ view, docChanged }) => {
      // We're not doing this in the `onChangeCallback` extension because we do not want TS file updates to be debounced (we want them throttled)
      if (docChanged) {
        updateTSFileThrottled(view);
      }
    }),
    onChangeCallback(async (_code, view) => {
      // No need to debounce here because this callback is already debounced

      // Re-compute lint diagnostics via tsserver
      view.dispatch(await setDiagnostics(view.state));
    }),
  ];
}
