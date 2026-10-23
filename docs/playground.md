---
layout: page
sidebar: false
aside: false
title: Playground
description: Try gdfmt in your browser. Edit GDScript, explore formatting options, and see validated output in real time.
footer: false
---

<script setup lang="ts">
import Playground from './.vitepress/theme/components/playground/Playground.vue'
</script>

<ClientOnly>
  <Playground />
  <template #fallback>
    <p style="padding: 48px; text-align: center">Loading playground… JavaScript is required.</p>
  </template>
</ClientOnly>
