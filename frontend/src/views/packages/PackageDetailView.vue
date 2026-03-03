<template>
  <div class="package-detail-view">
    <!-- 返回链接 -->
    <div class="breadcrumb">
      <router-link :to="{ name: 'Packages' }" class="back-link">
        ← 返回软件包列表
      </router-link>
    </div>

    <!-- 加载中 -->
    <div v-if="loading" class="loading-state">
      <div class="spinner"></div>
      <span>加载中...</span>
    </div>

    <!-- 包详情 -->
    <template v-else-if="pkg">
      <div class="package-header">
        <div class="package-icon">
          <span v-if="pkg.package_type === 'docker'">🐳</span>
          <span v-else-if="pkg.package_type === 'npm'">📦</span>
          <span v-else>📁</span>
        </div>
        <div class="package-title">
          <h1>{{ pkg.name }}</h1>
          <div class="package-meta">
            <span class="package-type-badge" :class="pkg.package_type">
              {{ pkg.package_type }}
            </span>
            <span class="separator">·</span>
            <span class="version">v{{ pkg.version }}</span>
            <span class="separator">·</span>
            <span class="date">发布于 {{ formatDate(pkg.created_at) }}</span>
          </div>
        </div>
        <div class="package-actions">
          <button class="btn btn-danger" @click="deletePackage" v-if="canDelete">
            删除
          </button>
        </div>
      </div>

      <!-- 安装命令 -->
      <div class="install-section">
        <h3>安装</h3>
        <div class="install-command">
          <code>{{ installCommand }}</code>
          <button class="copy-btn" @click="copyCommand">
            {{ copied ? '已复制' : '复制' }}
          </button>
        </div>
      </div>

      <!-- 版本列表 -->
      <div class="versions-section">
        <h3>版本历史</h3>
        <div class="version-list">
          <div 
            v-for="version in versions" 
            :key="version.id" 
            class="version-item"
            :class="{ current: version.version === pkg.version }"
          >
            <div class="version-info">
              <span class="version-tag">v{{ version.version }}</span>
              <span class="version-date">{{ formatDate(version.created_at) }}</span>
            </div>
            <div class="version-meta">
              <span v-if="version.size">{{ formatSize(version.size) }}</span>
            </div>
          </div>
        </div>
      </div>

      <!-- Docker 特有信息 -->
      <div v-if="pkg.package_type === 'docker'" class="docker-section">
        <h3>Docker 镜像信息</h3>
        <div class="info-grid">
          <div class="info-item">
            <label>镜像 Digest</label>
            <code>{{ dockerManifest?.digest || 'N/A' }}</code>
          </div>
          <div class="info-item">
            <label>架构</label>
            <span>{{ dockerManifest?.architecture || 'N/A' }}</span>
          </div>
          <div class="info-item">
            <label>OS</label>
            <span>{{ dockerManifest?.os || 'N/A' }}</span>
          </div>
          <div class="info-item">
            <label>层数</label>
            <span>{{ dockerManifest?.layers?.length || 'N/A' }}</span>
          </div>
        </div>

        <h4>使用方法</h4>
        <div class="code-blocks">
          <div class="code-block">
            <div class="code-header">拉取镜像</div>
            <pre><code>docker pull {{ fullImageName }}</code></pre>
          </div>
          <div class="code-block">
            <div class="code-header">运行容器</div>
            <pre><code>docker run -it {{ fullImageName }}</code></pre>
          </div>
        </div>
      </div>

      <!-- npm 特有信息 -->
      <div v-if="pkg.package_type === 'npm'" class="npm-section">
        <h3>npm 包信息</h3>
        <div class="info-grid">
          <div class="info-item" v-if="npmMetadata?.description">
            <label>描述</label>
            <span>{{ npmMetadata.description }}</span>
          </div>
          <div class="info-item" v-if="npmMetadata?.license">
            <label>许可证</label>
            <span>{{ npmMetadata.license }}</span>
          </div>
          <div class="info-item" v-if="npmMetadata?.homepage">
            <label>主页</label>
            <a :href="npmMetadata.homepage" target="_blank">{{ npmMetadata.homepage }}</a>
          </div>
          <div class="info-item" v-if="npmMetadata?.repository">
            <label>仓库</label>
            <span>{{ npmMetadata.repository }}</span>
          </div>
        </div>

        <h4>使用方法</h4>
        <div class="code-blocks">
          <div class="code-block">
            <div class="code-header">配置 .npmrc</div>
            <pre><code>@{{ namespace }}:registry=https://{{ registryDomain }}/npm/
//{{ registryDomain }}/npm/:_authToken=YOUR_TOKEN</code></pre>
          </div>
          <div class="code-block">
            <div class="code-header">安装</div>
            <pre><code>npm install {{ fullPackageName }}</code></pre>
          </div>
        </div>

        <!-- 依赖 -->
        <div v-if="npmMetadata?.dependencies" class="dependencies-section">
          <h4>依赖</h4>
          <div class="dependency-list">
            <div 
              v-for="(version, name) in npmMetadata.dependencies" 
              :key="name"
              class="dependency-item"
            >
              <span class="dep-name">{{ name }}</span>
              <span class="dep-version">{{ version }}</span>
            </div>
          </div>
        </div>
      </div>

      <!-- 文件列表 -->
      <div class="files-section" v-if="files.length > 0">
        <h3>文件</h3>
        <div class="file-list">
          <div v-for="file in files" :key="file.id" class="file-item">
            <span class="file-name">{{ file.file_name }}</span>
            <span class="file-size">{{ formatSize(file.size) }}</span>
            <a :href="file.download_url" class="download-btn">下载</a>
          </div>
        </div>
      </div>
    </template>

    <!-- 404 -->
    <div v-else class="empty-state">
      <h3>软件包不存在</h3>
      <router-link :to="{ name: 'Packages' }">返回列表</router-link>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { api } from '@/api'
import type { Package, PackageFile } from '@/types'

interface DockerManifest {
  digest: string
  architecture?: string
  os?: string
  layers?: { digest: string; size: number }[]
}

interface NpmMetadata {
  description?: string
  license?: string
  homepage?: string
  repository?: string
  dependencies?: Record<string, string>
}

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
const pkg = ref<Package | null>(null)
const versions = ref<Package[]>([])
const files = ref<PackageFile[]>([])
const dockerManifest = ref<DockerManifest | null>(null)
const npmMetadata = ref<NpmMetadata | null>(null)
const copied = ref(false)

// 从项目或路由获取信息
const namespace = computed(() => props.project?.namespace?.path || route.params.namespace as string)
const projectName = computed(() => props.project?.name || route.params.project as string)
const packageId = computed(() => route.params.packageId as string)

// Registry 域名
const registryDomain = computed(() => {
  return window.location.hostname.replace(/^[^.]+/, 'registry')
})

// 权限
const canDelete = computed(() => {
  // TODO: 检查用户权限
  return true
})

// 完整镜像名（Docker）
const fullImageName = computed(() => {
  if (!pkg.value || pkg.value.package_type !== 'docker') return ''
  return `${registryDomain.value}/${namespace.value}/${projectName.value}/${pkg.value.name}:${pkg.value.version}`
})

// 完整包名（npm）
const fullPackageName = computed(() => {
  if (!pkg.value || pkg.value.package_type !== 'npm') return ''
  return `@${namespace.value}/${pkg.value.name}@${pkg.value.version}`
})

// 安装命令
const installCommand = computed(() => {
  if (!pkg.value) return ''
  if (pkg.value.package_type === 'docker') {
    return `docker pull ${fullImageName.value}`
  } else if (pkg.value.package_type === 'npm') {
    return `npm install ${fullPackageName.value}`
  }
  return ''
})

// 加载包详情
async function loadPackage() {
  if (!namespace.value || !projectName.value || !packageId.value) return
  
  loading.value = true
  try {
    // 加载包基本信息
    const pkgResponse = await api.packages.get(namespace.value, projectName.value, packageId.value)
    pkg.value = pkgResponse.data

    // 加载版本列表
    if (pkg.value) {
      const versionsResponse = await api.packages.listVersions(
        namespace.value, 
        projectName.value, 
        pkg.value.name
      )
      versions.value = versionsResponse.data || []

      // 加载文件列表
      const filesResponse = await api.packages.listFiles(
        namespace.value,
        projectName.value,
        packageId.value
      )
      files.value = filesResponse.data || []

      // 加载特定类型的元数据
      if (pkg.value.package_type === 'docker') {
        // TODO: 加载 Docker manifest
      } else if (pkg.value.package_type === 'npm') {
        // TODO: 加载 npm 元数据
      }
    }
  } catch (error) {
    console.error('Failed to load package:', error)
    pkg.value = null
  } finally {
    loading.value = false
  }
}

// 复制命令
function copyCommand() {
  navigator.clipboard.writeText(installCommand.value)
  copied.value = true
  setTimeout(() => {
    copied.value = false
  }, 2000)
}

// 删除包
async function deletePackage() {
  if (!pkg.value) return
  
  if (!confirm(`确定要删除软件包 ${pkg.value.name} v${pkg.value.version} 吗？此操作不可撤销。`)) {
    return
  }

  try {
    await api.packages.delete(namespace.value, projectName.value, packageId.value)
    router.push({ name: 'Packages' })
  } catch (error) {
    console.error('Failed to delete package:', error)
    alert('删除失败')
  }
}

// 格式化日期
function formatDate(dateStr: string): string {
  const date = new Date(dateStr)
  return date.toLocaleDateString('zh-CN', {
    year: 'numeric',
    month: 'long',
    day: 'numeric'
  })
}

// 格式化文件大小
function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`
  return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`
}

onMounted(() => {
  loadPackage()
})
</script>

<style lang="scss" scoped>
.package-detail-view {
  padding: $spacing-lg;
}

.breadcrumb {
  margin-bottom: $spacing-lg;

  .back-link {
    color: $text-muted;
    text-decoration: none;

    &:hover {
      color: $primary-color;
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

.package-header {
  display: flex;
  align-items: flex-start;
  gap: $spacing-lg;
  margin-bottom: $spacing-xl;

  .package-icon {
    font-size: 48px;
    width: 80px;
    height: 80px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: $bg-secondary;
    border-radius: $border-radius;
  }

  .package-title {
    flex: 1;

    h1 {
      margin: 0 0 $spacing-sm 0;
      font-size: $font-size-xxl;
    }

    .package-meta {
      display: flex;
      align-items: center;
      gap: $spacing-xs;
      color: $text-muted;

      .separator {
        margin: 0 $spacing-xs;
      }

      .package-type-badge {
        padding: 2px 8px;
        border-radius: 4px;
        font-size: $font-size-sm;

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
  }
}

.install-section {
  margin-bottom: $spacing-xl;
  padding: $spacing-lg;
  background: $bg-secondary;
  border-radius: $border-radius;

  h3 {
    margin: 0 0 $spacing-md 0;
    font-size: $font-size-lg;
  }

  .install-command {
    display: flex;
    align-items: center;
    gap: $spacing-md;
    padding: $spacing-md;
    background: $bg-primary;
    border-radius: $border-radius;

    code {
      flex: 1;
      font-family: 'Monaco', 'Menlo', monospace;
      font-size: $font-size-base;
    }

    .copy-btn {
      padding: $spacing-xs $spacing-md;
      background: $primary-color;
      color: white;
      border: none;
      border-radius: $border-radius;
      cursor: pointer;

      &:hover {
        background: darken($primary-color, 10%);
      }
    }
  }
}

.versions-section,
.docker-section,
.npm-section,
.files-section {
  margin-bottom: $spacing-xl;

  h3 {
    font-size: $font-size-lg;
    margin-bottom: $spacing-md;
  }

  h4 {
    font-size: $font-size-base;
    margin: $spacing-lg 0 $spacing-md 0;
  }
}

.version-list {
  display: flex;
  flex-direction: column;
  gap: $spacing-xs;
}

.version-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: $spacing-md;
  background: $bg-primary;
  border: 1px solid $border-color;
  border-radius: $border-radius;

  &.current {
    border-color: $primary-color;
    background: lighten($primary-color, 45%);
  }

  .version-info {
    display: flex;
    align-items: center;
    gap: $spacing-md;

    .version-tag {
      font-weight: 500;
    }

    .version-date {
      color: $text-muted;
      font-size: $font-size-sm;
    }
  }

  .version-meta {
    color: $text-muted;
    font-size: $font-size-sm;
  }
}

.info-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
  gap: $spacing-md;

  .info-item {
    padding: $spacing-md;
    background: $bg-secondary;
    border-radius: $border-radius;

    label {
      display: block;
      font-size: $font-size-sm;
      color: $text-muted;
      margin-bottom: $spacing-xs;
    }

    code {
      font-family: 'Monaco', 'Menlo', monospace;
      font-size: $font-size-sm;
      word-break: break-all;
    }
  }
}

.code-blocks {
  display: flex;
  flex-direction: column;
  gap: $spacing-md;
}

.code-block {
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

.dependencies-section {
  .dependency-list {
    display: flex;
    flex-wrap: wrap;
    gap: $spacing-sm;
  }

  .dependency-item {
    display: flex;
    align-items: center;
    gap: $spacing-xs;
    padding: $spacing-xs $spacing-sm;
    background: $bg-secondary;
    border-radius: $border-radius;
    font-size: $font-size-sm;

    .dep-name {
      font-weight: 500;
    }

    .dep-version {
      color: $text-muted;
    }
  }
}

.file-list {
  display: flex;
  flex-direction: column;
  gap: $spacing-xs;
}

.file-item {
  display: flex;
  align-items: center;
  gap: $spacing-md;
  padding: $spacing-md;
  background: $bg-primary;
  border: 1px solid $border-color;
  border-radius: $border-radius;

  .file-name {
    flex: 1;
    font-family: 'Monaco', 'Menlo', monospace;
    font-size: $font-size-sm;
  }

  .file-size {
    color: $text-muted;
    font-size: $font-size-sm;
  }

  .download-btn {
    padding: $spacing-xs $spacing-sm;
    background: $primary-color;
    color: white;
    text-decoration: none;
    border-radius: $border-radius;
    font-size: $font-size-sm;

    &:hover {
      background: darken($primary-color, 10%);
    }
  }
}

.empty-state {
  text-align: center;
  padding: $spacing-xxl;

  h3 {
    margin-bottom: $spacing-md;
  }
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
