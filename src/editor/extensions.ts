import { defaultKeymap, history, historyKeymap } from '@codemirror/commands';
import { json } from '@codemirror/lang-json';
import { bracketMatching } from '@codemirror/language';
import { highlightSelectionMatches, searchKeymap } from '@codemirror/search';
import type { Extension } from '@codemirror/state';
import { Prec } from '@codemirror/state';
import { keymap, lineNumbers as lineNumbersExt, placeholder } from '@codemirror/view';
import { appHighlight, appTheme } from './themes';

export type EditorLanguage = 'text' | 'json';

export function languageExtension(language: EditorLanguage): Extension {
  switch (language) {
    case 'json':
      return [json(), bracketMatching()];
    case 'text':
      return [];
  }
}

/**
 * Mod-Enter must reach the window-level hotkey listener (send), never be
 * consumed by the editor — a highest-precedence no-op binding that returns
 * false lets the event bubble untouched past any default keymap entries.
 */
const passThroughSend = Prec.highest(
  keymap.of([{ key: 'Mod-Enter', run: () => false }]),
);

export function baseExtensions(options: {
  lineNumbers: boolean;
  placeholderText?: string;
}): Extension {
  const extensions: Extension[] = [
    appTheme,
    appHighlight,
    history(),
    highlightSelectionMatches(),
    passThroughSend,
    keymap.of([...defaultKeymap, ...historyKeymap, ...searchKeymap]),
  ];
  if (options.lineNumbers) {
    extensions.push(lineNumbersExt());
  }
  if (options.placeholderText !== undefined && options.placeholderText !== '') {
    extensions.push(placeholder(options.placeholderText));
  }
  return extensions;
}
