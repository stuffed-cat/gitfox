<template>
  <div class="code-viewer">
    <MonacoEditor
      ref="editorRef"
      :model-value="content"
      :language="language"
      :theme="theme"
      :height="height"
      :readonly="true"
      :minimap="minimap"
      :line-numbers="lineNumbers"
      :font-size="fontSize"
      :word-wrap="wordWrap"
      :options="editorOptions"
    />
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import MonacoEditor from './MonacoEditor.vue'

interface Props {
  content: string
  filename?: string
  language?: string
  theme?: 'vs' | 'vs-dark' | 'hc-black'
  height?: string | number
  minimap?: boolean
  lineNumbers?: 'on' | 'off' | 'relative' | 'interval'
  fontSize?: number
  wordWrap?: 'off' | 'on' | 'wordWrapColumn' | 'bounded'
}

const props = withDefaults(defineProps<Props>(), {
  filename: '',
  language: '',
  theme: 'vs-dark',
  height: '500px',
  minimap: false,
  lineNumbers: 'on',
  fontSize: 13,
  wordWrap: 'off'
})

const editorRef = ref<InstanceType<typeof MonacoEditor>>()

// 自动检测语言
const language = computed(() => {
  if (props.language) return props.language
  if (props.filename && editorRef.value) {
    return editorRef.value.getLanguageFromFilename(props.filename)
  }
  return 'plaintext'
})

// 只读优化选项
const editorOptions = computed(() => ({
  readOnly: true,
  domReadOnly: true,
  cursorStyle: 'line' as const,
  renderLineHighlight: 'none' as const,
  occurrencesHighlight: 'off' as const,
  selectionHighlight: false,
  links: true,
  folding: true,
  foldingStrategy: 'indentation' as const,
  showFoldingControls: 'mouseover' as const,
  scrollbar: {
    verticalScrollbarSize: 10,
    horizontalScrollbarSize: 10,
  },
}))

defineExpose({
  getEditor: () => editorRef.value?.getEditor(),
})
</script>

<style lang="scss" scoped>
.code-viewer {
  width: 100%;
}
</style>
