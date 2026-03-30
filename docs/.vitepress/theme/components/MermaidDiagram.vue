<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue'

const props = defineProps<{
  code: string
}>()

const svg = ref('')
const error = ref('')

let observer: MutationObserver | null = null

async function renderDiagram() {
  try {
    error.value = ''

    const mermaid = (await import('mermaid')).default
    const isDark = document.documentElement.classList.contains('dark')

    mermaid.initialize({
      startOnLoad: false,
      securityLevel: 'loose',
      theme: isDark ? 'dark' : 'default',
      fontFamily: 'Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif',
      flowchart: {
        htmlLabels: true,
        useMaxWidth: true,
      },
    })

    const id = `rr-mermaid-${Math.random().toString(36).slice(2)}`
    const rendered = await mermaid.render(id, props.code)
    svg.value = rendered.svg
  } catch (err) {
    svg.value = ''
    error.value = err instanceof Error ? err.message : String(err)
  }
}

onMounted(() => {
  void renderDiagram()

  observer = new MutationObserver(() => {
    void renderDiagram()
  })

  observer.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ['class'],
  })
})

onBeforeUnmount(() => {
  observer?.disconnect()
})
</script>

<template>
  <div class="rr-mermaid-shell">
    <div v-if="svg" class="rr-mermaid-diagram" v-html="svg" />
    <pre v-else-if="error" class="rr-mermaid-error">{{ error }}</pre>
  </div>
</template>

<style scoped>
.rr-mermaid-shell {
  margin: 1.5rem 0;
  overflow-x: auto;
}

.rr-mermaid-diagram {
  padding: 1rem;
  border: 1px solid var(--vp-c-divider);
  border-radius: 12px;
  background: var(--vp-c-bg-soft);
}

.rr-mermaid-diagram:deep(svg) {
  display: block;
  max-width: 100%;
  height: auto;
  margin: 0 auto;
}

.rr-mermaid-diagram:deep(foreignObject) {
  overflow: visible;
}

.rr-mermaid-diagram:deep(.label),
.rr-mermaid-diagram:deep(.nodeLabel),
.rr-mermaid-diagram:deep(.edgeLabel) {
  line-height: 1.25;
}

.rr-mermaid-diagram:deep(.label p),
.rr-mermaid-diagram:deep(.nodeLabel p),
.rr-mermaid-diagram:deep(.edgeLabel p) {
  margin: 0;
  line-height: 1.25;
}

.rr-mermaid-diagram:deep(.label div),
.rr-mermaid-diagram:deep(.nodeLabel div),
.rr-mermaid-diagram:deep(.edgeLabel div) {
  line-height: 1.25;
}

.rr-mermaid-error {
  padding: 1rem;
  overflow-x: auto;
  border: 1px solid var(--vp-c-danger-2);
  border-radius: 12px;
  color: var(--vp-c-danger-1);
  background: var(--vp-c-bg-soft);
}
</style>
