<template>
  <div class="job-log-viewer">
    <div class="log-toolbar">
      <div class="toolbar-left">
        <button class="tool-btn" @click="scrollToBottom" title="滚动到底部">
          <svg viewBox="0 0 16 16" width="14" height="14">
            <path d="M8 11L3 6h10l-5 5z" fill="currentColor"/>
          </svg>
        </button>
        <button class="tool-btn" @click="toggleAutoScroll" :class="{ active: autoScroll }" title="自动滚动">
          <svg viewBox="0 0 16 16" width="14" height="14">
            <path d="M8 2v12M3 9l5 5 5-5" fill="none" stroke="currentColor" stroke-width="1.5"/>
          </svg>
        </button>
        <button class="tool-btn" @click="toggleWrap" title="自动换行">
          <svg viewBox="0 0 16 16" width="14" height="14">
            <path d="M2 4h12M2 8h8a2 2 0 012 2v0a2 2 0 01-2 2H8" fill="none" stroke="currentColor" stroke-width="1.5"/>
            <path d="M10 11l-2 2 2 2" fill="none" stroke="currentColor" stroke-width="1.5"/>
          </svg>
        </button>
        <div class="divider"></div>
        <input 
          type="text" 
          class="search-input" 
          v-model="searchQuery"
          placeholder="搜索日志..."
          @input="handleSearch"
        />
        <span v-if="searchQuery" class="search-count">{{ searchMatches }} 个匹配</span>
      </div>
      <div class="toolbar-right">
        <span class="connection-status" :class="connectionStatus">
          <span class="status-dot"></span>
          {{ connectionStatusText }}
        </span>
        <button class="tool-btn" @click="downloadLog" title="下载日志">
          <svg viewBox="0 0 16 16" width="14" height="14">
            <path d="M8 2v8M4 7l4 4 4-4M2 14h12" fill="none" stroke="currentColor" stroke-width="1.5"/>
          </svg>
          下载
        </button>
        <button class="tool-btn" @click="copyLog" title="复制全部">
          <svg viewBox="0 0 16 16" width="14" height="14">
            <rect x="5" y="5" width="9" height="9" rx="1" fill="none" stroke="currentColor" stroke-width="1.5"/>
            <path d="M3 11V3a1 1 0 011-1h8" fill="none" stroke="currentColor" stroke-width="1.5"/>
          </svg>
          复制
        </button>
      </div>
    </div>

    <div class="log-content" ref="logContainer" :class="{ wrap: wrapLines }">
      <div v-if="loading && !logLines.length" class="log-loading">
        正在加载日志...
      </div>
      <div v-else-if="!logLines.length" class="log-empty">
        暂无日志输出
      </div>
      <template v-else>
        <div 
          v-for="(line, index) in displayLines" 
          :key="index"
          class="log-line"
          :class="{ highlight: line.match }"
        >
          <span class="line-number">{{ index + 1 }}</span>
          <span class="line-content" v-html="line.html"></span>
        </div>
      </template>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onBeforeUnmount, nextTick } from 'vue'
import api from '@/api'

interface Props {
  namespace: string
  project: string
  pipelineId: number | string
  jobId: number | string
  jobName?: string
}

const props = defineProps<Props>()

const logContainer = ref<HTMLElement | null>(null)
const logLines = ref<string[]>([])
const loading = ref(false)
const searchQuery = ref('')
const searchMatches = ref(0)
const autoScroll = ref(true)
const wrapLines = ref(false)
const ws = ref<WebSocket | null>(null)
const connectionStatus = ref<'connected' | 'connecting' | 'disconnected'>('connecting')

const connectionStatusText = computed(() => {
  const map = {
    connected: '实时更新',
    connecting: '连接中...',
    disconnected: '已断开'
  }
  return map[connectionStatus.value]
})

interface DisplayLine {
  html: string
  match: boolean
}

const displayLines = computed((): DisplayLine[] => {
  return logLines.value.map(line => {
    let html = escapeHtml(line)
    html = ansiToHtml(html)
    
    let match = false
    if (searchQuery.value) {
      const regex = new RegExp(escapeRegex(searchQuery.value), 'gi')
      const hasMatch = regex.test(html)
      if (hasMatch) {
        html = html.replace(regex, '<mark>$&</mark>')
        match = true
      }
    }
    
    return { html, match }
  })
})

function escapeHtml(text: string): string {
  const div = document.createElement('div')
  div.textContent = text
  return div.innerHTML
}

function escapeRegex(text: string): string {
  return text.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
}

function ansiToHtml(text: string): string {
  // Simple ANSI color code to HTML conversion
  const ansiMap: Record<string, string> = {
    '30': 'color:#000', '31': 'color:#d73a49', '32': 'color:#22863a',
    '33': 'color:#b08800', '34': 'color:#005cc5', '35': 'color:#6f42c1',
    '36': 'color:#0598bc', '37': 'color:#586069',
    '90': 'color:#6a737d', '91': 'color:#f97583', '92': 'color:#85e89d',
    '93': 'color:#ffea7f', '94': 'color:#79b8ff', '95': 'color:#b392f0',
    '96': 'color:#56d4dd', '97': 'color:#e1e4e8'
  }
  
  return text.replace(/\x1b\[(\d+)(;\d+)*m/g, (match, code) => {
    if (code === '0' || code === '00') return '</span>'
    const style = ansiMap[code]
    return style ? `<span style="${style}">` : ''
  })
}

function handleSearch() {
  if (!searchQuery.value) {
    searchMatches.value = 0
    return
  }
  
  const regex = new RegExp(escapeRegex(searchQuery.value), 'gi')
  let count = 0
  logLines.value.forEach(line => {
    const matches = line.match(regex)
    if (matches) count += matches.length
  })
  searchMatches.value = count
}

function scrollToBottom() {
  nextTick(() => {
    if (logContainer.value) {
      logContainer.value.scrollTop = logContainer.value.scrollHeight
    }
  })
}

function toggleAutoScroll() {
  autoScroll.value = !autoScroll.value
  if (autoScroll.value) scrollToBottom()
}

function toggleWrap() {
  wrapLines.value = !wrapLines.value
}

async function downloadLog() {
  const url = `/api/v1/projects/${props.namespace}/${props.project}/pipelines/${props.pipelineId}/jobs/${props.jobId}/log/download`
  window.open(url, '_blank')
}

async function copyLog() {
  const text = logLines.value.join('\n')
  try {
    await navigator.clipboard.writeText(text)
    // Use a simple visual feedback instead of alert
    const btn = document.activeElement as HTMLButtonElement
    if (btn) {
      const originalText = btn.innerHTML
      btn.innerHTML = '<svg viewBox="0 0 16 16" width="14" height="14"><path d="M13 4L6 11 3 8" fill="none" stroke="currentColor" stroke-width="2"/></svg> 已复制'
      btn.style.color = '#4ec9b0'
      setTimeout(() => {
        btn.innerHTML = originalText
        btn.style.color = ''
      }, 2000)
    }
  } catch (err) {
    console.error('Failed to copy:', err)
  }
}

async function loadLog() {
  loading.value = true
  try {
    const result = await api.pipelines.getJobLog(
      { namespace: props.namespace, project: props.project },
      String(props.pipelineId),
      String(props.jobId)
    )
    logLines.value = result.log.split('\n')
    handleSearch()
    if (autoScroll.value) scrollToBottom()
  } catch (error) {
    console.error('Failed to load log:', error)
    logLines.value = ['加载日志失败']
  } finally {
    loading.value = false
  }
}

function connectWebSocket() {
  const protocol = location.protocol === 'https:' ? 'wss:' : 'ws:'
  const wsUrl = `${protocol}//${location.host}/api/v1/projects/${props.namespace}/${props.project}/pipelines/${props.pipelineId}/jobs/${props.jobId}/log/stream`
  
  connectionStatus.value = 'connecting'
  ws.value = new WebSocket(wsUrl)
  
  ws.value.onopen = () => {
    connectionStatus.value = 'connected'
    console.log('WebSocket connected')
  }
  
  ws.value.onmessage = (event) => {
    const newLines = event.data.split('\n').filter((l: string) => l)
    if (newLines.length > 0) {
      logLines.value.push(...newLines)
      handleSearch()
      if (autoScroll.value) {
        nextTick(() => scrollToBottom())
      }
    }
  }
  
  ws.value.onerror = (error) => {
    console.error('WebSocket error:', error)
    connectionStatus.value = 'disconnected'
  }
  
  ws.value.onclose = () => {
    connectionStatus.value = 'disconnected'
    console.log('WebSocket disconnected')
    // Try to reconnect after 5 seconds
    setTimeout(() => {
      if (!ws.value || ws.value.readyState === WebSocket.CLOSED) {
        connectWebSocket()
      }
    }, 5000)
  }
}

onMounted(() => {
  loadLog().then(() => {
    // Connect WebSocket for live updates
    connectWebSocket()
  })
})

onBeforeUnmount(() => {
  if (ws.value) {
    ws.value.close()
    ws.value = null
  }
})

watch(() => props.jobId, () => {
  if (ws.value) ws.value.close()
  logLines.value = []
  loadLog().then(() => connectWebSocket())
})
</script>

<style lang="scss" scoped>
.job-log-viewer {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: #1e1e1e;
  color: #d4d4d4;
  font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', monospace;
  font-size: 13px;
}

.log-toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 12px;
  background: #252526;
  border-bottom: 1px solid #3e3e42;
  gap: 8px;
}

.toolbar-left,
.toolbar-right {
  display: flex;
  align-items: center;
  gap: 4px;
}

.tool-btn {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 4px 8px;
  background: transparent;
  border: 1px solid transparent;
  border-radius: 3px;
  color: #cccccc;
  cursor: pointer;
  font-size: 12px;
  transition: all 0.2s;

  &:hover {
    background: #2a2d2e;
    border-color: #454545;
  }

  &.active {
    background: #094771;
    border-color: #007acc;
    color: #ffffff;
  }

  svg {
    flex-shrink: 0;
  }
}

.divider {
  width: 1px;
  height: 20px;
  background: #3e3e42;
  margin: 0 4px;
}

.search-input {
  padding: 4px 8px;
  background: #3c3c3c;
  border: 1px solid #3e3e42;
  border-radius: 3px;
  color: #cccccc;
  font-size: 12px;
  min-width: 200px;

  &:focus {
    outline: none;
    border-color: #007acc;
    background: #1e1e1e;
  }

  &::placeholder {
    color: #858585;
  }
}

.search-count {
  font-size: 11px;
  color: #858585;
  white-space: nowrap;
}

.connection-status {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 8px;
  font-size: 11px;
  border-radius: 3px;
  
  &.connected {
    color: #4ec9b0;
    .status-dot {
      background: #4ec9b0;
      box-shadow: 0 0 4px #4ec9b0;
    }
  }
  
  &.connecting {
    color: #dcdcaa;
    .status-dot {
      background: #dcdcaa;
      animation: pulse 2s ease-in-out infinite;
    }
  }
  
  &.disconnected {
    color: #f48771;
    .status-dot {
      background: #f48771;
    }
  }
}

.status-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.5; }
}

.log-content {
  flex: 1;
  overflow: auto;
  padding: 12px 0;
  
  &.wrap .line-content {
    white-space: pre-wrap;
    word-break: break-all;
  }
}

.log-loading,
.log-empty {
  padding: 20px;
  text-align: center;
  color: #858585;
}

.log-line {
  display: flex;
  padding: 0 12px;
  transition: background 0.1s;
  
  &:hover {
    background: #2a2d2e;
  }
  
  &.highlight {
    background: #613214;
  }
}

.line-number {
  flex-shrink: 0;
  width: 50px;
  padding-right: 12px;
  color: #858585;
  text-align: right;
  user-select: none;
  border-right: 1px solid #3e3e42;
  margin-right: 12px;
}

.line-content {
  flex: 1;
  white-space: pre;
  
  :deep(mark) {
    background: #f9826c;
    color: #000;
    padding: 1px 2px;
    border-radius: 2px;
  }
}
</style>
