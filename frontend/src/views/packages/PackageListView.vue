<template>
  <div class="package-list">
    <!-- 顶栏：标签页 + 搜索 -->
    <div class="package-toolbar">
      <div class="toolbar-tabs">
        <button
          class="tab-btn"
          :class="{ active: activeTab === 'all' }"
          @click="activeTab = 'all'"
        >
          全部
          <span class="tab-count">{{ totalCount }}</span>
        </button>
        <button
          class="tab-btn"
          :class="{ active: activeTab === 'docker' }"
          @click="activeTab = 'docker'"
        >
          <svg class="tab-icon" viewBox="0 0 16 16" fill="none">
            <path d="M1.5 8h2v2h-2zM4.5 8h2v2h-2zM7.5 8h2v2h-2zM4.5 5h2v2h-2zM7.5 5h2v2h-2zM10.5 6c.5-1 1.5-1.5 3-1.5.3 1 .2 2-.5 3H1c0-3 1.5-5.5 4.5-5.5 1 0 2 .5 2.5 1.5h2c.5-1 1-1.5 2-1.5" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
          Docker
          <span class="tab-count">{{ dockerCount }}</span>
        </button>
        <button
          class="tab-btn"
          :class="{ active: activeTab === 'npm' }"
          @click="activeTab = 'npm'"
        >
          <svg class="tab-icon" viewBox="0 0 16 16" fill="none">
            <path d="M2 3h12v10H2V3zM5 6v4h2V7h1v3h3V6H5z" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
          npm
          <span class="tab-count">{{ npmCount }}</span>
        </button>
      </div>
      <div class="toolbar-actions">
        <div class="search-input">
          <svg viewBox="0 0 16 16" width="14" height="14">
            <circle cx="7" cy="7" r="5" stroke="currentColor" fill="none" stroke-width="1.5"/>
            <path d="M11 11l3 3" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          </svg>
          <input
            type="text"
            v-model="searchQuery"
            placeholder="搜索软件包..."
            @input="debouncedSearch"
          />
        </div>
        <select v-model="sortBy" class="sort-select">
          <option value="updated_at">按更新时间</option>
          <option value="created_at">按创建时间</option>
          <option value="name">按名称</option>
          <option value="version">按版本</option>
        </select>
      </div>
    </div>

    <!-- 加载中 -->
    <div v-if="loading" class="loading-state">
      <div class="loading-spinner"></div>
    </div>

    <!-- 空状态 -->
    <div v-else-if="filteredPackages.length === 0" class="empty-state">
      <svg viewBox="0 0 48 48" width="48" height="48">
        <path d="M24 4L4 14v20l20 10 20-10V14L24 4z" stroke="currentColor" stroke-width="2" fill="none"/>
        <path d="M24 24v20M4 14l20 10 20-10" stroke="currentColor" stroke-width="2"/>
      </svg>
      <h3>暂无软件包</h3>
      <p v-if="searchQuery">未找到匹配 "<strong>{{ searchQuery }}</strong>" 的软件包</p>
      <p v-else>此项目还没有发布任何软件包，查看下方的使用说明开始使用</p>
    </div>

    <!-- 软件包表格 -->
    <div v-else class="packages-table">
      <div
        v-for="pkg in filteredPackages"
        :key="pkg.id"
        class="package-row"
        @click="goToPackage(pkg)"
      >
        <!-- 类型图标 -->
        <div class="col-type">
          <div class="type-icon" :class="pkg.package_type">
            <svg v-if="pkg.package_type === 'docker'" viewBox="0 0 16 16" fill="none">
              <path d="M1.5 8h2v2h-2zM4.5 8h2v2h-2zM7.5 8h2v2h-2zM4.5 5h2v2h-2zM7.5 5h2v2h-2zM10.5 6c.5-1 1.5-1.5 3-1.5.3 1 .2 2-.5 3H1c0-3 1.5-5.5 4.5-5.5 1 0 2 .5 2.5 1.5h2c.5-1 1-1.5 2-1.5" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
            <svg v-else-if="pkg.package_type === 'npm'" viewBox="0 0 16 16" fill="none">
              <path d="M2 3h12v10H2V3zM5 6v4h2V7h1v3h3V6H5z" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
            <svg v-else viewBox="0 0 16 16" fill="none">
              <path d="M8 1L1 4v8l7 3 7-3V4L8 1zM8 8v7M1 4l7 4 7-4" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
          </div>
        </div>

        <!-- 包信息 -->
        <div class="col-info">
          <div class="package-name">
            <router-link
              :to="{ name: 'PackageDetail', params: { ...route.params, packageId: pkg.id.toString() } }"
              class="name-link"
              @click.stop
            >
              {{ pkg.name }}
            </router-link>
            <span class="type-badge" :class="pkg.package_type">{{ pkg.package_type }}</span>
          </div>
          <div class="package-meta">
            <span class="version">v{{ pkg.version }}</span>
            <span class="separator">·</span>
            <span class="date">{{ formatDate(pkg.created_at) }}</span>
            <template v-if="pkg.size">
              <span class="separator">·</span>
              <span class="size">{{ formatSize(pkg.size) }}</span>
            </template>
          </div>
        </div>

        <!-- 操作 -->
        <div class="col-actions">
          <button class="action-btn copy" title="复制安装命令" @click.stop="copyInstallCommand(pkg)">
            <svg viewBox="0 0 16 16" width="14" height="14">
              <rect x="5" y="5" width="8" height="10" rx="1" stroke="currentColor" fill="none" stroke-width="1.5"/>
              <path d="M3 11V3a1 1 0 011-1h6" stroke="currentColor" stroke-width="1.5" fill="none"/>
            </svg>
          </button>
        </div>
      </div>
    </div>

    <!-- 使用说明（折叠面板） -->
    <details class="usage-panel" open>
      <summary class="usage-header">
        <svg viewBox="0 0 16 16" width="14" height="14" class="chevron">
          <path d="M6 4l4 4-4 4" stroke="currentColor" stroke-width="1.5" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
        使用说明
      </summary>
      <div class="usage-content">
        <div class="usage-section" v-if="activeTab === 'all' || activeTab === 'docker'">
          <div class="section-header">
            <svg class="section-icon docker" viewBox="0 0 16 16" fill="none">
              <path d="M1.5 8h2v2h-2zM4.5 8h2v2h-2zM7.5 8h2v2h-2zM4.5 5h2v2h-2zM7.5 5h2v2h-2zM10.5 6c.5-1 1.5-1.5 3-1.5.3 1 .2 2-.5 3H1c0-3 1.5-5.5 4.5-5.5 1 0 2 .5 2.5 1.5h2c.5-1 1-1.5 2-1.5" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
            <span>Docker Registry</span>
          </div>
          <div class="code-block">
            <div class="code-label">登录</div>
            <pre><code>docker login {{ registryDomain }}</code></pre>
          </div>
          <div class="code-block">
            <div class="code-label">推送镜像</div>
            <pre><code>docker tag myimage {{ registryDomain }}/{{ namespace }}/{{ projectName }}/myimage:latest
docker push {{ registryDomain }}/{{ namespace }}/{{ projectName }}/myimage:latest</code></pre>
          </div>
          <div class="code-block">
            <div class="code-label">拉取镜像</div>
            <pre><code>docker pull {{ registryDomain }}/{{ namespace }}/{{ projectName }}/myimage:latest</code></pre>
          </div>
        </div>

        <div class="usage-section" v-if="activeTab === 'all' || activeTab === 'npm'">
          <div class="section-header">
            <svg class="section-icon npm" viewBox="0 0 16 16" fill="none">
              <path d="M2 3h12v10H2V3zM5 6v4h2V7h1v3h3V6H5z" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
            <span>npm Registry</span>
          </div>
          <div class="code-block">
            <div class="code-label">配置 .npmrc</div>
            <pre><code>@{{ namespace }}:registry=https://{{ registryDomain }}/npm/
//{{ registryDomain }}/npm/:_authToken=YOUR_PERSONAL_ACCESS_TOKEN</code></pre>
          </div>
          <div class="code-block">
            <div class="code-label">发布包</div>
            <pre><code>npm publish</code></pre>
          </div>
          <div class="code-block">
            <div class="code-label">安装包</div>
            <pre><code>npm install @{{ namespace }}/package-name</code></pre>
          </div>
        </div>
      </div>
    </details>

    <!-- 复制成功提示 -->
    <Transition name="toast">
      <div v-if="copySuccess" class="toast-message">
        <svg viewBox="0 0 16 16" width="14" height="14">
          <path d="M3 8l4 4 6-8" stroke="currentColor" stroke-width="2" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
        已复制到剪贴板
      </div>
    </Transition>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { api } from '@/api'
import type { Package } from '@/types'

const props = defineProps<{
  project?: {
    id: number
    name: string
    namespace: {
      path: string
    }
  }
}>()

const route = useRoute()
const router = useRouter()

// 状态
const loading = ref(false)
const packages = ref<Package[]>([])
const activeTab = ref<'all' | 'docker' | 'npm'>('all')
const searchQuery = ref('')
const sortBy = ref('updated_at')
const copySuccess = ref(false)
const copySuccessTimeout = ref<number | null>(null)

// 从项目或路由获取 namespace 和 project 信息
const namespace = computed(() => props.project?.namespace?.path || route.params.namespace as string)
const projectName = computed(() => props.project?.name || route.params.project as string)

// Registry 域名 - 使用当前域名
const registryDomain = computed(() => {
  // 对于统一入口部署，直接使用当前主机名
  // 对于分离部署场景，可以配置子域名如 registry.example.com
  return window.location.host
})

// 计数
const totalCount = computed(() => packages.value.length)
const dockerCount = computed(() => packages.value.filter(p => p.package_type === 'docker').length)
const npmCount = computed(() => packages.value.filter(p => p.package_type === 'npm').length)

// 过滤后的包列表
const filteredPackages = computed(() => {
  let result = packages.value

  // 按类型过滤
  if (activeTab.value !== 'all') {
    result = result.filter(p => p.package_type === activeTab.value)
  }

  // 按搜索词过滤
  if (searchQuery.value) {
    const query = searchQuery.value.toLowerCase()
    result = result.filter(p => 
      p.name.toLowerCase().includes(query) ||
      p.version.toLowerCase().includes(query)
    )
  }

  // 排序
  result = [...result].sort((a, b) => {
    switch (sortBy.value) {
      case 'name':
        return a.name.localeCompare(b.name)
      case 'created_at':
        return new Date(b.created_at).getTime() - new Date(a.created_at).getTime()
      case 'updated_at':
        return new Date(b.updated_at || b.created_at).getTime() - new Date(a.updated_at || a.created_at).getTime()
      case 'version':
        return b.version.localeCompare(a.version)
      default:
        return 0
    }
  })

  return result
})

// 加载包列表
async function loadPackages() {
  if (!namespace.value || !projectName.value) return
  
  loading.value = true
  try {
    const response = await api.packages.list(namespace.value, projectName.value, {
      package_type: activeTab.value === 'all' ? undefined : activeTab.value
    })
    packages.value = response.data || []
  } catch (error) {
    console.error('Failed to load packages:', error)
    packages.value = []
  } finally {
    loading.value = false
  }
}

// 防抖搜索
let searchTimeout: number | null = null
function debouncedSearch() {
  if (searchTimeout) clearTimeout(searchTimeout)
  searchTimeout = window.setTimeout(() => {
    // 搜索是本地过滤，不需要重新加载
  }, 300)
}

// 跳转到包详情
function goToPackage(pkg: Package) {
  router.push({
    name: 'PackageDetail',
    params: {
      ...route.params,
      packageId: pkg.id.toString()
    }
  })
}

// 复制安装命令
function copyInstallCommand(pkg: Package) {
  let command = ''
  if (pkg.package_type === 'docker') {
    command = `docker pull ${registryDomain.value}/${namespace.value}/${projectName.value}/${pkg.name}:${pkg.version}`
  } else if (pkg.package_type === 'npm') {
    command = `npm install @${namespace.value}/${pkg.name}@${pkg.version}`
  }
  
  navigator.clipboard.writeText(command)
  
  // 显示复制成功提示
  if (copySuccessTimeout.value) {
    clearTimeout(copySuccessTimeout.value)
  }
  copySuccess.value = true
  copySuccessTimeout.value = window.setTimeout(() => {
    copySuccess.value = false
  }, 2000)
}

// 格式化日期
function formatDate(dateStr: string): string {
  const date = new Date(dateStr)
  const now = new Date()
  const diff = now.getTime() - date.getTime()
  
  if (diff < 60000) return '刚刚'
  if (diff < 3600000) return `${Math.floor(diff / 60000)} 分钟前`
  if (diff < 86400000) return `${Math.floor(diff / 3600000)} 小时前`
  if (diff < 604800000) return `${Math.floor(diff / 86400000)} 天前`
  
  return date.toLocaleDateString('zh-CN')
}

// 格式化文件大小
function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`
  return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`
}

// 监听 tab 变化
watch(activeTab, () => {
  loadPackages()
})

onMounted(() => {
  loadPackages()
})
</script>

<style lang="scss" scoped>
// ── 工具栏 ────────────────────────────────────────────────
.package-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 16px;
  border-bottom: 1px solid $border-color;
}

.toolbar-tabs {
  display: flex;
  gap: 4px;
}

.tab-btn {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 12px;
  background: none;
  border: none;
  border-bottom: 2px solid transparent;
  font-size: $font-size-sm;
  color: $text-secondary;
  cursor: pointer;
  transition: all 0.15s;
  border-radius: 0;

  &:hover {
    color: $text-primary;
  }

  &.active {
    color: $text-primary;
    font-weight: 500;
    border-bottom-color: $primary-color;
  }

  .tab-icon {
    width: 14px;
    height: 14px;
    flex-shrink: 0;
  }

  .tab-count {
    background: $bg-secondary;
    border-radius: 10px;
    padding: 1px 6px;
    font-size: 11px;
    min-width: 18px;
    text-align: center;
  }
}

.toolbar-actions {
  display: flex;
  align-items: center;
  gap: 12px;
}

.search-input {
  position: relative;
  display: flex;
  align-items: center;

  svg {
    position: absolute;
    left: 10px;
    color: $text-tertiary;
  }

  input {
    padding: 6px 12px 6px 32px;
    border: 1px solid $border-color;
    border-radius: $border-radius;
    font-size: $font-size-sm;
    width: 200px;
    background: $bg-primary;
    color: $text-primary;

    &::placeholder {
      color: $text-tertiary;
    }

    &:focus {
      outline: none;
      border-color: $primary-color;
      box-shadow: 0 0 0 2px rgba($primary-color, 0.1);
    }
  }
}

.sort-select {
  padding: 6px 12px;
  border: 1px solid $border-color;
  border-radius: $border-radius;
  font-size: $font-size-sm;
  background: $bg-primary;
  color: $text-primary;
  cursor: pointer;

  &:focus {
    outline: none;
    border-color: $primary-color;
  }
}

// ── 加载状态 ────────────────────────────────────────────────
.loading-state {
  display: flex;
  justify-content: center;
  padding: 48px;
}

.loading-spinner {
  width: 24px;
  height: 24px;
  border: 2px solid $border-color;
  border-top-color: $primary-color;
  border-radius: 50%;
  animation: spin 1s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

// ── 空状态 ────────────────────────────────────────────────
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  padding: 64px 24px;
  color: $text-secondary;

  svg {
    margin-bottom: 16px;
    color: $text-tertiary;
    opacity: 0.5;
  }

  h3 {
    margin: 0 0 8px;
    font-size: $font-size-lg;
    color: $text-primary;
  }

  p {
    margin: 0;
    font-size: $font-size-sm;
    color: $text-secondary;

    strong {
      color: $text-primary;
    }
  }
}

// ── 表格 ────────────────────────────────────────────────
.packages-table {
  border-top: 1px solid $border-color;
}

.package-row {
  display: grid;
  grid-template-columns: 48px 1fr 48px;
  align-items: center;
  padding: 12px 16px;
  border-bottom: 1px solid $border-color;
  cursor: pointer;
  transition: background 0.1s;

  &:last-child {
    border-bottom: none;
  }

  &:hover {
    background: $bg-secondary;
  }
}

// ── 类型图标列 ────────────────────────────────────────────
.col-type {
  .type-icon {
    width: 32px;
    height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: $border-radius;
    background: $bg-tertiary;

    svg {
      width: 16px;
      height: 16px;
    }

    &.docker {
      background: rgba(13, 183, 237, 0.1);
      color: #0db7ed;
    }

    &.npm {
      background: rgba(203, 55, 53, 0.1);
      color: #cb3735;
    }
  }
}

// ── 信息列 ────────────────────────────────────────────────
.col-info {
  min-width: 0;
  padding: 0 12px;
}

.package-name {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 2px;

  .name-link {
    font-weight: 600;
    font-size: $font-size-sm;
    color: $text-primary;
    text-decoration: none;

    &:hover {
      color: $primary-color;
      text-decoration: underline;
    }
  }

  .type-badge {
    font-size: 11px;
    padding: 1px 6px;
    border-radius: 3px;
    font-weight: 500;
    text-transform: uppercase;

    &.docker {
      background: rgba(13, 183, 237, 0.1);
      color: #0db7ed;
    }

    &.npm {
      background: rgba(203, 55, 53, 0.1);
      color: #cb3735;
    }
  }
}

.package-meta {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 12px;
  color: $text-secondary;

  .version {
    font-family: monospace;
    font-weight: 500;
  }

  .separator {
    color: $text-tertiary;
  }
}

// ── 操作列 ────────────────────────────────────────────────
.col-actions {
  display: flex;
  justify-content: flex-end;
}

.action-btn {
  width: 28px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: none;
  border: none;
  border-radius: $border-radius;
  color: $text-secondary;
  cursor: pointer;
  transition: all 0.15s;

  &:hover {
    background: $bg-tertiary;
    color: $text-primary;
  }

  &.copy:hover {
    color: $primary-color;
  }
}

// ── 使用说明面板 ────────────────────────────────────────
.usage-panel {
  margin-top: 24px;
  border: 1px solid $border-color;
  border-radius: $border-radius;
  background: $bg-primary;
}

.usage-header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 12px 16px;
  font-size: $font-size-sm;
  font-weight: 500;
  color: $text-primary;
  cursor: pointer;
  user-select: none;

  &::-webkit-details-marker {
    display: none;
  }

  .chevron {
    transition: transform 0.2s;
  }
}

.usage-panel[open] .chevron {
  transform: rotate(90deg);
}

.usage-content {
  padding: 0 16px 16px;
  border-top: 1px solid $border-color;
}

.usage-section {
  margin-top: 16px;

  &:first-child {
    margin-top: 16px;
  }
}

.section-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 12px;
  font-size: $font-size-sm;
  font-weight: 600;
  color: $text-primary;

  .section-icon {
    width: 16px;
    height: 16px;

    &.docker {
      color: #0db7ed;
    }

    &.npm {
      color: #cb3735;
    }
  }
}

.code-block {
  margin-bottom: 12px;
  border: 1px solid $border-color;
  border-radius: $border-radius;
  overflow: hidden;

  &:last-child {
    margin-bottom: 0;
  }

  .code-label {
    padding: 6px 12px;
    font-size: 11px;
    font-weight: 500;
    color: $text-secondary;
    background: $bg-secondary;
    border-bottom: 1px solid $border-color;
  }

  pre {
    margin: 0;
    padding: 12px;
    background: $bg-primary;
    overflow-x: auto;

    code {
      font-family: 'Monaco', 'Menlo', 'Consolas', monospace;
      font-size: 12px;
      color: $text-primary;
      line-height: 1.5;
    }
  }
}

// ── Toast 提示 ────────────────────────────────────────────
.toast-message {
  position: fixed;
  bottom: 24px;
  left: 50%;
  transform: translateX(-50%);
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 16px;
  background: $gray-800;
  color: white;
  font-size: $font-size-sm;
  border-radius: $border-radius;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
  z-index: 1000;

  svg {
    color: $color-success;
  }
}

.toast-enter-active,
.toast-leave-active {
  transition: all 0.25s ease;
}

.toast-enter-from,
.toast-leave-to {
  opacity: 0;
  transform: translateX(-50%) translateY(10px);
}
</style>
