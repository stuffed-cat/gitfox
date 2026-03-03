<template>
  <div class="package-list-view">
    <div class="page-header">
      <h1>软件包仓库</h1>
      <p class="subtitle">管理项目的 Docker 镜像和 npm 包</p>
    </div>

    <!-- 类型切换 -->
    <div class="package-tabs">
      <button 
        :class="['tab-btn', { active: activeTab === 'all' }]" 
        @click="activeTab = 'all'"
      >
        全部 <span class="badge" v-if="totalCount">{{ totalCount }}</span>
      </button>
      <button 
        :class="['tab-btn', { active: activeTab === 'docker' }]" 
        @click="activeTab = 'docker'"
      >
        <span class="icon">🐳</span> Docker
        <span class="badge" v-if="dockerCount">{{ dockerCount }}</span>
      </button>
      <button 
        :class="['tab-btn', { active: activeTab === 'npm' }]" 
        @click="activeTab = 'npm'"
      >
        <span class="icon">📦</span> npm
        <span class="badge" v-if="npmCount">{{ npmCount }}</span>
      </button>
    </div>

    <!-- 搜索和排序 -->
    <div class="filter-bar">
      <div class="search-box">
        <input 
          type="text" 
          v-model="searchQuery" 
          placeholder="搜索软件包..."
          @input="debouncedSearch"
        />
      </div>
      <div class="sort-box">
        <select v-model="sortBy">
          <option value="name">按名称</option>
          <option value="created_at">按创建时间</option>
          <option value="updated_at">按更新时间</option>
          <option value="version">按版本</option>
        </select>
      </div>
    </div>

    <!-- 加载中 -->
    <div v-if="loading" class="loading-state">
      <div class="spinner"></div>
      <span>加载中...</span>
    </div>

    <!-- 空状态 -->
    <div v-else-if="filteredPackages.length === 0" class="empty-state">
      <div class="empty-icon">📦</div>
      <h3>暂无软件包</h3>
      <p v-if="searchQuery">未找到匹配"{{ searchQuery }}"的软件包</p>
      <p v-else>
        此项目还没有发布任何软件包。
        <br />
        查看下方的使用说明开始使用。
      </p>
    </div>

    <!-- 软件包列表 -->
    <div v-else class="package-list">
      <div 
        v-for="pkg in filteredPackages" 
        :key="pkg.id" 
        class="package-item"
        @click="goToPackage(pkg)"
      >
        <div class="package-icon">
          <span v-if="pkg.package_type === 'docker'">🐳</span>
          <span v-else-if="pkg.package_type === 'npm'">📦</span>
          <span v-else>📁</span>
        </div>
        <div class="package-info">
          <div class="package-name">
            {{ pkg.name }}
            <span class="package-type-badge" :class="pkg.package_type">
              {{ pkg.package_type }}
            </span>
          </div>
          <div class="package-meta">
            <span class="version">{{ pkg.version }}</span>
            <span class="separator">·</span>
            <span class="date">{{ formatDate(pkg.created_at) }}</span>
            <span class="separator" v-if="pkg.size">·</span>
            <span class="size" v-if="pkg.size">{{ formatSize(pkg.size) }}</span>
          </div>
        </div>
        <div class="package-actions">
          <button class="btn btn-sm" @click.stop="copyInstallCommand(pkg)">
            复制命令
          </button>
        </div>
      </div>
    </div>

    <!-- 使用说明 -->
    <div class="usage-guide">
      <h3>使用说明</h3>
      
      <div class="guide-section" v-if="activeTab === 'all' || activeTab === 'docker'">
        <h4>🐳 Docker Registry</h4>
        <div class="code-block">
          <div class="code-header">登录</div>
          <pre><code>docker login {{ registryDomain }}</code></pre>
        </div>
        <div class="code-block">
          <div class="code-header">推送镜像</div>
          <pre><code>docker tag myimage {{ registryDomain }}/{{ namespace }}/{{ projectName }}/myimage:latest
docker push {{ registryDomain }}/{{ namespace }}/{{ projectName }}/myimage:latest</code></pre>
        </div>
        <div class="code-block">
          <div class="code-header">拉取镜像</div>
          <pre><code>docker pull {{ registryDomain }}/{{ namespace }}/{{ projectName }}/myimage:latest</code></pre>
        </div>
      </div>

      <div class="guide-section" v-if="activeTab === 'all' || activeTab === 'npm'">
        <h4>📦 npm Registry</h4>
        <div class="code-block">
          <div class="code-header">配置 .npmrc</div>
          <pre><code>@{{ namespace }}:registry=https://{{ registryDomain }}/npm/
//{{ registryDomain }}/npm/:_authToken=YOUR_PERSONAL_ACCESS_TOKEN</code></pre>
        </div>
        <div class="code-block">
          <div class="code-header">发布包</div>
          <pre><code>npm publish</code></pre>
        </div>
        <div class="code-block">
          <div class="code-header">安装包</div>
          <pre><code>npm install @{{ namespace }}/package-name</code></pre>
        </div>
      </div>
    </div>
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

// 从项目或路由获取 namespace 和 project 信息
const namespace = computed(() => props.project?.namespace?.path || route.params.namespace as string)
const projectName = computed(() => props.project?.name || route.params.project as string)

// Registry 域名（从配置获取，这里使用默认值）
const registryDomain = computed(() => {
  // TODO: 从系统配置获取
  return window.location.hostname.replace(/^[^.]+/, 'registry')
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
  // TODO: 显示复制成功提示
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
.package-list-view {
  padding: $spacing-lg;
}

.page-header {
  margin-bottom: $spacing-lg;

  h1 {
    font-size: $font-size-xxl;
    margin-bottom: $spacing-xs;
  }

  .subtitle {
    color: $text-muted;
    margin: 0;
  }
}

.package-tabs {
  display: flex;
  gap: $spacing-sm;
  margin-bottom: $spacing-md;
  border-bottom: 1px solid $border-color;
  padding-bottom: $spacing-sm;

  .tab-btn {
    display: flex;
    align-items: center;
    gap: $spacing-xs;
    padding: $spacing-sm $spacing-md;
    background: none;
    border: none;
    border-radius: $border-radius;
    cursor: pointer;
    font-size: $font-size-base;
    color: $text-muted;
    transition: all 0.2s;

    &:hover {
      background: $bg-secondary;
      color: $text-primary;
    }

    &.active {
      background: $primary-color;
      color: white;
    }

    .badge {
      background: rgba(255, 255, 255, 0.2);
      padding: 2px 6px;
      border-radius: 10px;
      font-size: $font-size-sm;
    }
  }
}

.filter-bar {
  display: flex;
  gap: $spacing-md;
  margin-bottom: $spacing-lg;

  .search-box {
    flex: 1;

    input {
      width: 100%;
      padding: $spacing-sm $spacing-md;
      border: 1px solid $border-color;
      border-radius: $border-radius;
      font-size: $font-size-base;

      &:focus {
        outline: none;
        border-color: $primary-color;
      }
    }
  }

  .sort-box {
    select {
      padding: $spacing-sm $spacing-md;
      border: 1px solid $border-color;
      border-radius: $border-radius;
      font-size: $font-size-base;
      background: $bg-primary;
    }
  }
}

.loading-state {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: $spacing-sm;
  padding: $spacing-xxl;
  color: $text-muted;

  .spinner {
    width: 20px;
    height: 20px;
    border: 2px solid $border-color;
    border-top-color: $primary-color;
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }
}

.empty-state {
  text-align: center;
  padding: $spacing-xxl;
  color: $text-muted;

  .empty-icon {
    font-size: 48px;
    margin-bottom: $spacing-md;
  }

  h3 {
    margin-bottom: $spacing-sm;
    color: $text-primary;
  }
}

.package-list {
  display: flex;
  flex-direction: column;
  gap: $spacing-sm;
}

.package-item {
  display: flex;
  align-items: center;
  gap: $spacing-md;
  padding: $spacing-md;
  background: $bg-primary;
  border: 1px solid $border-color;
  border-radius: $border-radius;
  cursor: pointer;
  transition: all 0.2s;

  &:hover {
    background: $bg-secondary;
    border-color: $primary-color;
  }

  .package-icon {
    font-size: 24px;
    width: 40px;
    height: 40px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: $bg-secondary;
    border-radius: $border-radius;
  }

  .package-info {
    flex: 1;

    .package-name {
      font-weight: 500;
      display: flex;
      align-items: center;
      gap: $spacing-sm;

      .package-type-badge {
        font-size: $font-size-xs;
        padding: 2px 6px;
        border-radius: 4px;
        font-weight: normal;

        &.docker {
          background: #0db7ed20;
          color: #0db7ed;
        }

        &.npm {
          background: #cb373520;
          color: #cb3735;
        }
      }
    }

    .package-meta {
      font-size: $font-size-sm;
      color: $text-muted;
      margin-top: 4px;

      .separator {
        margin: 0 $spacing-xs;
      }
    }
  }

  .package-actions {
    .btn-sm {
      padding: $spacing-xs $spacing-sm;
      font-size: $font-size-sm;
    }
  }
}

.usage-guide {
  margin-top: $spacing-xxl;
  padding: $spacing-lg;
  background: $bg-secondary;
  border-radius: $border-radius;

  h3 {
    margin-bottom: $spacing-lg;
    font-size: $font-size-lg;
  }

  .guide-section {
    margin-bottom: $spacing-lg;

    &:last-child {
      margin-bottom: 0;
    }

    h4 {
      margin-bottom: $spacing-md;
      font-size: $font-size-base;
    }
  }

  .code-block {
    margin-bottom: $spacing-md;
    border-radius: $border-radius;
    overflow: hidden;

    .code-header {
      background: darken($bg-secondary, 5%);
      padding: $spacing-xs $spacing-sm;
      font-size: $font-size-sm;
      color: $text-muted;
    }

    pre {
      margin: 0;
      padding: $spacing-md;
      background: $bg-primary;
      overflow-x: auto;

      code {
        font-family: 'Monaco', 'Menlo', monospace;
        font-size: $font-size-sm;
      }
    }
  }
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
