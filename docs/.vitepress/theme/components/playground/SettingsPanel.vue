<script setup lang="ts">
import type { FormatOptions } from '@gdfmt/wasm'
import { RotateCcw, Download, Settings2 } from '@lucide/vue'
import { computed } from 'vue'
import { Button } from '../ui/button'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '../ui/select'
import { Switch } from '../ui/switch'
import { createDefaultOptions, optionFields } from './options'
import type { PlaygroundOptions } from './options'
import { usePlaygroundI18n } from './usePlaygroundI18n'

const { t } = usePlaygroundI18n()
const options = defineModel<PlaygroundOptions>({ required: true })
const emit = defineEmits<{ download: [] }>()

const valueLabels = computed(() => new Map(Object.entries(t.value.values)))

function updateFormat(key: keyof FormatOptions, value: unknown): void {
  if (typeof value !== 'string') return
  const field = optionFields.find(field => field.key === key)
  const parsed = field?.values
    ? field.values.find(choice => String(choice) === value)
    : Number(value)
  if (parsed === undefined) return
  if (
    typeof parsed === 'number' &&
    (!Number.isInteger(parsed) ||
      (field?.min !== undefined && parsed < field.min) ||
      (field?.max !== undefined && parsed > field.max))
  ) {
    return
  }
  options.value = {
    ...options.value,
    format: { ...options.value.format, [key]: parsed },
  }
}

function updateLineWidth(event: Event): void {
  const target = event.target
  if (!(target instanceof HTMLInputElement)) return
  updateFormat('lineWidth', target.value)
  target.value = String(options.value.format.lineWidth)
}

function updateVersion(value: unknown): void {
  if (
    value !== '4.3' &&
    value !== '4.4' &&
    value !== '4.5' &&
    value !== '4.6' &&
    value !== '4.7'
  )
    return
  options.value = { ...options.value, language: { godotVersion: value } }
}

function updateFinalNewline(value: boolean): void {
  options.value = {
    ...options.value,
    format: { ...options.value.format, finalNewline: value },
  }
}
</script>

<template>
  <aside
    id="playground-settings"
    class="pg-settings tw:min-w-0 tw:border-l tw:border-border tw:bg-secondary/40"
    :aria-label="t.formatterSettings"
  >
    <div
      class="tw:flex tw:h-12 tw:items-center tw:justify-between tw:border-b tw:border-border tw:px-4"
    >
      <h2 class="tw:flex tw:items-center tw:gap-2 tw:text-sm tw:font-medium">
        <Settings2 class="tw:size-4" />{{ t.configuration }}
      </h2>
      <Button
        variant="ghost"
        size="icon-sm"
        :title="t.resetOptions"
        :aria-label="t.resetOptions"
        @click="options = createDefaultOptions()"
        ><RotateCcw class="tw:size-3.5"
      /></Button>
    </div>
    <div
      class="pg-settings-body tw:max-h-[max(460px,calc(100dvh-240px))] tw:overflow-y-auto tw:p-4"
    >
      <section>
        <label
          for="pg-version"
          class="tw:mb-2 tw:block tw:text-xs tw:font-medium"
          >{{ t.languageVersion }}</label
        >
        <Select
          :model-value="options.language.godotVersion"
          @update:model-value="updateVersion"
        >
          <SelectTrigger
            id="pg-version"
            class="tw:w-full tw:gap-1 tw:px-2 tw:text-[13px]"
            size="sm"
          >
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem
              v-for="version in ['4.7', '4.6', '4.5', '4.4', '4.3']"
              :key="version"
              :value="version"
            >
              Godot {{ version }}
            </SelectItem>
          </SelectContent>
        </Select>
        <p
          class="tw:mt-2 tw:mb-5 tw:text-xs tw:leading-5 tw:text-muted-foreground"
        >
          {{ t.languageHint }}
        </p>
        <div
          v-for="field in optionFields.slice(0, 5)"
          :key="field.key"
          class="tw:mb-4 tw:grid tw:grid-cols-[1fr_100px] tw:items-center tw:gap-3"
        >
          <label
            :for="`pg-${field.key}`"
            class="tw:text-xs"
            >{{ t.fields[field.key] }}</label
          >
          <Select
            v-if="field.values"
            :model-value="String(options.format[field.key])"
            @update:model-value="updateFormat(field.key, $event)"
          >
            <SelectTrigger
              :id="`pg-${field.key}`"
              class="tw:w-full tw:gap-1 tw:px-2 tw:text-[13px]"
              size="sm"
            >
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem
                v-for="value in field.values"
                :key="value"
                :value="String(value)"
              >
                {{ valueLabels.get(String(value)) ?? value }}
              </SelectItem>
            </SelectContent>
          </Select>
          <input
            v-else
            :id="`pg-${field.key}`"
            class="pg-control"
            type="number"
            :min="field.min"
            :max="field.max"
            :value="options.format[field.key]"
            @change="updateLineWidth"
          />
        </div>
        <div
          class="tw:mb-5 tw:flex tw:items-center tw:justify-between tw:gap-3"
        >
          <label
            for="pg-final-newline"
            class="tw:text-xs"
            >{{ t.finalNewline }}</label
          >
          <Switch
            id="pg-final-newline"
            :model-value="options.format.finalNewline"
            @update:model-value="updateFinalNewline"
          />
        </div>
      </section>
      <section class="tw:border-t tw:border-border tw:pt-4">
        <h3
          class="tw:mb-4 tw:text-xs tw:font-semibold tw:text-muted-foreground"
        >
          {{ t.layoutOptions }}
        </h3>
        <div
          v-for="field in optionFields.slice(5)"
          :key="field.key"
          class="tw:mb-4 tw:grid tw:grid-cols-[1fr_100px] tw:items-center tw:gap-3"
        >
          <label
            :for="`pg-${field.key}`"
            class="tw:text-xs"
            >{{ t.fields[field.key] }}</label
          >
          <Select
            :model-value="String(options.format[field.key])"
            @update:model-value="updateFormat(field.key, $event)"
          >
            <SelectTrigger
              :id="`pg-${field.key}`"
              class="tw:w-full tw:gap-1 tw:px-2 tw:text-[13px]"
              size="sm"
            >
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem
                v-for="value in field.values"
                :key="value"
                :value="String(value)"
              >
                {{ valueLabels.get(String(value)) ?? value }}
              </SelectItem>
            </SelectContent>
          </Select>
        </div>
      </section>
      <Button
        variant="outline"
        size="sm"
        class="tw:mt-2 tw:w-full"
        @click="emit('download')"
        ><Download class="tw:size-3.5" />{{ t.exportConfig }}</Button
      >
    </div>
  </aside>
</template>
