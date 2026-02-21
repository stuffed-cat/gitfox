<template>
  <div ref="editorContainer" class="monaco-editor-wrapper"></div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from 'vue'
import * as monaco from 'monaco-editor'

const props = defineProps<{
  value: string
  language: string
  path: string
  theme?: string
  readOnly?: boolean
}>()

const emit = defineEmits<{
  (e: 'change', value: string): void
  (e: 'save'): void
  (e: 'cursor', position: { line: number; column: number }): void
}>()

const editorContainer = ref<HTMLElement | null>(null)
let editor: monaco.editor.IStandaloneCodeEditor | null = null

onMounted(() => {
  if (!editorContainer.value) return
  
  editor = monaco.editor.create(editorContainer.value, {
    value: props.value,
    language: props.language,
    theme: props.theme || 'gitfox-dark',
    readOnly: props.readOnly,
    automaticLayout: true,
    minimap: {
      enabled: true,
      scale: 1,
      showSlider: 'mouseover'
    },
    scrollBeyondLastLine: false,
    fontSize: 14,
    fontFamily: "'JetBrains Mono', 'Fira Code', 'SF Mono', Monaco, Consolas, monospace",
    fontLigatures: true,
    lineNumbers: 'on',
    renderLineHighlight: 'all',
    cursorBlinking: 'smooth',
    cursorSmoothCaretAnimation: 'on',
    smoothScrolling: true,
    padding: { top: 8, bottom: 8 },
    bracketPairColorization: { enabled: true },
    guides: {
      bracketPairs: true,
      indentation: true
    },
    suggest: {
      showKeywords: true,
      showSnippets: true
    },
    quickSuggestions: true,
    wordWrap: 'off',
    folding: true,
    foldingStrategy: 'indentation'
  })
  
  // Handle content changes
  editor.onDidChangeModelContent(() => {
    if (editor) {
      emit('change', editor.getValue())
    }
  })
  
  // Handle cursor position changes
  editor.onDidChangeCursorPosition((e) => {
    emit('cursor', {
      line: e.position.lineNumber,
      column: e.position.column
    })
  })
  
  // Handle save command (Ctrl+S)
  editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS, () => {
    emit('save')
  })
})

onUnmounted(() => {
  if (editor) {
    editor.dispose()
    editor = null
  }
})

// Watch for value changes from outside
watch(() => props.value, (newValue) => {
  if (editor && editor.getValue() !== newValue) {
    const position = editor.getPosition()
    editor.setValue(newValue)
    if (position) {
      editor.setPosition(position)
    }
  }
})

// Watch for language changes
watch(() => props.language, (newLanguage) => {
  if (editor) {
    const model = editor.getModel()
    if (model) {
      monaco.editor.setModelLanguage(model, newLanguage)
    }
  }
})

// Watch for theme changes
watch(() => props.theme, (newTheme) => {
  if (newTheme) {
    monaco.editor.setTheme(newTheme)
  }
})
</script>

<style lang="scss" scoped>
.monaco-editor-wrapper {
  width: 100%;
  height: 100%;
}
</style>
