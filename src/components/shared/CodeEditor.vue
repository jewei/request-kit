<script setup lang="ts">
import { Compartment, EditorState } from '@codemirror/state';
import { EditorView } from '@codemirror/view';
import { onBeforeUnmount, onMounted, ref, watch } from 'vue';
import {
  baseExtensions,
  languageExtension,
  type EditorLanguage,
} from '../../editor/extensions';

const props = withDefaults(
  defineProps<{
    modelValue: string;
    language?: EditorLanguage;
    readonly?: boolean;
    lineNumbers?: boolean;
    placeholder?: string;
  }>(),
  { language: 'text', readonly: false, lineNumbers: true, placeholder: '' },
);
const emit = defineEmits<{ 'update:modelValue': [value: string] }>();

const host = ref<HTMLDivElement | null>(null);
let view: EditorView | null = null;
const languageCompartment = new Compartment();
const readonlyCompartment = new Compartment();

onMounted(() => {
  if (!host.value) return;
  view = new EditorView({
    parent: host.value,
    state: EditorState.create({
      doc: props.modelValue,
      extensions: [
        baseExtensions({
          lineNumbers: props.lineNumbers,
          placeholderText: props.placeholder,
        }),
        languageCompartment.of(languageExtension(props.language)),
        readonlyCompartment.of(EditorState.readOnly.of(props.readonly)),
        EditorView.updateListener.of((update) => {
          if (update.docChanged) {
            emit('update:modelValue', update.state.doc.toString());
          }
        }),
        EditorView.lineWrapping,
      ],
    }),
  });
});

onBeforeUnmount(() => {
  view?.destroy();
  view = null;
});

// External value changes are applied only when the editor lacks focus —
// while the user types, the editor itself is the source of truth.
watch(
  () => props.modelValue,
  (value) => {
    if (view && !view.hasFocus && value !== view.state.doc.toString()) {
      view.dispatch({
        changes: { from: 0, to: view.state.doc.length, insert: value },
      });
    }
  },
);

watch(
  () => props.language,
  (language) => {
    view?.dispatch({ effects: languageCompartment.reconfigure(languageExtension(language)) });
  },
);

watch(
  () => props.readonly,
  (readonly) => {
    view?.dispatch({
      effects: readonlyCompartment.reconfigure(EditorState.readOnly.of(readonly)),
    });
  },
);
</script>

<template>
  <div ref="host" class="code-editor" />
</template>

<style scoped>
.code-editor {
  height: 100%;
  min-height: 0;
  overflow: hidden;
  border: 1px solid var(--rk-border);
  border-radius: 4px;
}
.code-editor :deep(.cm-editor) {
  height: 100%;
}
.code-editor :deep(.cm-scroller) {
  overflow: auto;
}
</style>
