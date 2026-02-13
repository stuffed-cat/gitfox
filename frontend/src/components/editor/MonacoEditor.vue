<template>
  <div ref="containerRef" class="monaco-editor-container" :style="containerStyle"></div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted, shallowRef } from 'vue'
import * as monaco from 'monaco-editor'

// Monaco Editor Workers - 本地加载，不依赖 CDN
import editorWorker from 'monaco-editor/esm/vs/editor/editor.worker?worker'
import jsonWorker from 'monaco-editor/esm/vs/language/json/json.worker?worker'
import cssWorker from 'monaco-editor/esm/vs/language/css/css.worker?worker'
import htmlWorker from 'monaco-editor/esm/vs/language/html/html.worker?worker'
import tsWorker from 'monaco-editor/esm/vs/language/typescript/ts.worker?worker'

// 配置 Monaco 环境，根据语言选择正确的 worker
self.MonacoEnvironment = {
  getWorker(_workerId: string, label: string) {
    if (label === 'json') {
      return new jsonWorker()
    }
    if (label === 'css' || label === 'scss' || label === 'less') {
      return new cssWorker()
    }
    if (label === 'html' || label === 'handlebars' || label === 'razor') {
      return new htmlWorker()
    }
    if (label === 'typescript' || label === 'javascript') {
      return new tsWorker()
    }
    return new editorWorker()
  }
}

interface Props {
  modelValue?: string
  language?: string
  theme?: 'vs' | 'vs-dark' | 'hc-black'
  readonly?: boolean
  height?: string | number
  minimap?: boolean
  lineNumbers?: 'on' | 'off' | 'relative' | 'interval'
  wordWrap?: 'off' | 'on' | 'wordWrapColumn' | 'bounded'
  fontSize?: number
  tabSize?: number
  options?: monaco.editor.IStandaloneEditorConstructionOptions
}

const props = withDefaults(defineProps<Props>(), {
  modelValue: '',
  language: 'plaintext',
  theme: 'vs-dark',
  readonly: false,
  height: '400px',
  minimap: true,
  lineNumbers: 'on',
  wordWrap: 'off',
  fontSize: 14,
  tabSize: 2,
  options: () => ({})
})

const emit = defineEmits<{
  (e: 'update:modelValue', value: string): void
  (e: 'change', value: string): void
  (e: 'save', value: string): void
  (e: 'editor-mounted', editor: monaco.editor.IStandaloneCodeEditor): void
}>()

const containerRef = ref<HTMLDivElement>()
const editor = shallowRef<monaco.editor.IStandaloneCodeEditor>()

const containerStyle = computed(() => ({
  height: typeof props.height === 'number' ? `${props.height}px` : props.height,
  width: '100%'
}))

// 根据文件扩展名获取语言
const languageMap: Record<string, string> = {
  // JavaScript/TypeScript
  js: 'javascript', mjs: 'javascript', cjs: 'javascript',
  ts: 'typescript', mts: 'typescript', cts: 'typescript',
  jsx: 'javascript', tsx: 'typescript',
  // Web
  html: 'html', htm: 'html', vue: 'html', svelte: 'html',
  css: 'css', scss: 'scss', sass: 'scss', less: 'less',
  // Data
  json: 'json', jsonc: 'json',
  yaml: 'yaml', yml: 'yaml',
  xml: 'xml', svg: 'xml', xsl: 'xml',
  toml: 'ini',
  // Programming
  py: 'python', pyw: 'python', pyi: 'python',
  rb: 'ruby', rake: 'ruby',
  php: 'php',
  java: 'java',
  kt: 'kotlin', kts: 'kotlin',
  scala: 'scala',
  go: 'go',
  rs: 'rust',
  c: 'c', h: 'c',
  cpp: 'cpp', cc: 'cpp', cxx: 'cpp', hpp: 'cpp', hxx: 'cpp',
  cs: 'csharp',
  swift: 'swift',
  m: 'objective-c', mm: 'objective-c',
  r: 'r', R: 'r',
  lua: 'lua',
  pl: 'perl', pm: 'perl',
  // Shell
  sh: 'shell', bash: 'shell', zsh: 'shell', fish: 'shell',
  ps1: 'powershell', psm1: 'powershell',
  bat: 'bat', cmd: 'bat',
  // Database
  sql: 'sql', mysql: 'sql', pgsql: 'sql',
  // Config
  dockerfile: 'dockerfile',
  makefile: 'makefile',
  cmake: 'cmake',
  // Markup
  md: 'markdown', markdown: 'markdown', mdx: 'markdown',
  tex: 'latex', latex: 'latex',
  // Other
  graphql: 'graphql', gql: 'graphql',
  proto: 'protobuf',
  ini: 'ini', conf: 'ini', cfg: 'ini',
  diff: 'diff', patch: 'diff',
}

function getLanguageFromFilename(filename: string): string {
  const ext = filename.split('.').pop()?.toLowerCase() || ''
  const name = filename.toLowerCase()
  
  // 特殊文件名
  if (name === 'dockerfile' || name.startsWith('dockerfile.')) return 'dockerfile'
  if (name === 'makefile' || name === 'gnumakefile') return 'makefile'
  if (name === 'cmakelists.txt') return 'cmake'
  if (name === '.gitignore' || name === '.dockerignore') return 'ignore'
  if (name === '.env' || name.startsWith('.env.')) return 'dotenv'
  if (name === 'cargo.toml' || name === 'cargo.lock') return 'toml'
  if (name === 'package.json' || name === 'tsconfig.json') return 'json'
  
  return languageMap[ext] || 'plaintext'
}

function createEditor() {
  if (!containerRef.value) return

  const editorOptions: monaco.editor.IStandaloneEditorConstructionOptions = {
    value: props.modelValue,
    language: props.language,
    theme: props.theme,
    readOnly: props.readonly,
    minimap: { enabled: props.minimap },
    lineNumbers: props.lineNumbers,
    wordWrap: props.wordWrap,
    fontSize: props.fontSize,
    tabSize: props.tabSize,
    automaticLayout: true,
    scrollBeyondLastLine: false,
    renderLineHighlight: 'all',
    cursorBlinking: 'smooth',
    cursorSmoothCaretAnimation: 'on',
    smoothScrolling: true,
    bracketPairColorization: { enabled: true },
    guides: {
      bracketPairs: true,
      indentation: true,
    },
    padding: { top: 8, bottom: 8 },
    ...props.options
  }

  editor.value = monaco.editor.create(containerRef.value, editorOptions)

  // 监听内容变化
  editor.value.onDidChangeModelContent(() => {
    const value = editor.value?.getValue() || ''
    emit('update:modelValue', value)
    emit('change', value)
  })

  // 添加保存快捷键
  editor.value.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS, () => {
    emit('save', editor.value?.getValue() || '')
  })

  emit('editor-mounted', editor.value)
}

function disposeEditor() {
  editor.value?.dispose()
  editor.value = undefined
}

// 暴露编辑器实例和方法
defineExpose({
  getEditor: () => editor.value,
  getValue: () => editor.value?.getValue() || '',
  setValue: (value: string) => editor.value?.setValue(value),
  getLanguageFromFilename,
  setLanguage: (language: string) => {
    const model = editor.value?.getModel()
    if (model) {
      monaco.editor.setModelLanguage(model, language)
    }
  },
  focus: () => editor.value?.focus(),
  layout: () => editor.value?.layout(),
})

// 监听属性变化
watch(() => props.modelValue, (newValue) => {
  if (editor.value && newValue !== editor.value.getValue()) {
    editor.value.setValue(newValue)
  }
})

watch(() => props.language, (newLanguage) => {
  const model = editor.value?.getModel()
  if (model) {
    monaco.editor.setModelLanguage(model, newLanguage)
  }
})

watch(() => props.theme, (newTheme) => {
  monaco.editor.setTheme(newTheme)
})

watch(() => props.readonly, (newReadonly) => {
  editor.value?.updateOptions({ readOnly: newReadonly })
})

watch(() => props.fontSize, (newSize) => {
  editor.value?.updateOptions({ fontSize: newSize })
})

onMounted(() => {
  createEditor()
})

onUnmounted(() => {
  disposeEditor()
})
</script>

<style lang="scss" scoped>
.monaco-editor-container {
  border: 1px solid var(--border-color, #30363d);
  border-radius: 6px;
  overflow: hidden;
}
</style>
