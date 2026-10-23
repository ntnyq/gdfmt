import type { FormatResult } from '@gdfmt/wasm'
import { computed, onMounted, onUnmounted, shallowRef, watch } from 'vue'
import { createDefaultOptions, examples } from './options'
import type { FormatRequest, PlaygroundError, WorkerResponse } from './protocol'

export function usePlayground() {
  const source = shallowRef(examples[0].source)
  const options = shallowRef(createDefaultOptions())
  const isLive = shallowRef(true)
  const state = shallowRef<'loading' | 'ready' | 'formatting' | 'error'>(
    'loading',
  )
  const result = shallowRef<FormatResult | null>(null)
  const duration = shallowRef<number | null>(null)
  const error = shallowRef<PlaygroundError | null>(null)
  const isStale = shallowRef(false)
  let worker: Worker | undefined
  let debounce: ReturnType<typeof setTimeout> | undefined
  let deadline: ReturnType<typeof setTimeout> | undefined
  let revision = 0
  let isReady = false

  const output = computed(() => result.value?.output ?? '')
  const canUseOutput = computed(
    () =>
      result.value?.output != null && !isStale.value && state.value === 'ready',
  )

  function fail(code: PlaygroundError['code'], detail?: string): void {
    clearTimeout(deadline)
    isReady = false
    worker?.terminate()
    worker = undefined
    state.value = 'error'
    error.value = { code, detail }
    result.value = null
    duration.value = null
  }

  function format(): void {
    clearTimeout(debounce)
    if (!worker || !isReady) return
    const request: FormatRequest = {
      id: ++revision,
      source: source.value,
      options: JSON.parse(JSON.stringify(options.value)),
    }
    state.value = 'formatting'
    error.value = null
    clearTimeout(deadline)
    deadline = setTimeout(() => fail('formatTimeout'), 15_000)
    worker.postMessage(request)
  }

  function startWorker(): void {
    worker?.terminate()
    clearTimeout(deadline)
    isReady = false
    revision++
    state.value = 'loading'
    error.value = null
    result.value = null
    duration.value = null
    try {
      worker = new Worker(new URL('./formatter.worker.ts', import.meta.url), {
        type: 'module',
      })
      deadline = setTimeout(() => fail('loadTimeout'), 30_000)
      worker.onmessage = ({ data }: MessageEvent<WorkerResponse>) => {
        if (data.type === 'ready') {
          clearTimeout(deadline)
          isReady = true
          format()
          return
        }
        if (data.id !== null && data.id !== revision) return
        clearTimeout(deadline)
        if (data.type === 'error') {
          fail(data.id === null ? 'loadFailed' : 'formatFailed', data.message)
          return
        }
        result.value = data.result
        duration.value = data.duration
        isStale.value = false
        state.value = 'ready'
      }
      worker.onerror = () => fail(isReady ? 'workerStopped' : 'loadFailed')
      worker.onmessageerror = () => fail('unreadableResponse')
    } catch (cause) {
      fail('loadFailed', cause instanceof Error ? cause.message : String(cause))
    }
  }

  watch(
    [source, options],
    () => {
      revision++
      isStale.value = true
      result.value = null
      duration.value = null
      clearTimeout(debounce)
      clearTimeout(deadline)
      if (isReady) state.value = 'ready'
      if (isLive.value) debounce = setTimeout(format, 250)
    },
    { flush: 'sync' },
  )

  watch(isLive, value => {
    clearTimeout(debounce)
    if (value && isStale.value) format()
  })

  onMounted(startWorker)
  onUnmounted(() => {
    clearTimeout(debounce)
    clearTimeout(deadline)
    worker?.terminate()
  })

  return {
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
    retry: startWorker,
  }
}
