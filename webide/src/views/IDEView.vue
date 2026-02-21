<template>
  <div class="ide-layout">
    <!-- Activity Bar (VSCode-style leftmost icon bar) -->
    <aside class="activity-bar">
      <div class="activity-icons">
        <button 
          v-for="item in activityItems" 
          :key="item.id"
          class="activity-icon"
          :class="{ active: activeActivity === item.id }"
          :title="item.title"
          @click="setActivity(item.id)"
        >
          <component :is="item.icon" />
          <span v-if="item.badge" class="badge">{{ item.badge }}</span>
        </button>
      </div>
      <div class="activity-bottom">
        <button class="activity-icon" title="设置" @click="showSettings = true">
          <SettingsIcon />
        </button>
      </div>
    </aside>

    <!-- Sidebar -->
    <aside 
      class="sidebar" 
      :class="{ collapsed: !sidebarVisible }"
      :style="{ width: sidebarVisible ? `${sidebarWidth}px` : '0' }"
    >
      <div class="sidebar-header">
        <span class="sidebar-title">{{ sidebarTitle }}</span>
        <button class="btn-icon" @click="sidebarVisible = false">
          <ChevronLeftIcon />
        </button>
      </div>
      
      <!-- Explorer Panel -->
      <div v-if="activeActivity === 'explorer'" class="sidebar-content">
        <div class="explorer-header">
          <span class="repo-name">{{ owner }}/{{ repo }}</span>
          <div class="explorer-actions">
            <button class="btn-icon btn-sm" title="新建文件" @click="createNewFile">
              <FilePlusIcon />
            </button>
            <button class="btn-icon btn-sm" title="新建文件夹" @click="createNewFolder">
              <FolderPlusIcon />
            </button>
            <button class="btn-icon btn-sm" title="刷新" @click="refreshTree">
              <RefreshIcon />
            </button>
          </div>
        </div>
        <div class="file-tree" ref="fileTreeRef">
          <FileTreeNode
            v-for="node in editorStore.fileTree"
            :key="node.path"
            :node="node"
            :depth="0"
            :expanded-folders="editorStore.expandedFolders"
            :active-path="editorStore.activeFilePath"
            @select="handleFileSelect"
            @toggle="editorStore.toggleFolder"
            @context-menu="handleContextMenu"
          />
        </div>
      </div>

      <!-- Search Panel -->
      <div v-else-if="activeActivity === 'search'" class="sidebar-content">
        <div class="search-input-wrapper">
          <input
            type="text"
            v-model="searchQuery"
            placeholder="搜索文件..."
            class="input input-sm"
            @input="handleSearch"
          />
        </div>
        <div class="search-results">
          <div v-if="searchResults.length === 0 && searchQuery" class="search-empty">
            没有找到匹配的文件
          </div>
          <div 
            v-for="result in searchResults" 
            :key="result.path"
            class="search-result-item"
            @click="handleFileSelect(result.path)"
          >
            <FileIcon :name="result.path" />
            <span class="result-name">{{ result.name }}</span>
            <span class="result-path">{{ result.dir }}</span>
          </div>
        </div>
      </div>

      <!-- Extensions Panel -->
      <div v-else-if="activeActivity === 'extensions'" class="sidebar-content">
        <div class="extensions-header">
          <input
            type="text"
            v-model="extensionSearch"
            placeholder="搜索扩展..."
            class="input input-sm"
          />
        </div>
        <div class="extensions-list">
          <div class="extension-section">
            <h4>已安装</h4>
            <div v-if="installedExtensions.length === 0" class="empty-state">
              暂无已安装的扩展
            </div>
            <ExtensionItem
              v-for="ext in installedExtensions"
              :key="ext.id"
              :extension="ext"
              @toggle="toggleExtension"
              @uninstall="uninstallExtension"
            />
          </div>
          <div class="extension-section">
            <h4>推荐</h4>
            <ExtensionItem
              v-for="ext in recommendedExtensions"
              :key="ext.id"
              :extension="ext"
              @install="installExtension"
            />
          </div>
        </div>
      </div>

      <!-- Git Panel -->
      <div v-else-if="activeActivity === 'git'" class="sidebar-content">
        <div class="git-section">
          <div class="branch-selector">
            <GitBranchIcon />
            <select v-model="currentBranch" class="branch-select">
              <option v-for="branch in branches" :key="branch" :value="branch">
                {{ branch }}
              </option>
            </select>
          </div>
          <div class="changes-section">
            <h4>更改 ({{ changedFiles.length }})</h4>
            <div v-if="changedFiles.length === 0" class="empty-state">
              没有未提交的更改
            </div>
            <div 
              v-for="file in changedFiles" 
              :key="file.path"
              class="changed-file"
            >
              <span class="change-indicator" :class="file.action">
                {{ file.action === 'create' ? 'A' : file.action === 'delete' ? 'D' : 'M' }}
              </span>
              <span class="file-name">{{ file.name }}</span>
            </div>
          </div>
        </div>
      </div>

      <!-- Resize handle -->
      <div 
        class="resize-handle"
        @mousedown="startResize"
      ></div>
    </aside>

    <!-- Main Editor Area -->
    <main class="editor-area">
      <!-- Tab Bar -->
      <div class="tab-bar" v-if="editorStore.openFilesArray.length > 0">
        <div class="tabs-container">
          <div
            v-for="file in editorStore.openFilesArray"
            :key="file.path"
            class="tab"
            :class="{ active: file.path === editorStore.activeFilePath }"
            @click="editorStore.setActiveFile(file.path)"
            @mousedown.middle="closeTab(file.path)"
          >
            <FileIcon :name="file.name" :size="14" />
            <span class="tab-name">{{ file.name }}</span>
            <span v-if="file.modified" class="tab-modified">●</span>
            <button class="tab-close" @click.stop="closeTab(file.path)">
              <CloseIcon />
            </button>
          </div>
        </div>
        <div class="tab-actions">
          <button class="btn-icon btn-sm" title="更多打开的编辑器" @click="showOpenEditors">
            <MoreIcon />
          </button>
        </div>
      </div>

      <!-- Editor -->
      <div class="editor-container" v-if="editorStore.activeFile">
        <MonacoEditor
          :value="editorStore.activeFile.content"
          :language="editorStore.activeFile.language"
          :path="editorStore.activeFile.path"
          :theme="isDarkTheme ? 'gitfox-dark' : 'vs'"
          @change="handleEditorChange"
          @save="saveCurrentFile"
        />
      </div>

      <!-- Welcome Screen -->
      <div class="welcome-screen" v-else>
        <div class="welcome-content">
          <h2>GitFox WebIDE</h2>
          <p>选择左侧文件开始编辑</p>
          <div class="quick-actions">
            <button class="quick-action" @click="showQuickOpen = true">
              <SearchIcon />
              <span>快速打开文件</span>
              <kbd>Ctrl+P</kbd>
            </button>
            <button class="quick-action" @click="showCommandPalette = true">
              <TerminalIcon />
              <span>命令面板</span>
              <kbd>Ctrl+Shift+P</kbd>
            </button>
          </div>
        </div>
      </div>

      <!-- Bottom Panel (Terminal, Output, etc.) -->
      <div 
        class="bottom-panel" 
        v-if="panelVisible"
        :style="{ height: `${panelHeight}px` }"
      >
        <div class="panel-resize-handle" @mousedown="startPanelResize"></div>
        <div class="panel-tabs">
          <button 
            v-for="panel in panels" 
            :key="panel.id"
            class="panel-tab"
            :class="{ active: activePanel === panel.id }"
            @click="activePanel = panel.id"
          >
            {{ panel.title }}
          </button>
          <button class="panel-close" @click="panelVisible = false">
            <CloseIcon />
          </button>
        </div>
        <div class="panel-content">
          <TerminalPanel v-if="activePanel === 'terminal'" />
          <OutputPanel v-else-if="activePanel === 'output'" :logs="outputLogs" />
          <ProblemsPanel v-else-if="activePanel === 'problems'" :problems="problems" />
        </div>
      </div>
    </main>

    <!-- Status Bar -->
    <footer class="status-bar">
      <div class="status-left">
        <span class="status-item" @click="showBranchMenu">
          <GitBranchIcon />
          {{ currentBranch }}
        </span>
        <span v-if="editorStore.hasUnsavedChanges" class="status-item warning">
          <span class="dot"></span>
          {{ editorStore.unsavedFiles.length }} 个未保存
        </span>
      </div>
      <div class="status-right">
        <span class="status-item" v-if="editorStore.activeFile">
          {{ editorStore.activeFile.language }}
        </span>
        <span class="status-item" v-if="cursorPosition">
          行 {{ cursorPosition.line }}, 列 {{ cursorPosition.column }}
        </span>
        <span class="status-item" @click="togglePanel('terminal')">
          <TerminalIcon />
        </span>
        <span class="status-item" @click="toggleTheme">
          <ThemeIcon />
        </span>
      </div>
    </footer>

    <!-- Quick Open Modal -->
    <Teleport to="body">
      <div v-if="showQuickOpen" class="modal-overlay" @click.self="showQuickOpen = false">
        <div class="quick-open-modal">
          <input
            ref="quickOpenInput"
            type="text"
            v-model="quickOpenQuery"
            placeholder="输入文件名..."
            class="quick-open-input"
            @keydown.enter="openQuickOpenResult"
            @keydown.escape="showQuickOpen = false"
            @keydown.up.prevent="navigateQuickOpen(-1)"
            @keydown.down.prevent="navigateQuickOpen(1)"
          />
          <div class="quick-open-results">
            <div
              v-for="(result, index) in quickOpenResults"
              :key="result.path"
              class="quick-open-item"
              :class="{ selected: index === quickOpenIndex }"
              @click="openFile(result.path)"
            >
              <FileIcon :name="result.name" />
              <span class="result-name">{{ result.name }}</span>
              <span class="result-path">{{ result.dir }}</span>
            </div>
          </div>
        </div>
      </div>
    </Teleport>

    <!-- Command Palette Modal -->
    <Teleport to="body">
      <div v-if="showCommandPalette" class="modal-overlay" @click.self="showCommandPalette = false">
        <div class="command-palette-modal">
          <input
            ref="commandPaletteInput"
            type="text"
            v-model="commandQuery"
            placeholder="输入命令..."
            class="command-palette-input"
            @keydown.enter="executeCommand"
            @keydown.escape="showCommandPalette = false"
            @keydown.up.prevent="navigateCommands(-1)"
            @keydown.down.prevent="navigateCommands(1)"
          />
          <div class="command-results">
            <div
              v-for="(cmd, index) in filteredCommands"
              :key="cmd.id"
              class="command-item"
              :class="{ selected: index === commandIndex }"
              @click="runCommand(cmd)"
            >
              <span class="command-label">{{ cmd.label }}</span>
              <span v-if="cmd.keybinding" class="command-keybinding">{{ cmd.keybinding }}</span>
            </div>
          </div>
        </div>
      </div>
    </Teleport>

    <!-- Context Menu -->
    <Teleport to="body">
      <div 
        v-if="contextMenu.visible" 
        class="context-menu"
        :style="{ left: `${contextMenu.x}px`, top: `${contextMenu.y}px` }"
        @click="contextMenu.visible = false"
      >
        <div class="menu-item" @click="openFile(contextMenu.path)">
          <FileIcon :name="contextMenu.path || ''" :size="14" />
          打开
        </div>
        <div class="menu-item" @click="copyPath">
          <CopyIcon />
          复制路径
        </div>
        <div class="menu-separator"></div>
        <div class="menu-item" @click="renameFile">
          <EditIcon />
          重命名
        </div>
        <div class="menu-item danger" @click="deleteFile">
          <TrashIcon />
          删除
        </div>
      </div>
    </Teleport>

    <!-- Settings Modal -->
    <Teleport to="body">
      <div v-if="showSettings" class="modal-overlay" @click.self="showSettings = false">
        <div class="modal settings-modal">
          <div class="modal-header">
            <h3>设置</h3>
            <button class="close-btn" @click="showSettings = false">
              <CloseIcon />
            </button>
          </div>
          <div class="modal-body">
            <SettingsPanel @close="showSettings = false" />
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch, nextTick } from 'vue'
import { useRoute } from 'vue-router'
import { useEditorStore } from '@/stores/editor'
import { useThemeStore } from '@/stores/theme'
import MonacoEditor from '@/components/MonacoEditor.vue'
import FileTreeNode from '@/components/FileTreeNode.vue'
import FileIcon from '@/components/icons/FileIcon.vue'
import ExtensionItem from '@/components/ExtensionItem.vue'
import TerminalPanel from '@/components/panels/TerminalPanel.vue'
import OutputPanel from '@/components/panels/OutputPanel.vue'
import ProblemsPanel from '@/components/panels/ProblemsPanel.vue'
import SettingsPanel from '@/components/SettingsPanel.vue'

// Icons
import ExplorerIcon from '@/components/icons/ExplorerIcon.vue'
import SearchIcon from '@/components/icons/SearchIcon.vue'
import GitBranchIcon from '@/components/icons/GitBranchIcon.vue'
import ExtensionsIcon from '@/components/icons/ExtensionsIcon.vue'
import SettingsIcon from '@/components/icons/SettingsIcon.vue'
import ChevronLeftIcon from '@/components/icons/ChevronLeftIcon.vue'
import FilePlusIcon from '@/components/icons/FilePlusIcon.vue'
import FolderPlusIcon from '@/components/icons/FolderPlusIcon.vue'
import RefreshIcon from '@/components/icons/RefreshIcon.vue'
import CloseIcon from '@/components/icons/CloseIcon.vue'
import MoreIcon from '@/components/icons/MoreIcon.vue'
import TerminalIcon from '@/components/icons/TerminalIcon.vue'
import ThemeIcon from '@/components/icons/ThemeIcon.vue'
import CopyIcon from '@/components/icons/CopyIcon.vue'
import EditIcon from '@/components/icons/EditIcon.vue'
import TrashIcon from '@/components/icons/TrashIcon.vue'

import type { Extension, Command } from '@/types'

// Props
const props = defineProps<{
  owner?: string
  repo?: string
  gitRef?: string
  path?: string
}>()

// Stores
const editorStore = useEditorStore()
const themeStore = useThemeStore()
const route = useRoute()

// Theme
const isDarkTheme = computed(() => themeStore.theme === 'dark')

// Layout state
const sidebarVisible = ref(true)
const sidebarWidth = ref(260)
const panelVisible = ref(false)
const panelHeight = ref(200)
const activeActivity = ref('explorer')
const activePanel = ref('terminal')

// Activity bar items
const activityItems = computed(() => [
  { id: 'explorer', title: '资源管理器', icon: ExplorerIcon },
  { id: 'search', title: '搜索', icon: SearchIcon },
  { id: 'git', title: 'Git', icon: GitBranchIcon, badge: changedFiles.value.length || undefined },
  { id: 'extensions', title: '扩展', icon: ExtensionsIcon }
])

// Sidebar title
const sidebarTitle = computed(() => {
  const titles: Record<string, string> = {
    explorer: '资源管理器',
    search: '搜索',
    git: '源代码管理',
    extensions: '扩展'
  }
  return titles[activeActivity.value] || ''
})

// Git state
const currentBranch = ref('main')
const branches = ref<string[]>(['main'])
const changedFiles = computed(() => {
  return editorStore.unsavedFiles.map(f => ({
    path: f.path,
    name: f.name,
    action: 'update' as 'create' | 'update' | 'delete'
  }))
})

// Search state
const searchQuery = ref('')
const searchResults = ref<Array<{ path: string; name: string; dir: string }>>([])

// Extension state
const extensionSearch = ref('')
const installedExtensions = ref<Extension[]>([])
const recommendedExtensions = ref<Extension[]>([
  {
    id: 'prettier',
    name: 'Prettier',
    version: '10.0.0',
    description: '代码格式化工具',
    publisher: 'prettier',
    enabled: false,
    categories: ['Formatters'],
    activationEvents: ['*']
  },
  {
    id: 'eslint',
    name: 'ESLint',
    version: '3.0.0',
    description: 'JavaScript/TypeScript 代码检查',
    publisher: 'eslint',
    enabled: false,
    categories: ['Linters'],
    activationEvents: ['onLanguage:javascript', 'onLanguage:typescript']
  }
])

// Panel state
const panels = [
  { id: 'terminal', title: '终端' },
  { id: 'output', title: '输出' },
  { id: 'problems', title: '问题' }
]
const outputLogs = ref<string[]>([])
const problems = ref<Array<{ file: string; line: number; message: string; severity: string }>>([])

// Cursor position
const cursorPosition = ref<{ line: number; column: number } | null>(null)

// Quick Open
const showQuickOpen = ref(false)
const quickOpenQuery = ref('')
const quickOpenIndex = ref(0)
const quickOpenInput = ref<HTMLInputElement | null>(null)
const quickOpenResults = computed(() => {
  if (!quickOpenQuery.value) return []
  const query = quickOpenQuery.value.toLowerCase()
  return flattenTree(editorStore.fileTree)
    .filter(f => f.name.toLowerCase().includes(query) || f.path.toLowerCase().includes(query))
    .slice(0, 20)
})

// Command Palette
const showCommandPalette = ref(false)
const commandQuery = ref('')
const commandIndex = ref(0)
const commandPaletteInput = ref<HTMLInputElement | null>(null)
const commands = ref<Command[]>([
  { id: 'save', label: '保存文件', keybinding: 'Ctrl+S', action: saveCurrentFile },
  { id: 'saveAll', label: '保存所有', keybinding: 'Ctrl+Shift+S', action: () => editorStore.saveAllFiles() },
  { id: 'quickOpen', label: '快速打开', keybinding: 'Ctrl+P', action: () => { showQuickOpen.value = true } },
  { id: 'toggleSidebar', label: '切换侧边栏', keybinding: 'Ctrl+B', action: () => { sidebarVisible.value = !sidebarVisible.value } },
  { id: 'toggleTerminal', label: '切换终端', keybinding: 'Ctrl+`', action: () => togglePanel('terminal') },
  { id: 'toggleTheme', label: '切换主题', action: toggleTheme },
  { id: 'refresh', label: '刷新文件树', action: refreshTree }
])
const filteredCommands = computed(() => {
  if (!commandQuery.value) return commands.value
  const query = commandQuery.value.toLowerCase()
  return commands.value.filter(c => c.label.toLowerCase().includes(query))
})

// Context Menu
const contextMenu = ref({
  visible: false,
  x: 0,
  y: 0,
  path: ''
})

// Settings
const showSettings = ref(false)

// Get owner/repo from route or props
const owner = computed(() => (route.params.owner as string) || props.owner || '')
const repo = computed(() => (route.params.repo as string) || props.repo || '')

// Initialize
onMounted(async () => {
  if (owner.value && repo.value) {
    try {
      await editorStore.loadRepository(owner.value, repo.value, props.gitRef)
      currentBranch.value = editorStore.currentRef
      
      // Open initial file if specified
      if (props.path) {
        await editorStore.openFile(props.path)
      }
    } catch (error) {
      console.error('Failed to load repository:', error)
    }
  }
  
  // Register keyboard shortcuts
  document.addEventListener('keydown', handleKeydown)
})

onUnmounted(() => {
  document.removeEventListener('keydown', handleKeydown)
})

// Watch for quick open modal
watch(showQuickOpen, async (visible) => {
  if (visible) {
    quickOpenQuery.value = ''
    quickOpenIndex.value = 0
    await nextTick()
    quickOpenInput.value?.focus()
  }
})

// Watch for command palette modal
watch(showCommandPalette, async (visible) => {
  if (visible) {
    commandQuery.value = ''
    commandIndex.value = 0
    await nextTick()
    commandPaletteInput.value?.focus()
  }
})

// Methods
function setActivity(id: string) {
  if (activeActivity.value === id && sidebarVisible.value) {
    sidebarVisible.value = false
  } else {
    activeActivity.value = id
    sidebarVisible.value = true
  }
}

async function handleFileSelect(path: string) {
  await editorStore.openFile(path)
}

function handleEditorChange(content: string) {
  if (editorStore.activeFilePath) {
    editorStore.updateFileContent(editorStore.activeFilePath, content)
  }
}

async function saveCurrentFile() {
  if (editorStore.activeFilePath && editorStore.activeFile?.modified) {
    await editorStore.saveFile(editorStore.activeFilePath)
  }
}

function closeTab(path: string) {
  const file = editorStore.openFiles.get(path)
  if (file?.modified) {
    if (!confirm(`${file.name} 有未保存的更改，确定关闭吗？`)) {
      return
    }
  }
  editorStore.closeFile(path)
}

function togglePanel(id: string) {
  if (panelVisible.value && activePanel.value === id) {
    panelVisible.value = false
  } else {
    activePanel.value = id
    panelVisible.value = true
  }
}

function toggleTheme() {
  themeStore.toggleTheme()
}

function handleKeydown(e: KeyboardEvent) {
  // Ctrl+S - Save
  if (e.ctrlKey && e.key === 's') {
    e.preventDefault()
    saveCurrentFile()
  }
  // Ctrl+P - Quick Open
  else if (e.ctrlKey && e.key === 'p' && !e.shiftKey) {
    e.preventDefault()
    showQuickOpen.value = true
  }
  // Ctrl+Shift+P - Command Palette
  else if (e.ctrlKey && e.shiftKey && e.key === 'P') {
    e.preventDefault()
    showCommandPalette.value = true
  }
  // Ctrl+B - Toggle Sidebar
  else if (e.ctrlKey && e.key === 'b') {
    e.preventDefault()
    sidebarVisible.value = !sidebarVisible.value
  }
  // Ctrl+` - Toggle Terminal
  else if (e.ctrlKey && e.key === '`') {
    e.preventDefault()
    togglePanel('terminal')
  }
  // Ctrl+W - Close Tab
  else if (e.ctrlKey && e.key === 'w') {
    e.preventDefault()
    if (editorStore.activeFilePath) {
      closeTab(editorStore.activeFilePath)
    }
  }
}

// Resize handlers
let isResizing = false
let isPanelResizing = false

function startResize(e: MouseEvent) {
  isResizing = true
  document.addEventListener('mousemove', handleResize)
  document.addEventListener('mouseup', stopResize)
  e.preventDefault()
}

function handleResize(e: MouseEvent) {
  if (!isResizing) return
  const newWidth = e.clientX - 48 // Subtract activity bar width
  sidebarWidth.value = Math.max(200, Math.min(500, newWidth))
}

function stopResize() {
  isResizing = false
  document.removeEventListener('mousemove', handleResize)
  document.removeEventListener('mouseup', stopResize)
}

function startPanelResize(e: MouseEvent) {
  isPanelResizing = true
  document.addEventListener('mousemove', handlePanelResize)
  document.addEventListener('mouseup', stopPanelResize)
  e.preventDefault()
}

function handlePanelResize(e: MouseEvent) {
  if (!isPanelResizing) return
  const editorArea = document.querySelector('.editor-area') as HTMLElement
  if (!editorArea) return
  const rect = editorArea.getBoundingClientRect()
  const newHeight = rect.bottom - e.clientY
  panelHeight.value = Math.max(100, Math.min(400, newHeight))
}

function stopPanelResize() {
  isPanelResizing = false
  document.removeEventListener('mousemove', handlePanelResize)
  document.removeEventListener('mouseup', stopPanelResize)
}

// Quick Open navigation
function navigateQuickOpen(direction: number) {
  const maxIndex = quickOpenResults.value.length - 1
  quickOpenIndex.value = Math.max(0, Math.min(maxIndex, quickOpenIndex.value + direction))
}

function openQuickOpenResult() {
  const result = quickOpenResults.value[quickOpenIndex.value]
  if (result) {
    openFile(result.path)
    showQuickOpen.value = false
  }
}

async function openFile(path: string) {
  await editorStore.openFile(path)
  showQuickOpen.value = false
}

// Command Palette navigation
function navigateCommands(direction: number) {
  const maxIndex = filteredCommands.value.length - 1
  commandIndex.value = Math.max(0, Math.min(maxIndex, commandIndex.value + direction))
}

function executeCommand() {
  const cmd = filteredCommands.value[commandIndex.value]
  if (cmd) {
    runCommand(cmd)
  }
}

function runCommand(cmd: Command) {
  cmd.action()
  showCommandPalette.value = false
}

// Context menu
function handleContextMenu(data: { event: MouseEvent; path: string }) {
  contextMenu.value = {
    visible: true,
    x: data.event.clientX,
    y: data.event.clientY,
    path: data.path
  }
}

function copyPath() {
  navigator.clipboard.writeText(contextMenu.value.path)
}

function renameFile() {
  // TODO: Implement rename
  console.log('Rename:', contextMenu.value.path)
}

function deleteFile() {
  // TODO: Implement delete
  console.log('Delete:', contextMenu.value.path)
}

// Search
function handleSearch() {
  if (!searchQuery.value) {
    searchResults.value = []
    return
  }
  const query = searchQuery.value.toLowerCase()
  searchResults.value = flattenTree(editorStore.fileTree)
    .filter(f => f.name.toLowerCase().includes(query))
}

// Extensions
function installExtension(ext: Extension) {
  installedExtensions.value.push({ ...ext, enabled: true })
}

function uninstallExtension(ext: Extension) {
  installedExtensions.value = installedExtensions.value.filter(e => e.id !== ext.id)
}

function toggleExtension(ext: Extension) {
  const installed = installedExtensions.value.find(e => e.id === ext.id)
  if (installed) {
    installed.enabled = !installed.enabled
  }
}

// File operations
function createNewFile() {
  // TODO: Implement create new file
  console.log('Create new file')
}

function createNewFolder() {
  // TODO: Implement create new folder
  console.log('Create new folder')
}

async function refreshTree() {
  if (owner.value && repo.value) {
    await editorStore.loadRepository(owner.value, repo.value)
  }
}

function showOpenEditors() {
  // TODO: Show open editors dropdown
  console.log('Show open editors')
}

function showBranchMenu() {
  // TODO: Show branch menu
  console.log('Show branch menu')
}

// Helpers
function flattenTree(nodes: typeof editorStore.fileTree): Array<{ path: string; name: string; dir: string }> {
  const result: Array<{ path: string; name: string; dir: string }> = []
  
  function traverse(nodes: typeof editorStore.fileTree) {
    for (const node of nodes) {
      if (node.type === 'file') {
        const parts = node.path.split('/')
        result.push({
          path: node.path,
          name: parts[parts.length - 1],
          dir: parts.slice(0, -1).join('/')
        })
      }
      if (node.children) {
        traverse(node.children)
      }
    }
  }
  
  traverse(nodes)
  return result
}
</script>

<style lang="scss" scoped>
.ide-layout {
  display: grid;
  grid-template-areas:
    "activity sidebar editor"
    "activity sidebar editor"
    "statusbar statusbar statusbar";
  grid-template-columns: var(--ide-activity-bar-width) auto 1fr;
  grid-template-rows: 1fr auto;
  height: 100vh;
  width: 100vw;
  overflow: hidden;
}

// Activity Bar
.activity-bar {
  grid-area: activity;
  background: var(--ide-activity-bar-bg);
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  padding: 4px 0;
  
  .activity-icons {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
  }
  
  .activity-bottom {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
  }
  
  .activity-icon {
    position: relative;
    width: 48px;
    height: 48px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: none;
    border: none;
    color: var(--ide-activity-bar-fg);
    cursor: pointer;
    transition: color var(--ide-transition-fast);
    
    &:hover {
      color: var(--ide-activity-bar-active);
    }
    
    &.active {
      color: var(--ide-activity-bar-active);
      
      &::before {
        content: '';
        position: absolute;
        left: 0;
        top: 50%;
        transform: translateY(-50%);
        width: 2px;
        height: 24px;
        background: var(--ide-activity-bar-indicator);
        border-radius: 0 1px 1px 0;
      }
    }
    
    svg {
      width: 24px;
      height: 24px;
    }
    
    .badge {
      position: absolute;
      top: 8px;
      right: 8px;
      min-width: 16px;
      height: 16px;
      padding: 0 4px;
      background: var(--ide-primary);
      color: var(--ide-bg);
      font-size: 10px;
      font-weight: 600;
      border-radius: 8px;
      display: flex;
      align-items: center;
      justify-content: center;
    }
  }
}

// Sidebar
.sidebar {
  grid-area: sidebar;
  background: var(--ide-sidebar-bg);
  display: flex;
  flex-direction: column;
  position: relative;
  transition: width var(--ide-transition-normal);
  overflow: hidden;
  
  &.collapsed {
    width: 0 !important;
  }
  
  .sidebar-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 12px;
    background: var(--ide-sidebar-header-bg);
    border-bottom: 1px solid var(--ide-border-subtle);
    
    .sidebar-title {
      font-size: 11px;
      font-weight: 600;
      text-transform: uppercase;
      letter-spacing: 0.5px;
      color: var(--ide-text-muted);
    }
    
    .btn-icon {
      background: none;
      border: none;
      color: var(--ide-text-muted);
      cursor: pointer;
      padding: 4px;
      border-radius: 4px;
      
      &:hover {
        background: var(--ide-surface-hover);
        color: var(--ide-text);
      }
      
      svg {
        width: 16px;
        height: 16px;
      }
    }
  }
  
  .sidebar-content {
    flex: 1;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }
  
  .resize-handle {
    position: absolute;
    right: 0;
    top: 0;
    bottom: 0;
    width: 4px;
    cursor: ew-resize;
    
    &:hover {
      background: var(--ide-primary);
    }
  }
}

// Explorer
.explorer-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
  border-bottom: 1px solid var(--ide-border-subtle);
  
  .repo-name {
    font-size: 13px;
    font-weight: 500;
  }
  
  .explorer-actions {
    display: flex;
    gap: 4px;
  }
}

.file-tree {
  flex: 1;
  overflow-y: auto;
  padding: 4px 0;
}

// Search
.search-input-wrapper {
  padding: 8px 12px;
}

.search-results {
  flex: 1;
  overflow-y: auto;
}

.search-empty {
  padding: 20px;
  text-align: center;
  color: var(--ide-text-muted);
  font-size: 12px;
}

.search-result-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 12px;
  cursor: pointer;
  
  &:hover {
    background: var(--ide-sidebar-item-hover);
  }
  
  .result-name {
    flex: 1;
    font-size: 13px;
  }
  
  .result-path {
    font-size: 11px;
    color: var(--ide-text-muted);
  }
}

// Extensions
.extensions-header {
  padding: 8px 12px;
}

.extensions-list {
  flex: 1;
  overflow-y: auto;
}

.extension-section {
  padding: 8px 0;
  
  h4 {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--ide-text-muted);
    padding: 0 12px 8px;
  }
}

// Git
.git-section {
  padding: 12px;
}

.branch-selector {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px;
  background: var(--ide-surface);
  border-radius: 4px;
  margin-bottom: 16px;
  
  svg {
    width: 16px;
    height: 16px;
    color: var(--ide-text-muted);
  }
  
  .branch-select {
    flex: 1;
    background: none;
    border: none;
    color: var(--ide-text);
    font-size: 13px;
    cursor: pointer;
    outline: none;
  }
}

.changes-section {
  h4 {
    font-size: 12px;
    font-weight: 600;
    color: var(--ide-text-muted);
    margin-bottom: 8px;
  }
}

.changed-file {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px 8px;
  font-size: 13px;
  
  .change-indicator {
    width: 16px;
    height: 16px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 10px;
    font-weight: 600;
    border-radius: 2px;
    
    &.create { background: var(--ide-success); color: var(--ide-bg); }
    &.update { background: var(--ide-warning); color: var(--ide-bg); }
    &.delete { background: var(--ide-danger); color: white; }
  }
}

.empty-state {
  padding: 20px;
  text-align: center;
  color: var(--ide-text-muted);
  font-size: 12px;
}

// Editor Area
.editor-area {
  grid-area: editor;
  display: flex;
  flex-direction: column;
  background: var(--ide-editor-bg);
  overflow: hidden;
}

// Tab Bar
.tab-bar {
  display: flex;
  align-items: center;
  background: var(--ide-tab-bg);
  border-bottom: 1px solid var(--ide-border-subtle);
  height: var(--ide-tab-height);
  
  .tabs-container {
    display: flex;
    flex: 1;
    overflow-x: auto;
    
    &::-webkit-scrollbar {
      height: 3px;
    }
  }
  
  .tab {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 12px;
    height: var(--ide-tab-height);
    background: var(--ide-tab-bg);
    color: var(--ide-tab-fg);
    border-right: 1px solid var(--ide-tab-border);
    cursor: pointer;
    white-space: nowrap;
    font-size: 13px;
    transition: background var(--ide-transition-fast);
    
    &:hover {
      background: var(--ide-surface-hover);
    }
    
    &.active {
      background: var(--ide-tab-active-bg);
      color: var(--ide-tab-active-fg);
      border-bottom: 2px solid var(--ide-primary);
      margin-bottom: -1px;
    }
    
    .tab-name {
      max-width: 150px;
      overflow: hidden;
      text-overflow: ellipsis;
    }
    
    .tab-modified {
      color: var(--ide-tab-modified);
      font-size: 16px;
      line-height: 1;
    }
    
    .tab-close {
      background: none;
      border: none;
      color: var(--ide-text-subtle);
      cursor: pointer;
      padding: 2px;
      border-radius: 4px;
      display: flex;
      opacity: 0;
      
      &:hover {
        background: var(--ide-surface-hover);
        color: var(--ide-text);
      }
      
      svg {
        width: 14px;
        height: 14px;
      }
    }
    
    &:hover .tab-close,
    &.active .tab-close {
      opacity: 1;
    }
  }
  
  .tab-actions {
    padding: 0 8px;
  }
}

// Editor Container
.editor-container {
  flex: 1;
  overflow: hidden;
}

// Welcome Screen
.welcome-screen {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  
  .welcome-content {
    text-align: center;
    max-width: 400px;
    
    h2 {
      font-size: 24px;
      font-weight: 600;
      margin-bottom: 8px;
      color: var(--ide-text);
    }
    
    p {
      color: var(--ide-text-muted);
      margin-bottom: 24px;
    }
  }
  
  .quick-actions {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  
  .quick-action {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 16px;
    background: var(--ide-surface);
    border: 1px solid var(--ide-border);
    border-radius: 6px;
    cursor: pointer;
    text-align: left;
    color: var(--ide-text);
    transition: all var(--ide-transition-fast);
    
    &:hover {
      background: var(--ide-surface-hover);
      border-color: var(--ide-primary);
    }
    
    svg {
      width: 18px;
      height: 18px;
      color: var(--ide-primary);
    }
    
    span {
      flex: 1;
    }
    
    kbd {
      padding: 2px 6px;
      background: var(--ide-bg);
      border-radius: 4px;
      font-size: 11px;
      color: var(--ide-text-muted);
    }
  }
}

// Bottom Panel
.bottom-panel {
  background: var(--ide-panel-bg);
  border-top: 1px solid var(--ide-panel-border);
  display: flex;
  flex-direction: column;
  
  .panel-resize-handle {
    height: 4px;
    cursor: ns-resize;
    
    &:hover {
      background: var(--ide-primary);
    }
  }
  
  .panel-tabs {
    display: flex;
    align-items: center;
    padding: 0 8px;
    background: var(--ide-panel-header-bg);
    border-bottom: 1px solid var(--ide-border-subtle);
    
    .panel-tab {
      padding: 8px 12px;
      font-size: 12px;
      color: var(--ide-text-muted);
      background: none;
      border: none;
      cursor: pointer;
      
      &:hover {
        color: var(--ide-text);
      }
      
      &.active {
        color: var(--ide-text);
        border-bottom: 2px solid var(--ide-primary);
        margin-bottom: -1px;
      }
    }
    
    .panel-close {
      margin-left: auto;
      background: none;
      border: none;
      color: var(--ide-text-muted);
      cursor: pointer;
      padding: 4px;
      
      &:hover {
        color: var(--ide-text);
      }
      
      svg {
        width: 14px;
        height: 14px;
      }
    }
  }
  
  .panel-content {
    flex: 1;
    overflow: hidden;
  }
}

// Status Bar
.status-bar {
  grid-area: statusbar;
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: var(--ide-statusbar-height);
  padding: 0 8px;
  background: var(--ide-statusbar-bg);
  color: var(--ide-statusbar-fg);
  font-size: 12px;
  
  .status-left,
  .status-right {
    display: flex;
    align-items: center;
    gap: 2px;
  }
  
  .status-item {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 0 8px;
    height: 100%;
    cursor: pointer;
    
    &:hover {
      background: rgba(0, 0, 0, 0.2);
    }
    
    &.warning {
      color: var(--ide-warning);
      
      .dot {
        width: 6px;
        height: 6px;
        background: currentColor;
        border-radius: 50%;
      }
    }
    
    svg {
      width: 14px;
      height: 14px;
    }
  }
}

// Quick Open Modal
.quick-open-modal {
  width: 600px;
  max-width: 90vw;
  background: var(--ide-bg-secondary);
  border: 1px solid var(--ide-border);
  border-radius: 8px;
  box-shadow: var(--ide-shadow-lg);
  overflow: hidden;
  
  .quick-open-input {
    width: 100%;
    padding: 12px 16px;
    font-size: 14px;
    background: none;
    border: none;
    border-bottom: 1px solid var(--ide-border);
    color: var(--ide-text);
    outline: none;
    
    &::placeholder {
      color: var(--ide-text-subtle);
    }
  }
  
  .quick-open-results {
    max-height: 400px;
    overflow-y: auto;
  }
  
  .quick-open-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 16px;
    cursor: pointer;
    
    &:hover,
    &.selected {
      background: var(--ide-surface-hover);
    }
    
    .result-name {
      font-size: 14px;
      color: var(--ide-text);
    }
    
    .result-path {
      font-size: 12px;
      color: var(--ide-text-muted);
    }
  }
}

// Command Palette Modal
.command-palette-modal {
  width: 600px;
  max-width: 90vw;
  background: var(--ide-bg-secondary);
  border: 1px solid var(--ide-border);
  border-radius: 8px;
  box-shadow: var(--ide-shadow-lg);
  overflow: hidden;
  
  .command-palette-input {
    width: 100%;
    padding: 12px 16px;
    font-size: 14px;
    background: none;
    border: none;
    border-bottom: 1px solid var(--ide-border);
    color: var(--ide-text);
    outline: none;
    
    &::placeholder {
      color: var(--ide-text-subtle);
    }
  }
  
  .command-results {
    max-height: 400px;
    overflow-y: auto;
  }
  
  .command-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 16px;
    cursor: pointer;
    
    &:hover,
    &.selected {
      background: var(--ide-surface-hover);
    }
    
    .command-label {
      font-size: 14px;
      color: var(--ide-text);
    }
    
    .command-keybinding {
      font-size: 12px;
      color: var(--ide-text-muted);
      padding: 2px 6px;
      background: var(--ide-surface);
      border-radius: 4px;
    }
  }
}

// Settings Modal
.settings-modal {
  width: 800px;
  max-width: 90vw;
  height: 600px;
  max-height: 90vh;
}

// Utility classes
.btn-icon {
  background: none;
  border: none;
  color: var(--ide-text-muted);
  cursor: pointer;
  padding: 4px;
  border-radius: 4px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  
  &:hover {
    background: var(--ide-surface-hover);
    color: var(--ide-text);
  }
  
  svg {
    width: 16px;
    height: 16px;
  }
  
  &.btn-sm svg {
    width: 14px;
    height: 14px;
  }
}
</style>
