<script setup lang="ts">
import {
  Check,
  CheckCheck,
  CircleAlert,
  Clock3,
  Code2,
  Copy,
  Download,
  LoaderCircle,
  Play,
  Settings2,
  ShieldCheck,
  Undo2,
} from '@lucide/vue'
import { computed, shallowRef, useTemplateRef } from 'vue'
import { Button } from '../ui/button'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '../ui/select'
import { Switch } from '../ui/switch'
import CodeEditor from './CodeEditor.vue'
import { examples } from './options'
import { byteOffsetToPosition } from './protocol'
import SettingsPanel from './SettingsPanel.vue'
import { usePlayground } from './usePlayground'
import { usePlaygroundI18n } from './usePlaygroundI18n'
import '../../playground.css'

const { t } = usePlaygroundI18n()
const {
  source,
  options,
  isLive,
  state,
  result,
  duration,
  error,
  isStale,
  output,
  canUseOutput,
  format,
  retry,
} = usePlayground()
const isSettingsOpen = shallowRef(true)
const selectedExample = shallowRef('player')
const feedback = shallowRef<
  'copyRequiresHttps' | 'outputCopied' | 'copyDenied' | null
>(null)
const sourceEditor = useTemplateRef('sourceEditor')
const isBusy = computed(
  () => state.value === 'loading' || state.value === 'formatting',
)
const diagnostics = computed(() => result.value?.diagnostics ?? [])
const sourceLines = computed(() =>
  source.value
    ? source.value.split('\n').length - Number(source.value.endsWith('\n'))
    : 0,
)
const outputLines = computed(() =>
  output.value
    ? output.value.split('\n').length - Number(output.value.endsWith('\n'))
    : 0,
)
const durationLabel = computed(() =>
  duration.value === null ? '—' : `${duration.value.toFixed(2)} ms`,
)
const status = computed(() => {
  if (state.value === 'loading') return t.value.status.loading
  if (state.value === 'formatting') return t.value.status.formatting
  if (state.value === 'error') return t.value.status.unavailable
  if (isStale.value) return t.value.status.waiting
  if (result.value?.output === null) return t.value.status.invalid
  if (result.value?.changed) return t.value.status.formatted
  return t.value.status.unchanged
})
const locatedDiagnostics = computed(() =>
  diagnostics.value.map(diagnostic => ({
    ...diagnostic,
    position: byteOffsetToPosition(source.value, diagnostic.span.start),
  })),
)

function selectExample(value: unknown): void {
  const example = examples.find(example => example.id === value)
  if (!example) return
  selectedExample.value = example.id
  source.value = example.source
  feedback.value = null
}

function resetSource(): void {
  source.value =
    examples.find(example => example.id === selectedExample.value)?.source ??
    examples[0].source
  feedback.value = null
}

async function copyOutput(): Promise<void> {
  feedback.value = null
  if (!canUseOutput.value) return
  if (!window.isSecureContext || !navigator.clipboard?.writeText) {
    feedback.value = 'copyRequiresHttps'
    return
  }
  try {
    await navigator.clipboard.writeText(output.value)
    feedback.value = 'outputCopied'
  } catch {
    feedback.value = 'copyDenied'
  }
}

function download(content: string, name: string, type: string): void {
  const url = URL.createObjectURL(new Blob([content], { type }))
  const anchor = document.createElement('a')
  anchor.href = url
  anchor.download = name
  anchor.click()
  setTimeout(() => URL.revokeObjectURL(url), 1000)
}

function downloadConfig(): void {
  download(
    `${JSON.stringify({ schemaVersion: 1, ...options.value }, null, 2)}\n`,
    '.gdfmtrc.json',
    'application/json',
  )
}
</script>

<template>
  <main
    class="playground"
    :aria-label="t.playground"
  >
    <div
      class="tw:mb-3 tw:flex tw:flex-wrap tw:items-center tw:justify-between tw:gap-3"
    >
      <div class="tw:flex tw:items-center tw:gap-2">
        <Code2 class="tw:size-4 tw:text-muted-foreground" />
        <div>
          <label
            class="tw:sr-only"
            for="pg-example"
            >{{ t.chooseExample }}</label
          >
          <Select
            :model-value="selectedExample"
            @update:model-value="selectExample"
          >
            <SelectTrigger
              id="pg-example"
              size="sm"
            >
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem
                v-for="example in examples"
                :key="example.id"
                :value="example.id"
              >
                {{ t.examples[example.id] }}
              </SelectItem>
            </SelectContent>
          </Select>
        </div>
        <Button
          variant="ghost"
          size="icon-sm"
          :title="t.resetSource"
          :aria-label="t.resetSource"
          @click="resetSource"
          ><Undo2 class="tw:size-4"
        /></Button>
      </div>
      <div class="tw:flex tw:flex-wrap tw:items-center tw:gap-3">
        <div class="tw:flex tw:items-center tw:gap-2">
          <Switch
            id="pg-live"
            v-model="isLive"
          />
          <label
            for="pg-live"
            class="tw:cursor-pointer tw:text-xs"
            >{{ t.liveFormatting }}</label
          >
        </div>
        <Button
          variant="outline"
          size="sm"
          :aria-expanded="isSettingsOpen"
          aria-controls="playground-settings"
          @click="isSettingsOpen = !isSettingsOpen"
          ><Settings2 class="tw:size-3.5" />{{ t.settings }}</Button
        >
        <Button
          size="sm"
          :disabled="isBusy || state === 'error'"
          @click="format"
          ><LoaderCircle
            v-if="isBusy"
            class="tw:size-3.5 tw:animate-spin tw:motion-reduce:animate-none"
          /><Play
            v-else
            class="tw:size-3.5"
          />{{ t.format }}</Button
        >
      </div>
    </div>

    <div
      class="pg-workspace"
      :class="{ 'with-settings': isSettingsOpen }"
    >
      <section
        class="tw:min-w-0"
        :aria-label="t.input"
      >
        <div
          class="tw:flex tw:h-12 tw:items-center tw:justify-between tw:border-b tw:border-border tw:px-4"
        >
          <div class="tw:flex tw:items-center tw:gap-2 tw:text-sm">
            <span
              class="tw:size-2 tw:rounded-full tw:bg-[var(--vp-c-brand-3)]"
            /><span class="tw:font-medium">{{ t.source }}</span
            ><span class="tw:ml-1 tw:text-xs tw:text-muted-foreground"
              >script.gd</span
            >
          </div>
          <span class="tw:text-xs tw:text-muted-foreground">{{
            t.editable
          }}</span>
        </div>
        <CodeEditor
          ref="sourceEditor"
          v-model="source"
          :options="options"
          :diagnostics="diagnostics"
          :label="t.sourceLabel"
          @format="format"
        />
      </section>
      <section
        class="pg-output tw:min-w-0 tw:border-l tw:border-border"
        :aria-label="t.outputLabel"
      >
        <div
          class="tw:flex tw:h-12 tw:items-center tw:justify-between tw:border-b tw:border-border tw:px-4"
        >
          <div class="tw:flex tw:items-center tw:gap-2 tw:text-sm">
            <CheckCheck class="tw:size-4 tw:text-[var(--vp-c-brand-1)]" />
            <h2 class="tw:font-medium">
              {{ t.formatted }}
            </h2>
          </div>
          <div class="tw:flex tw:items-center tw:gap-1">
            <Button
              variant="ghost"
              size="icon-sm"
              :disabled="!canUseOutput"
              :title="t.copyOutput"
              :aria-label="t.copyOutput"
              @click="copyOutput"
              ><Copy class="tw:size-3.5"
            /></Button>
            <Button
              variant="ghost"
              size="icon-sm"
              :disabled="!canUseOutput"
              :title="t.downloadOutput"
              :aria-label="t.downloadOutput"
              @click="download(output, 'script.formatted.gd', 'text/plain')"
              ><Download class="tw:size-3.5"
            /></Button>
          </div>
        </div>
        <div class="tw:relative">
          <CodeEditor
            :model-value="output"
            read-only
            :options="options"
            :label="t.formattedLabel"
          />
          <div
            v-if="!canUseOutput"
            class="tw:pointer-events-none tw:absolute tw:inset-0 tw:flex tw:items-center tw:justify-center tw:px-8"
          >
            <p
              class="tw:max-w-72 tw:text-center tw:text-sm tw:leading-6 tw:text-muted-foreground"
            >
              {{ status }}
            </p>
          </div>
        </div>
      </section>
      <SettingsPanel
        v-if="isSettingsOpen"
        v-model="options"
        @download="downloadConfig"
      />
    </div>

    <div
      class="tw:mt-3 tw:flex tw:flex-wrap tw:items-center tw:justify-between tw:gap-3 tw:text-xs tw:text-muted-foreground"
    >
      <div
        role="status"
        aria-live="polite"
        class="tw:flex tw:items-center tw:gap-2"
        data-testid="format-status"
      >
        <LoaderCircle
          v-if="isBusy"
          class="tw:size-3.5 tw:animate-spin tw:motion-reduce:animate-none"
        />
        <CircleAlert
          v-else-if="state === 'error' || result?.output === null"
          class="tw:size-3.5 tw:text-destructive"
        />
        <Check
          v-else
          class="tw:size-3.5 tw:text-[var(--vp-c-brand-1)]"
        />
        <span>{{ status }}</span>
      </div>
      <div class="tw:flex tw:flex-wrap tw:items-center tw:gap-5">
        <span
          >{{ sourceLines }} {{ t.linesIn }}<span class="tw:mx-2">→</span
          >{{ outputLines }} {{ t.linesOut }}</span
        >
        <span
          class="tw:flex tw:items-center tw:gap-1.5"
          :title="t.durationHint"
          ><Clock3 class="tw:size-3.5" /><span
            class="tw:tabular-nums"
            data-testid="format-duration"
            >{{ durationLabel }}</span
          ></span
        >
      </div>
    </div>

    <div
      v-if="error"
      role="alert"
      class="tw:mt-4 tw:flex tw:flex-wrap tw:items-center tw:justify-between tw:gap-3 tw:rounded-md tw:border tw:border-destructive/30 tw:bg-destructive/5 tw:p-4 tw:text-sm"
    >
      <div>
        <p>{{ t.errors[error.code] }}</p>
        <details
          v-if="error.detail"
          class="tw:mt-2 tw:text-xs tw:text-muted-foreground"
        >
          <summary class="tw:cursor-pointer">{{ t.errorDetails }}</summary>
          <p class="tw:mt-2 tw:break-words tw:font-mono">{{ error.detail }}</p>
        </details>
      </div>
      <Button
        variant="outline"
        size="sm"
        @click="retry"
        >{{ t.reloadFormatter }}</Button
      >
    </div>
    <section
      v-if="locatedDiagnostics.length"
      aria-live="polite"
      class="tw:mt-4 tw:rounded-md tw:border tw:border-destructive/30 tw:bg-destructive/5 tw:p-4"
      :aria-label="t.diagnostics"
    >
      <h2 class="tw:mb-2 tw:text-sm tw:font-medium">
        {{ t.diagnostics }}
        <span class="tw:text-muted-foreground"
          >({{ locatedDiagnostics.length }})</span
        >
      </h2>
      <ul class="tw:space-y-2">
        <li
          v-for="(diagnostic, index) in locatedDiagnostics"
          :key="index"
        >
          <button
            class="tw:text-left tw:text-xs tw:leading-6 tw:hover:underline"
            @click="sourceEditor?.revealDiagnostic(diagnostic)"
          >
            <span class="tw:mr-2 tw:font-mono tw:text-destructive"
              >{{ diagnostic.position.lineNumber }}:{{
                diagnostic.position.column
              }}
              {{ diagnostic.code }}</span
            >{{ diagnostic.message }}
          </button>
        </li>
      </ul>
    </section>
    <p
      v-if="feedback"
      role="status"
      class="tw:mt-3 tw:text-xs tw:text-[var(--vp-c-brand-1)]"
    >
      {{ t[feedback] }}
    </p>

    <footer
      class="tw:mt-6 tw:flex tw:flex-wrap tw:items-center tw:justify-between tw:gap-3 tw:border-t tw:border-border tw:pt-4 tw:text-xs tw:leading-5 tw:text-muted-foreground"
    >
      <p class="tw:flex tw:items-center tw:gap-2">
        <ShieldCheck class="tw:size-3.5 tw:shrink-0" />{{ t.privacy }}
      </p>
      <p>
        {{ t.shortcuts }}
      </p>
    </footer>
  </main>
</template>
