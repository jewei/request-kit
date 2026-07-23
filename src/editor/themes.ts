import { EditorView } from '@codemirror/view';
import { HighlightStyle, syntaxHighlighting } from '@codemirror/language';
import { tags } from '@lezer/highlight';

/**
 * CodeMirror theme driven by the app's CSS custom properties, so light/dark
 * flip together with the rest of the UI (PLAN.md "Theme").
 */
export const appTheme = EditorView.theme({
  '&': {
    backgroundColor: 'var(--rk-bg)',
    color: 'var(--rk-fg)',
    fontSize: '13px',
    height: '100%',
  },
  '.cm-content': {
    caretColor: 'var(--rk-fg)',
    fontFamily: "ui-monospace, 'SF Mono', Menlo, Consolas, monospace",
  },
  '.cm-gutters': {
    backgroundColor: 'var(--rk-bg)',
    color: 'var(--rk-muted)',
    borderRight: '1px solid var(--rk-border)',
  },
  '&.cm-focused': { outline: 'none' },
  '.cm-activeLine': { backgroundColor: 'color-mix(in srgb, var(--rk-accent) 6%, transparent)' },
  '.cm-selectionBackground, &.cm-focused .cm-selectionBackground': {
    backgroundColor: 'color-mix(in srgb, var(--rk-accent) 25%, transparent)',
  },
});

export const appHighlight = syntaxHighlighting(
  HighlightStyle.define([
    { tag: tags.propertyName, color: 'var(--rk-accent)' },
    { tag: tags.string, color: 'var(--rk-string, #16a34a)' },
    { tag: tags.number, color: 'var(--rk-number, #d97706)' },
    { tag: [tags.bool, tags.null], color: 'var(--rk-keyword, #9333ea)' },
    { tag: tags.punctuation, color: 'var(--rk-muted)' },
  ]),
);
