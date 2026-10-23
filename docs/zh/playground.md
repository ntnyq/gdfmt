---
layout: page
sidebar: false
aside: false
title: 演练场
description: 在浏览器中体验 gdfmt，编辑 GDScript、调整格式化选项，实时查看经过验证的结果。
footer: false
---

<script setup lang="ts">
import Playground from '../.vitepress/theme/components/playground/Playground.vue'
</script>

<ClientOnly>
  <Playground />
  <template #fallback>
    <p style="padding: 48px; text-align: center">正在加载演练场… 此页面需要启用 JavaScript。</p>
  </template>
</ClientOnly>
