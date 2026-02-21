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
      <div v-else-if="jobStatus === 'pending'" class="log-info">
        <svg viewBox="0 0 24 24" width="48" height="48" style="color: #f59e0b; margin-bottom: 12px;">
          <circle cx="12" cy="12" r="10" fill="none" stroke="currentColor" stroke-width="2"/>
          <path d="M12 6v6l4 2" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
        </svg>
        <div style="font-size: 16px; font-weight: 500; margin-bottom: 8px;">作业正在排队</div>
        <div style="color: #858585;">等待可用的 runner...</div>
      </div>
      <div v-else-if="wsError" class="log-error">
        <svg viewBox="0 0 24 24" width="48" height="48" style="color: #f48771; margin-bottom: 12px;">
          <circle cx="12" cy="12" r="10" fill="none" stroke="currentColor" stroke-width="2"/>
          <path d="M12 8v4M12 16v.5" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
        </svg>
        <div style="font-size: 16px; font-weight: 500; margin-bottom: 8px;">无法加载日志</div>
        <div style="color: #858585;">{{ wsError }}</div>
      </div>
      <div v-else-if="!logLines.length" class="log-empty">
        <svg viewBox="0 0 24 24" width="48" height="48" style="color: #858585; margin-bottom: 12px;">
          <path d="M9 12h6M9 16h6M9 8h6" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
          <rect x="5" y="4" width="14" height="16" rx="2" fill="none" stroke="currentColor" stroke-width="2"/>
        </svg>
        <div style="color: #858585;">暂无日志输出</div>
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
import { AnsiUp } from 'ansi_up'
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
const jobStatus = ref<string>('')
const wsError = ref<string>('')

// 创建 ansi_up 实例
const ansiUp = new AnsiUp()
ansiUp.use_classes = false // 使用内联样式而不是 CSS 类

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
    // 先转换 ANSI，再 escape HTML（否则会破坏 ANSI 转义序列）
    let html = ansiToHtml(line)
    
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

function escapeRegex(text: string): string {
  return text.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
}

function ansiToHtml(text: string): string {
  // 检查是否包含 ANSI 转义序列
  const hasAnsi = text.includes('\x1b[')
  
  if (hasAnsi) {
    // 有 ANSI 颜色，直接使用 ansi_up 转换
    return ansiUp.ansi_to_html(text)
  } else {
    // 没有 ANSI 颜色，先转义 HTML，再应用语法高亮
    const escaped = text
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;')
      .replace(/'/g, '&#039;')
    
    return applySyntaxHighlight(escaped)
  }
}

function applySyntaxHighlight(text: string): string {
  let result = text
  
  // Shell 命令行（$ 开头的行）
  result = result.replace(/^(\$\s+)(.*)$/gm, '<span style="color:#58a6ff;font-weight:bold">$1</span><span style="color:#c9d1d9">$2</span>')
  
  // === 分隔符（整行）
  result = result.replace(/^(===.*===)$/gm, '<span style="color:#58a6ff;font-weight:bold">$1</span>')
  
  // 行首的日志级别前缀（info:, warn:, debug:, error: 等）
  result = result.replace(/^(error|fatal):\s*/gmi, '<span style="color:#ff7b72;font-weight:bold">$1:</span> ')
  result = result.replace(/^(warn|warning):\s*/gmi, '<span style="color:#f0883e;font-weight:bold">$1:</span> ')
  result = result.replace(/^(info):\s*/gmi, '<span style="color:#79c0ff;font-weight:bold">$1:</span> ')
  result = result.replace(/^(debug|trace):\s*/gmi, '<span style="color:#8b949e">$1:</span> ')
  
  // Script failed / Script: xxx
  result = result.replace(/\b(Script failed)\b/gi, '<span style="color:#ff7b72;font-weight:bold">$1</span>')
  
  // Exit code
  result = result.replace(/\b(exit code):?\s*(\d+)/gi, '<span style="color:#ff7b72">$1</span>: <span style="color:#ff7b72;font-weight:bold">$2</span>')
  
  // URL（在没有被标签包围的情况下）
  result = result.replace(/\b(https?:\/\/[^\s<>&]+)/g, '<span style="color:#58a6ff;text-decoration:underline">$1</span>')
  
  return result
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

async function loadJobStatus() {
  try {
    // 先获取作业详情以检查状态
    const result = await api.pipelines.get(
      { namespace: props.namespace, project: props.project },
      String(props.pipelineId)
    )
    const job = result.jobs.find((j: any) => j.id === String(props.jobId))
    if (job) {
      jobStatus.value = job.status
      console.log(`Job ${props.jobId} status: ${job.status}`)
    } else {
      console.warn(`Job ${props.jobId} not found in pipeline response`)
      // 如果找不到 job，假设可能已完成
      jobStatus.value = 'unknown'
    }
  } catch (error) {
    console.error('Failed to load job status:', error)
    // 获取状态失败，假设可能已完成
    jobStatus.value = 'unknown'
  }
}

async function loadLog() {
  loading.value = true
  wsError.value = ''
  
  try {
    console.log(`Loading log for job ${props.jobId}...`)
    const result = await api.pipelines.getJobLog(
      { namespace: props.namespace, project: props.project },
      String(props.pipelineId),
      String(props.jobId)
    )
    console.log('Log loaded:', result)
    
    // Handle both string and object responses
    const logText = typeof result === 'string' ? result : (result.log || '')
    logLines.value = logText ? logText.split('\n') : []
    
    console.log(`Loaded ${logLines.value.length} log lines`)
    handleSearch()
    if (autoScroll.value) scrollToBottom()
  } catch (error: any) {
    console.error('Failed to load log:', error)
    // 不在这里设置错误，让状态提示来显示
    if (error.response?.status !== 404) {
      wsError.value = '加载日志失败: ' + (error.message || '未知错误')
    }
  } finally {
    loading.value = false
  }
}

function connectWebSocket() {
  const protocol = location.protocol === 'https:' ? 'wss:' : 'ws:'
  const wsUrl = `${protocol}//${location.host}/api/v1/projects/${props.namespace}/${props.project}/pipelines/${props.pipelineId}/jobs/${props.jobId}/log/stream`
  
  console.log('Connecting WebSocket:', wsUrl)
  connectionStatus.value = 'connecting'
  ws.value = new WebSocket(wsUrl)
  
  // 添加连接超时检测（10秒）
  const connectTimeout = setTimeout(() => {
    if (ws.value && ws.value.readyState === WebSocket.CONNECTING) {
      console.error('WebSocket connection timeout')
      ws.value.close()
      connectionStatus.value = 'disconnected'
      wsError.value = 'WebSocket 连接超时'
    }
  }, 10000)
  
  ws.value.onopen = () => {
    clearTimeout(connectTimeout)
    connectionStatus.value = 'connected'
    console.log('WebSocket connected')
  }
  
  ws.value.onmessage = (event) => {
    console.log('WebSocket message:', event.data.substring(0, 100))
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
    clearTimeout(connectTimeout)
    console.error('WebSocket error:', error)
    connectionStatus.value = 'disconnected'
  }
  
  ws.value.onclose = (event) => {
    clearTimeout(connectTimeout)
    connectionStatus.value = 'disconnected'
    console.log('WebSocket disconnected:', event.code, event.reason)
    
    // 如果是异常关闭（不是正常关闭码 1000），检查是否需要重连
    if (event.code !== 1000) {
      // 如果作业还在运行，尝试重连
      if (jobStatus.value === 'running') {
        setTimeout(() => {
          if (!ws.value || ws.value.readyState === WebSocket.CLOSED) {
            connectWebSocket()
          }
        }, 5000)
      } else if (jobStatus.value === 'pending') {
        // 作业在等待，定期检查状态
        setTimeout(async () => {
          await loadJobStatus()
          if (jobStatus.value === 'running') {
            connectWebSocket()
          } else if (jobStatus.value === 'pending') {
            // 继续等待
            setTimeout(() => ws.value?.close(), 100)
          }
        }, 3000)
      }
    }
  }
}

onMounted(async () => {
  await loadJobStatus()
  await loadLog()
  
  // 连接 WebSocket（除非明确是 pending 状态）
  if (jobStatus.value !== 'pending') {
    // running, success, failed, unknown 都尝试连接
    connectWebSocket()
  } else if (jobStatus.value === 'pending') {
    // 定期检查状态
    const checkInterval = setInterval(async () => {
      await loadJobStatus()
      if (jobStatus.value !== 'pending') {
        clearInterval(checkInterval)
        if (jobStatus.value === 'running') {
          await loadLog()
          connectWebSocket()
        }
      }
    }, 3000)
    
    // 清理
    const cleanup = () => clearInterval(checkInterval)
    if (typeof window !== 'undefined') {
      window.addEventListener('beforeunload', cleanup)
    }
  }
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
.log-empty,
.log-info,
.log-error {
  padding: 40px 20px;
  text-align: center;
  color: #858585;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
}

.log-info {
  color: #dcdcaa;
}

.log-error {
  color: #f48771;
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
