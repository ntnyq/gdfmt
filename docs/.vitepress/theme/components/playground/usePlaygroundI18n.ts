import { useData } from 'vitepress'
import { computed } from 'vue'
import { playgroundMessages } from './i18n'

export function usePlaygroundI18n() {
  const { lang } = useData()
  const locale = computed(() => (lang.value.startsWith('zh') ? 'zh' : 'en'))
  const t = computed(() => playgroundMessages[locale.value])
  return { locale, t }
}
