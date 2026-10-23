<script setup lang="ts">
import { reactiveOmit } from '@vueuse/core'
import type { SelectContentEmits, SelectContentProps } from 'reka-ui'
import {
  SelectContent,
  SelectPortal,
  SelectViewport,
  useForwardPropsEmits,
} from 'reka-ui'
import type { HTMLAttributes } from 'vue'
import { SelectScrollDownButton, SelectScrollUpButton } from '.'
import { cn } from '../../../lib/utils'

defineOptions({
  inheritAttrs: false,
})

const props = withDefaults(
  defineProps<SelectContentProps & { class?: HTMLAttributes['class'] }>(),
  {
    position: 'popper',
  },
)
const emits = defineEmits<SelectContentEmits>()

const delegatedProps = reactiveOmit(props, 'class')

const forwarded = useForwardPropsEmits(delegatedProps, emits)
</script>

<template>
  <SelectPortal>
    <SelectContent
      data-slot="select-content"
      v-bind="{ ...$attrs, ...forwarded }"
      :class="
        cn(
          'pg-select-content tw:bg-popover tw:text-popover-foreground tw:relative tw:z-50 tw:max-h-(--reka-select-content-available-height) tw:min-w-[8rem] tw:overflow-x-hidden tw:overflow-y-auto tw:rounded-md tw:border tw:shadow-md',
          position === 'popper' &&
            'tw:data-[side=bottom]:translate-y-1 tw:data-[side=left]:-translate-x-1 tw:data-[side=right]:translate-x-1 tw:data-[side=top]:-translate-y-1',
          props.class,
        )
      "
    >
      <SelectScrollUpButton />
      <SelectViewport
        :class="
          cn(
            'tw:p-1',
            position === 'popper' &&
              'tw:h-(--reka-select-trigger-height) tw:w-full tw:min-w-(--reka-select-trigger-width) tw:scroll-my-1',
          )
        "
      >
        <slot />
      </SelectViewport>
      <SelectScrollDownButton />
    </SelectContent>
  </SelectPortal>
</template>
