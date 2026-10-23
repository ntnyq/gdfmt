<script setup lang="ts">
import type { Diagnostic } from '@gdfmt/wasm'
import type { editor } from 'monaco-editor/editor/editor.api.js'
import { useData } from 'vitepress'
import { onMounted, onUnmounted, shallowRef, useTemplateRef, watch } from 'vue'
import type { PlaygroundOptions } from './options'
import { byteOffsetToPosition } from './protocol'
import { usePlaygroundI18n } from './usePlaygroundI18n'

interface Props {
  /** Accessible name for the editor. */
  label: string
  /** Whether edits are disallowed. */
  readOnly?: boolean
  /** Current formatting preferences for indentation and the width ruler. */
  options: PlaygroundOptions
  /** Diagnostics for the current input, expressed as UTF-8 byte spans. */
  diagnostics?: Diagnostic[]
}
const props = defineProps<Props>()
const { t } = usePlaygroundI18n()
const model = defineModel<string>({ required: true })
const emit = defineEmits<{ format: [] }>()
const container = useTemplateRef<HTMLDivElement>('container')
const { isDark } = useData()
const isLoaded = shallowRef(false)
const loadError = shallowRef('')
let instance: editor.IStandaloneCodeEditor | undefined
let textModel: editor.ITextModel | undefined
let subscription: { dispose(): void } | undefined
let isDisposed = false
let api: typeof import('monaco-editor/editor/editor.api.js') | undefined

function updateMarkers(): void {
  if (!api || !textModel) return
  const { MarkerSeverity } = api
  api.editor.setModelMarkers(
    textModel,
    'gdfmt',
    (props.diagnostics ?? []).map(diagnostic => {
      const start = byteOffsetToPosition(model.value, diagnostic.span.start)
      const end = byteOffsetToPosition(model.value, diagnostic.span.end)
      return {
        startLineNumber: start.lineNumber,
        startColumn: start.column,
        endLineNumber: end.lineNumber,
        endColumn: Math.max(
          end.column,
          start.lineNumber === end.lineNumber ? start.column + 1 : 1,
        ),
        severity:
          diagnostic.severity === 'error'
            ? MarkerSeverity.Error
            : diagnostic.severity === 'warning'
              ? MarkerSeverity.Warning
              : MarkerSeverity.Info,
        message: diagnostic.message,
        code: diagnostic.code,
        source: 'gdfmt',
      }
    }),
  )
}

function revealDiagnostic(diagnostic: Diagnostic): void {
  const position = byteOffsetToPosition(model.value, diagnostic.span.start)
  instance?.setPosition(position)
  instance?.revealPositionInCenter(position)
  instance?.focus()
}

watch(model, value => {
  if (textModel && value !== textModel.getValue()) textModel.setValue(value)
})
watch(isDark, value =>
  api?.editor.setTheme(value ? 'gdfmt-dark' : 'gdfmt-light'),
)
watch(
  () => props.options,
  value => {
    instance?.updateOptions({
      rulers: props.readOnly ? [] : [value.format.lineWidth],
    })
    textModel?.updateOptions({
      tabSize: value.format.indentWidth,
      insertSpaces: value.format.indentStyle === 'space',
    })
  },
)
watch(() => props.diagnostics, updateMarkers)
watch(
  () => props.label,
  value => instance?.updateOptions({ ariaLabel: value }),
)

onMounted(async () => {
  try {
    const { monaco } = await import('./monaco')
    if (isDisposed || !container.value) return
    api = monaco
    textModel = monaco.editor.createModel(model.value, 'gdscript')
    textModel.updateOptions({
      tabSize: props.options.format.indentWidth,
      insertSpaces: props.options.format.indentStyle === 'space',
    })
    instance = monaco.editor.create(container.value, {
      model: textModel,
      theme: isDark.value ? 'gdfmt-dark' : 'gdfmt-light',
      readOnly: props.readOnly,
      ariaLabel: props.label,
      automaticLayout: true,
      minimap: { enabled: false },
      fontFamily: '"JetBrains Mono", "SFMono-Regular", Consolas, monospace',
      fontSize: 13,
      lineHeight: 23,
      padding: { top: 20, bottom: 20 },
      scrollBeyondLastLine: false,
      renderLineHighlight: props.readOnly ? 'none' : 'line',
      lineNumbersMinChars: 3,
      rulers: props.readOnly ? [] : [props.options.format.lineWidth],
      wordWrap: 'off',
      folding: true,
      tabFocusMode: true,
      quickSuggestions: false,
      suggestOnTriggerCharacters: false,
      contextmenu: false,
    })
    subscription = instance.onDidChangeModelContent(() => {
      if (!props.readOnly && textModel) model.value = textModel.getValue()
    })
    instance.addAction({
      id: 'gdfmt.format',
      label: t.value.formatAction,
      keybindings: [monaco.KeyMod.CtrlCmd | monaco.KeyCode.Enter],
      run: () => emit('format'),
    })
    isLoaded.value = true
    updateMarkers()
  } catch (error) {
    loadError.value = error instanceof Error ? error.message : String(error)
  }
})

onUnmounted(() => {
  isDisposed = true
  subscription?.dispose()
  instance?.dispose()
  textModel?.dispose()
})

defineExpose({ revealDiagnostic })
</script>

<template>
  <div
    class="pg-editor tw:relative"
    :data-editor="readOnly ? 'output' : 'source'"
  >
    <div
      ref="container"
      class="tw:h-full tw:w-full"
    />
    <div
      v-if="!isLoaded"
      class="tw:absolute tw:inset-0 tw:flex tw:flex-col tw:bg-background"
    >
      <p
        class="tw:px-4 tw:py-2 tw:text-xs tw:text-muted-foreground"
        role="status"
      >
        {{ loadError ? t.editorUnavailable : t.loadingEditor }}
      </p>
      <textarea
        v-model="model"
        :aria-label="label"
        :readonly="readOnly"
        spellcheck="false"
        class="tw:h-full tw:w-full tw:resize-none tw:bg-background tw:p-4 tw:font-mono tw:text-sm tw:leading-6 tw:outline-none"
        @keydown.ctrl.enter.prevent="emit('format')"
        @keydown.meta.enter.prevent="emit('format')"
      />
    </div>
  </div>
</template>
