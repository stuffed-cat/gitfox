<template>
  <div class="package-detail">
    <!-- 面包屑 -->
    <div class="breadcrumb">
      <router-link :to="{ name: 'Packages' }" class="breadcrumb-link">
        <svg viewBox="0 0 16 16" width="12" height="12">
          <path d="M10 4L6 8l4 4" stroke="currentColor" stroke-width="1.5" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
        软件包仓库
      </router-link>
    </div>

    <!-- 加载中 -->
    <div v-if="loading" class="loading-state">
      <div class="loading-spinner"></div>
    </div>

    <!-- 包详情 -->
    <template v-else-if="pkg">
      <!-- 头部 -->
      <div class="detail-header">
        <div class="header-icon" :class="pkg.package_type">
          <svg v-if="pkg.package_type === 'docker'" viewBox="0 0 16 16" fill="none">
            <path d="M1.5 8h2v2h-2zM4.5 8h2v2h-2zM7.5 8h2v2h-2zM4.5 5h2v2h-2zM7.5 5h2v2h-2zM10.5 6c.5-1 1.5-1.5 3-1.5.3 1 .2 2-.5 3H1c0-3 1.5-5.5 4.5-5.5 1 0 2 .5 2.5 1.5h2c.5-1 1-1.5 2-1.5" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
          <svg v-else-if="pkg.package_type === 'npm'" viewBox="0 0 16 16" fill="none">
            <path d="M2 3h12v10H2V3zM5 6v4h2V7h1v3h3V6H5z" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
          <svg v-else-if="pkg.package_type === 'cargo'" viewBox="0 0 16 16" fill="none">
            <circle cx="8" cy="8" r="6.5" stroke="currentColor" fill="none" stroke-width="1.2"/>
            <circle cx="8" cy="8" r="2" fill="currentColor"/>
          </svg>
          <svg v-else viewBox="0 0 16 16" fill="none">
            <path d="M8 1L1 4v8l7 3 7-3V4L8 1zM8 8v7M1 4l7 4 7-4" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
        </div>
        <div class="header-info">
          <h1>{{ pkg.name }}</h1>
          <div class="header-meta">
            <span class="type-badge" :class="pkg.package_type">{{ pkg.package_type }}</span>
            <span class="separator">·</span>
            <span class="version">v{{ pkg.version }}</span>
            <span class="separator">·</span>
            <span class="date">{{ formatDate(pkg.created_at) }}</span>
          </div>
        </div>
        <div class="header-actions" v-if="canDelete">
          <button class="btn btn-danger btn-sm" @click="deletePackage">
            <svg viewBox="0 0 16 16" width="14" height="14">
              <path d="M3 4h10M6 4V2h4v2M5 4v9h6V4" stroke="currentColor" stroke-width="1.5" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
            删除
          </button>
        </div>
      </div>

      <!-- 安装命令卡片 -->
      <div class="install-card">
        <div class="card-header">
          <svg viewBox="0 0 16 16" width="14" height="14">
            <path d="M5 4L1 8l4 4M11 4l4 4-4 4M9 2l-2 12" stroke="currentColor" stroke-width="1.5" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
          <span>安装命令</span>
        </div>
        <div class="card-body">
          <code>{{ installCommand }}</code>
          <button class="copy-btn" @click="copyCommand" :class="{ copied }">
            <svg v-if="!copied" viewBox="0 0 16 16" width="14" height="14">
              <rect x="5" y="5" width="8" height="10" rx="1" stroke="currentColor" fill="none" stroke-width="1.5"/>
              <path d="M3 11V3a1 1 0 011-1h6" stroke="currentColor" stroke-width="1.5" fill="none"/>
            </svg>
            <svg v-else viewBox="0 0 16 16" width="14" height="14">
              <path d="M3 8l4 4 6-8" stroke="currentColor" stroke-width="2" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
            {{ copied ? '已复制' : '复制' }}
          </button>
        </div>
      </div>

      <!-- 版本历史 -->
      <details class="section-panel" open>
        <summary class="section-header">
          <svg viewBox="0 0 16 16" width="14" height="14" class="chevron">
            <path d="M6 4l4 4-4 4" stroke="currentColor" stroke-width="1.5" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
          版本历史
          <span class="count">{{ versions.length }}</span>
        </summary>
        <div class="section-content">
          <div class="version-table">
            <div 
              v-for="version in versions" 
              :key="version.id" 
              class="version-row"
              :class="{ current: version.version === pkg.version }"
            >
              <span class="version-tag">
                <svg viewBox="0 0 16 16" width="12" height="12">
                  <path d="M1 3l6-1 8 8-5 5-8-8zM5 5m-1 0a1 1 0 102 0a1 1 0 10-2 0" stroke="currentColor" stroke-width="1.5" fill="none"/>
                </svg>
                v{{ version.version }}
              </span>
              <span class="version-date">{{ formatDate(version.created_at) }}</span>
              <span class="version-size" v-if="version.size">{{ formatSize(version.size) }}</span>
            </div>
          </div>
        </div>
      </details>

      <!-- Docker 信息 -->
      <details v-if="pkg.package_type === 'docker'" class="section-panel" open>
        <summary class="section-header">
          <svg viewBox="0 0 16 16" width="14" height="14" class="chevron">
            <path d="M6 4l4 4-4 4" stroke="currentColor" stroke-width="1.5" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
          Docker 镜像信息
        </summary>
        <div class="section-content">
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

          <div class="usage-section">
            <h4>使用方法</h4>
            <div class="code-block">
              <div class="code-label">拉取镜像</div>
              <pre><code>docker pull {{ fullImageName }}</code></pre>
            </div>
            <div class="code-block">
              <div class="code-label">运行容器</div>
              <pre><code>docker run -it {{ fullImageName }}</code></pre>
            </div>
          </div>
        </div>
      </details>

      <!-- npm 信息 -->
      <details v-if="pkg.package_type === 'npm'" class="section-panel" open>
        <summary class="section-header">
          <svg viewBox="0 0 16 16" width="14" height="14" class="chevron">
            <path d="M6 4l4 4-4 4" stroke="currentColor" stroke-width="1.5" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
          npm 包信息
        </summary>
        <div class="section-content">
          <div class="info-grid" v-if="npmMetadata">
            <div class="info-item" v-if="npmMetadata.description">
              <label>描述</label>
              <span>{{ npmMetadata.description }}</span>
            </div>
            <div class="info-item" v-if="npmMetadata.license">
              <label>许可证</label>
              <span>{{ npmMetadata.license }}</span>
            </div>
            <div class="info-item" v-if="npmMetadata.homepage">
              <label>主页</label>
              <a :href="npmMetadata.homepage" target="_blank" class="external-link">{{ npmMetadata.homepage }}</a>
            </div>
            <div class="info-item" v-if="npmMetadata.repository">
              <label>仓库</label>
              <span>{{ npmMetadata.repository }}</span>
            </div>
          </div>

          <div class="usage-section">
            <h4>使用方法</h4>
            <div class="code-block">
              <div class="code-label">配置 .npmrc</div>
              <pre><code>@{{ namespace }}:registry=https://{{ registryDomain }}/npm/
//{{ registryDomain }}/npm/:_authToken=YOUR_TOKEN</code></pre>
            </div>
            <div class="code-block">
              <div class="code-label">安装</div>
              <pre><code>npm install {{ fullPackageName }}</code></pre>
            </div>
          </div>

          <div v-if="npmMetadata?.dependencies && Object.keys(npmMetadata.dependencies).length > 0" class="deps-section">
            <h4>依赖项</h4>
            <div class="deps-list">
              <span v-for="(ver, name) in npmMetadata.dependencies" :key="name" class="dep-tag">
                {{ name }} <code>{{ ver }}</code>
              </span>
            </div>
          </div>
        </div>
      </details>

      <!-- 文件列表 -->
      <details v-if="files.length > 0" class="section-panel" open>
        <summary class="section-header">
          <svg viewBox="0 0 16 16" width="14" height="14" class="chevron">
            <path d="M6 4l4 4-4 4" stroke="currentColor" stroke-width="1.5" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
          文件
          <span class="count">{{ files.length }}</span>
        </summary>
        <div class="section-content">
          <div class="files-table">
            <div v-for="file in files" :key="file.id" class="file-row">
              <svg viewBox="0 0 16 16" width="14" height="14" class="file-icon">
                <path d="M4 2a1 1 0 00-1 1v10a1 1 0 001 1h8a1 1 0 001-1V6l-4-4H4zM9 2v4h4" stroke="currentColor" stroke-width="1.5" fill="none"/>
              </svg>
              <span class="file-name">{{ file.file_name }}</span>
              <span class="file-size">{{ formatSize(file.size) }}</span>
              <a :href="file.download_url" class="download-link">
                <svg viewBox="0 0 16 16" width="14" height="14">
                  <path d="M8 2v8m-3-3l3 3 3-3M3 12h10" stroke="currentColor" stroke-width="1.5" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
                </svg>
              </a>
            </div>
          </div>
        </div>
      </details>
    </template>

    <!-- 404 -->
    <div v-else class="empty-state">
      <svg viewBox="0 0 48 48" width="48" height="48">
        <path d="M24 4L4 14v20l20 10 20-10V14L24 4z" stroke="currentColor" stroke-width="2" fill="none"/>
        <path d="M24 24v20M4 14l20 10 20-10" stroke="currentColor" stroke-width="2"/>
      </svg>
      <h3>软件包不存在</h3>
      <router-link :to="{ name: 'Packages' }" class="back-link">返回软件包列表</router-link>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import { api } from '@/api'
import type { NpmPackageInfo, CargoCrateInfo } from '@/api'
import { useAuthStore } from '@/stores/auth'
import type { Package, PackageFile, ProjectMember, PackageType } from '@/types'

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
const authStore = useAuthStore()

// 状态
const loading = ref(false)
const members = ref<ProjectMember[]>([])
const pkg = ref<Package | null>(null)
const versions = ref<Package[]>([])
const files = ref<PackageFile[]>([])
const dockerManifest = ref<DockerManifest | null>(null)
const npmMetadata = ref<NpmMetadata | null>(null)
const copied = ref(false)
const serverConfig = ref<{ registry_domain: string } | null>(null)

// 从项目或路由获取信息
// 路由是 /:pathSegments+，所以需要从数组中解析 namespace 和 projectName
const namespace = computed(() => {
  if (props.project?.namespace?.path) return props.project.namespace.path
  // 从 pathSegments 中解析：['gitfox', 'sdk', 'oa2', 'rust'] => 'gitfox/sdk/oa2'
  const segments = route.params.pathSegments
  if (Array.isArray(segments) && segments.length >= 2) {
    return segments.slice(0, -1).join('/')
  }
  return route.params.namespace as string
})
const projectName = computed(() => {
  if (props.project?.name) return props.project.name
  // 从 pathSegments 中解析：['gitfox', 'sdk', 'oa2', 'rust'] => 'rust'
  const segments = route.params.pathSegments
  if (Array.isArray(segments) && segments.length >= 1) {
    return segments[segments.length - 1]
  }
  return route.params.project as string
})
const packageType = computed(() => route.params.packageType as string)
const packageName = computed(() => {
  const name = route.params.packageName as string
  // 确保 URL 编码的包名被正确解码（如 %40gitfox%2Foa2-server → @gitfox/oa2-server）
  try {
    return decodeURIComponent(name)
  } catch {
    return name
  }
})

// 顶级命名空间（第一个 / 之前的部分，用于 Cargo registry）
// 例如：gitfox/sdk/oa2 → gitfox
const topLevelNamespace = computed(() => {
  const ns = namespace.value || ''
  return ns.split('/')[0]
})

// Registry 域名 - 从后端配置获取
const registryDomain = computed(() => {
  if (serverConfig.value?.registry_domain) {
    return serverConfig.value.registry_domain
  }
  // 降级到当前主机名
  return window.location.host
})

// 权限 - 检查当前用户是否为项目 owner、maintainer 或 developer
const canDelete = computed(() => {
  const currentUser = authStore.user
  if (!currentUser) return false
  
  // 检查是否为项目 owner
  if (props.project && String(props.project.id) === currentUser.id) return true
  
  // 检查项目成员角色 - owner, maintainer, developer 可以删除
  const member = members.value.find(m => String(m.user_id) === currentUser.id)
  if (member) {
    return member.role === 'owner' || member.role === 'maintainer' || member.role === 'developer'
  }
  
  return false
})

// 完整镜像名（Docker）
const fullImageName = computed(() => {
  if (!pkg.value || pkg.value.package_type !== 'docker') return ''
  return `${registryDomain.value}/${namespace.value}/${projectName.value}/${pkg.value.name}:${pkg.value.version}`
})

// 完整包名（npm）
const fullPackageName = computed(() => {
  if (!pkg.value || pkg.value.package_type !== 'npm') return ''
  // pkg.name 已经是完整的包名（如 @gitfox/oa2-server）
  return `${pkg.value.name}@${pkg.value.version}`
})

// 安装命令
const installCommand = computed(() => {
  if (!pkg.value) return ''
  if (pkg.value.package_type === 'docker') {
    return `docker pull ${fullImageName.value}`
  } else if (pkg.value.package_type === 'npm') {
    return `npm install ${fullPackageName.value}`
  } else if (pkg.value.package_type === 'cargo') {
    return `cargo add ${pkg.value.name}@${pkg.value.version} --registry ${namespace.value}`
  }
  return ''
})

// 加载服务器配置
async function loadServerConfig() {
  try {
    const config = await api.config.get()
    serverConfig.value = { registry_domain: config.registry_domain }
  } catch (error) {
    console.error('Failed to load server config:', error)
  }
}

// 加载包详情 - 使用 workhorse 的 registry API
async function loadPackage() {
  if (!namespace.value || !projectName.value || !packageType.value || !packageName.value) return
  
  loading.value = true
  try {
    const registryDomainValue = serverConfig.value?.registry_domain
    const type = packageType.value as PackageType
    
    if (type === 'npm') {
      // NPM: packageName 已经是完整包名（如 @gitfox/oa2-server）
      // 从包名中解析 scope 和 name
      let scope = ''
      let pkgName = packageName.value
      
      if (packageName.value.startsWith('@')) {
        const parts = packageName.value.slice(1).split('/')
        if (parts.length === 2) {
          scope = parts[0]
          pkgName = parts[1]
        }
      }
      
      // NPM: 使用 npm registry 标准 API
      const npmPkg: NpmPackageInfo = await api.registry.getNpmPackage(
        registryDomainValue,
        scope,
        pkgName
      )
      
      // 获取最新版本
      const latestVersion = npmPkg['dist-tags']?.latest || Object.keys(npmPkg.versions || {})[0] || ''
      const versionInfo = npmPkg.versions?.[latestVersion]
      
      // 转换为 Package 格式
      pkg.value = {
        id: 0, // npm 没有数字 ID
        project_id: props.project?.id || 0,
        name: packageName.value,
        version: latestVersion,
        package_type: 'npm',
        status: 'default',
        created_at: npmPkg.time?.[latestVersion] || new Date().toISOString(),
        updated_at: npmPkg.time?.[latestVersion] || new Date().toISOString(),
      }
      
      // 填充版本列表
      versions.value = Object.keys(npmPkg.versions || {}).map((ver, idx) => ({
        id: idx,
        project_id: props.project?.id || 0,
        name: packageName.value,
        version: ver,
        package_type: 'npm' as PackageType,
        status: 'default' as const,
        created_at: npmPkg.time?.[ver] || new Date().toISOString(),
        updated_at: npmPkg.time?.[ver] || new Date().toISOString(),
      }))
      
      // 填充 npm 元数据
      npmMetadata.value = {
        description: versionInfo?.description || npmPkg.description,
        license: versionInfo?.license || npmPkg.license,
        homepage: npmPkg.homepage,
        repository: typeof npmPkg.repository === 'string' ? npmPkg.repository : npmPkg.repository?.url,
        dependencies: versionInfo?.dependencies
      }
      
    } else if (type === 'cargo') {
      // Cargo: 使用 crates API（Cargo 使用顶级命名空间）
      const crateInfo: CargoCrateInfo = await api.registry.getCargoCrate(
        registryDomainValue,
        topLevelNamespace.value,
        packageName.value
      )
      
      // 转换为 Package 格式
      pkg.value = {
        id: 0,
        project_id: props.project?.id || 0,
        name: crateInfo.crate.name,
        version: crateInfo.crate.max_version,
        package_type: 'cargo',
        status: 'default',
        created_at: crateInfo.crate.created_at,
        updated_at: crateInfo.crate.updated_at,
      }
      
      // 填充版本列表
      versions.value = crateInfo.versions.map((ver, idx) => ({
        id: idx,
        project_id: props.project?.id || 0,
        name: crateInfo.crate.name,
        version: ver.num,
        package_type: 'cargo' as PackageType,
        status: ver.yanked ? 'hidden' as const : 'default' as const,
        created_at: ver.created_at,
        updated_at: ver.created_at,
        size: ver.crate_size,
      }))
      
      // Cargo 元数据填充到 npmMetadata（复用字段）
      npmMetadata.value = {
        description: crateInfo.crate.description,
        homepage: crateInfo.crate.homepage,
        repository: crateInfo.crate.repository,
      }
      
    } else if (type === 'docker') {
      // Docker: 使用 tags API
      const tagsResponse = await api.registry.getDockerTags(
        registryDomainValue,
        namespace.value,
        projectName.value,
        packageName.value
      )
      
      // Docker 只有 tags，选择第一个作为当前版本
      const currentTag = tagsResponse.tags?.[0] || 'latest'
      
      pkg.value = {
        id: 0,
        project_id: props.project?.id || 0,
        name: packageName.value,
        version: currentTag,
        package_type: 'docker',
        status: 'default',
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
      }
      
      // 每个 tag 作为一个版本
      versions.value = (tagsResponse.tags || []).map((tag, idx) => ({
        id: idx,
        project_id: props.project?.id || 0,
        name: packageName.value,
        version: tag,
        package_type: 'docker' as PackageType,
        status: 'default' as const,
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
      }))
    }
    
    // 加载项目成员用于权限检查
    try {
      members.value = await api.projects.getMembers({
        namespace: namespace.value,
        project: projectName.value
      })
    } catch { /* ignore - may not have permission to view members */ }
    
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

// 删除包 - 注意：registry API 不支持删除，这需要通过其他方式实现
async function deletePackage() {
  if (!pkg.value) return
  
  // Registry API 通常不支持直接删除，需要使用 yank（cargo）或 unpublish（npm）
  // 这里只显示警告
  alert('删除功能暂不可用。对于 Cargo，请使用 cargo yank；对于 npm，请使用 npm unpublish。')
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

onMounted(async () => {
  // 先加载配置，因为 loadPackage 依赖 serverConfig
  await loadServerConfig()
  await loadPackage()
})
</script>

<style lang="scss" scoped>
.package-detail {
  padding: 16px 20px;
  color: $text-primary;
}

.breadcrumb {
  margin-bottom: 16px;

  .breadcrumb-link {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 13px;
    color: $text-secondary;
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
  padding: 80px;

  .loading-spinner {
    width: 24px;
    height: 24px;
    border: 2px solid $border-color;
    border-top-color: $primary-color;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }
}

// 头部
.detail-header {
  display: flex;
  align-items: center;
  gap: 16px;
  padding-bottom: 20px;
  border-bottom: 1px solid $border-color;
  margin-bottom: 20px;

  .header-icon {
    width: 48px;
    height: 48px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 8px;
    background: $bg-tertiary;
    color: $text-secondary;

    svg {
      width: 24px;
      height: 24px;
    }

    &.docker {
      background: rgba(13, 183, 237, 0.08);
      color: #0db7ed;
    }

    &.npm {
      background: rgba(203, 55, 53, 0.08);
      color: #cb3735;
    }

    &.cargo {
      background: rgba(206, 65, 43, 0.08);
      color: #ce412b;
    }
  }

  .header-info {
    flex: 1;
    min-width: 0;

    h1 {
      margin: 0 0 6px 0;
      font-size: 20px;
      font-weight: 600;
      color: $text-primary;
    }

    .header-meta {
      display: flex;
      align-items: center;
      gap: 4px;
      font-size: 13px;
      color: $text-secondary;

      .separator {
        color: $text-tertiary;
      }

      .type-badge {
        padding: 2px 6px;
        border-radius: 3px;
        font-size: 11px;
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

        &.cargo {
          background: rgba(206, 65, 43, 0.1);
          color: #ce412b;
        }

        &.generic {
          background: $bg-tertiary;
          color: $text-secondary;
        }
      }
    }
  }

  .header-actions {
    .btn {
      display: inline-flex;
      align-items: center;
      gap: 6px;
      padding: 6px 12px;
      font-size: 13px;
      border: none;
      border-radius: 4px;
      cursor: pointer;

      &.btn-danger {
        background: #dc3545;
        color: white;

        &:hover {
          background: #c82333;
        }
      }
    }
  }
}

// 安装卡片
.install-card {
  background: $bg-secondary;
  border: 1px solid $border-color;
  border-radius: 6px;
  margin-bottom: 20px;

  .card-header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 14px;
    font-size: 13px;
    font-weight: 500;
    color: $text-primary;
    border-bottom: 1px solid $border-color;

    svg {
      color: $text-secondary;
    }
  }

  .card-body {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 14px;
    background: $bg-primary;
    border-radius: 0 0 6px 6px;

    code {
      flex: 1;
      font-family: 'Monaco', 'Menlo', 'Consolas', monospace;
      font-size: 13px;
      color: $text-primary;
    }

    .copy-btn {
      display: inline-flex;
      align-items: center;
      gap: 4px;
      padding: 6px 10px;
      font-size: 12px;
      background: $bg-tertiary;
      color: $text-secondary;
      border: 1px solid $border-color;
      border-radius: 4px;
      cursor: pointer;
      transition: all 0.15s;

      &:hover {
        background: $bg-secondary;
        color: $text-primary;
      }

      &.copied {
        background: rgba(40, 167, 69, 0.1);
        color: #28a745;
        border-color: rgba(40, 167, 69, 0.3);
      }
    }
  }
}

// 可折叠面板
.section-panel {
  border: 1px solid $border-color;
  border-radius: 6px;
  margin-bottom: 16px;
  background: $bg-primary;

  &[open] {
    .section-header .chevron {
      transform: rotate(90deg);
    }
  }

  .section-header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 14px;
    font-size: 14px;
    font-weight: 500;
    color: $text-primary;
    cursor: pointer;
    user-select: none;
    list-style: none;

    &::-webkit-details-marker {
      display: none;
    }

    &:hover {
      background: $bg-secondary;
    }

    .chevron {
      transition: transform 0.15s;
      color: $text-tertiary;
    }

    .count {
      margin-left: auto;
      padding: 2px 8px;
      font-size: 12px;
      font-weight: normal;
      background: $bg-tertiary;
      color: $text-secondary;
      border-radius: 10px;
    }
  }

  .section-content {
    border-top: 1px solid $border-color;
    padding: 14px;
  }
}

// 版本表格
.version-table {
  display: flex;
  flex-direction: column;
  gap: 1px;
  background: $border-color;
  border-radius: 4px;
  overflow: hidden;
}

.version-row {
  display: grid;
  grid-template-columns: 1fr auto auto;
  gap: 12px;
  align-items: center;
  padding: 10px 12px;
  background: $bg-primary;
  font-size: 13px;

  &.current {
    background: rgba($primary-color, 0.04);
  }

  .version-tag {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-weight: 500;
    color: $text-primary;

    svg {
      color: $text-tertiary;
    }
  }

  .version-date {
    color: $text-secondary;
  }

  .version-size {
    color: $text-tertiary;
    font-size: 12px;
  }
}

// 信息网格
.info-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
  gap: 12px;
}

.info-item {
  label {
    display: block;
    font-size: 11px;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: $text-tertiary;
    margin-bottom: 4px;
  }

  span, code {
    font-size: 13px;
    color: $text-primary;
  }

  code {
    font-family: 'Monaco', 'Menlo', monospace;
    word-break: break-all;
  }

  .external-link {
    color: $primary-color;
    text-decoration: none;

    &:hover {
      text-decoration: underline;
    }
  }
}

// 使用方法
.usage-section {
  margin-top: 16px;
  padding-top: 16px;
  border-top: 1px solid $border-color;

  h4 {
    margin: 0 0 12px 0;
    font-size: 13px;
    font-weight: 500;
    color: $text-primary;
  }
}

.code-block {
  margin-bottom: 12px;
  border: 1px solid $border-color;
  border-radius: 4px;
  overflow: hidden;

  &:last-child {
    margin-bottom: 0;
  }

  .code-label {
    padding: 6px 10px;
    font-size: 11px;
    font-weight: 500;
    color: $text-secondary;
    background: $bg-tertiary;
    border-bottom: 1px solid $border-color;
  }

  pre {
    margin: 0;
    padding: 10px 12px;
    background: $bg-primary;
    overflow-x: auto;

    code {
      font-family: 'Monaco', 'Menlo', monospace;
      font-size: 12px;
      color: $text-primary;
    }
  }
}

// 依赖项
.deps-section {
  margin-top: 16px;
  padding-top: 16px;
  border-top: 1px solid $border-color;

  h4 {
    margin: 0 0 10px 0;
    font-size: 13px;
    font-weight: 500;
    color: $text-primary;
  }
}

.deps-list {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.dep-tag {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 4px 8px;
  font-size: 12px;
  background: $bg-tertiary;
  color: $text-primary;
  border-radius: 3px;

  code {
    font-family: 'Monaco', 'Menlo', monospace;
    font-size: 11px;
    color: $text-secondary;
  }
}

// 文件表格
.files-table {
  display: flex;
  flex-direction: column;
  gap: 1px;
  background: $border-color;
  border-radius: 4px;
  overflow: hidden;
}

.file-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  background: $bg-primary;
  font-size: 13px;

  .file-icon {
    color: $text-tertiary;
    flex-shrink: 0;
  }

  .file-name {
    flex: 1;
    min-width: 0;
    font-family: 'Monaco', 'Menlo', monospace;
    color: $text-primary;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .file-size {
    color: $text-secondary;
    font-size: 12px;
    flex-shrink: 0;
  }

  .download-link {
    color: $text-secondary;
    padding: 4px;
    border-radius: 4px;
    transition: all 0.15s;

    &:hover {
      background: $bg-tertiary;
      color: $primary-color;
    }
  }
}

// 空状态
.empty-state {
  text-align: center;
  padding: 60px 20px;
  color: $text-secondary;

  svg {
    margin-bottom: 16px;
    opacity: 0.4;
  }

  h3 {
    margin: 0 0 12px 0;
    font-size: 16px;
    font-weight: 500;
    color: $text-primary;
  }

  .back-link {
    color: $primary-color;
    text-decoration: none;
    font-size: 14px;

    &:hover {
      text-decoration: underline;
    }
  }
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
