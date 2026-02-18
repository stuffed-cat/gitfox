<template>
  <div ref="containerRef" class="monaco-diff-editor-container" :style="{ height: heightProp }"></div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch, computed } from 'vue'
import * as monaco from 'monaco-editor'

interface Props {
  original: string
  modified: string
  language?: string
  theme?: string
  height?: string | number
  readonly?: boolean
  renderSideBySide?: boolean
  minimap?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  language: 'plaintext',
  theme: 'vs-dark',
  height: 400,
  readonly: true,
  renderSideBySide: true,
  minimap: false,
})

const emit = defineEmits<{
  'editor-mounted': [editor: monaco.editor.IStandaloneDiffEditor]
}>()

const containerRef = ref<HTMLDivElement>()
const diffEditor = ref<monaco.editor.IStandaloneDiffEditor>()

const heightProp = computed(() => {
  return typeof props.height === 'number' ? `${props.height}px` : props.height
})

function createDiffEditor() {
  if (!containerRef.value) return

  // 先设置主题
  monaco.editor.setTheme(props.theme)

  const diffEditorOptions: monaco.editor.IStandaloneDiffEditorConstructionOptions = {
    readOnly: props.readonly,
    renderSideBySide: props.renderSideBySide,
    minimap: { enabled: props.minimap },
    automaticLayout: true,
    scrollBeyondLastLine: false,
    renderLineHighlight: 'all',
    scrollbar: {
      verticalScrollbarSize: 10,
      horizontalScrollbarSize: 10,
    },
    fontSize: 13,
    lineHeight: 19,
    padding: { top: 8, bottom: 8 },
    enableSplitViewResizing: true,
    renderOverviewRuler: true,
    diffWordWrap: 'on',
  }

  diffEditor.value = monaco.editor.createDiffEditor(containerRef.value, diffEditorOptions)

  const originalModel = monaco.editor.createModel(props.original, props.language)
  const modifiedModel = monaco.editor.createModel(props.modified, props.language)

  diffEditor.value.setModel({
    original: originalModel,
    modified: modifiedModel,
  })

  emit('editor-mounted', diffEditor.value)
}

function disposeDiffEditor() {
  const model = diffEditor.value?.getModel()
  model?.original?.dispose()
  model?.modified?.dispose()
  diffEditor.value?.dispose()
  diffEditor.value = undefined
}

function updateModels() {
  if (!diffEditor.value) return

  const currentModel = diffEditor.value.getModel()
  currentModel?.original?.dispose()
  currentModel?.modified?.dispose()

  const originalModel = monaco.editor.createModel(props.original, props.language)
  const modifiedModel = monaco.editor.createModel(props.modified, props.language)

  diffEditor.value.setModel({
    original: originalModel,
    modified: modifiedModel,
  })
}

// 暴露方法
defineExpose({
  getDiffEditor: () => diffEditor.value,
  layout: () => diffEditor.value?.layout(),
})

// 监听内容变化
watch([() => props.original, () => props.modified, () => props.language], () => {
  updateModels()
})

watch(() => props.theme, (newTheme) => {
  monaco.editor.setTheme(newTheme)
})

watch(() => props.renderSideBySide, (newValue) => {
  diffEditor.value?.updateOptions({ renderSideBySide: newValue })
})

onMounted(() => {
  createDiffEditor()
})

onUnmounted(() => {
  disposeDiffEditor()
})
</script>

<style lang="scss" scoped>
.monaco-diff-editor-container {
  border: 1px solid var(--border-color, #30363d);
  border-radius: 6px;
  overflow: hidden;
  width: 100%;

  :deep(.monaco-diff-editor) {
    .margin,
    .lines-content {
      font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', 'Consolas', 'source-code-pro', monospace;
    }
  }
}
</style>
