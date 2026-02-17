<template>
  <div class="search-page">
    <div class="search-content">
      <!-- 搜索过滤器 -->
      <div class="search-filters">
        <div class="filter-tabs">
          <button
            v-for="tab in searchTabs"
            :key="tab.key"
            class="filter-tab"
            :class="{ active: activeTab === tab.key }"
            @click="activeTab = tab.key; performSearch()"
          >
            {{ tab.label }}
            <span v-if="resultCounts[tab.key] > 0" class="count">{{ resultCounts[tab.key] }}</span>
          </button>
        </div>
      </div>

      <!-- 搜索结果 -->
      <div class="search-results">
        <!-- 加载状态 -->
        <div v-if="loading" class="loading-state">
          <div class="spinner"></div>
          <p>搜索中...</p>
        </div>

        <!-- 空状态 - 未搜索 -->
        <div v-else-if="!hasSearched" class="empty-state">
          <svg width="64" height="64" viewBox="0 0 64 64" fill="none">
            <circle cx="26" cy="26" r="20" stroke="currentColor" stroke-width="2"/>
            <path d="M42 42l14 14" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
          </svg>
          <h3>开始搜索</h3>
          <p>输入关键词搜索项目、群组、用户等内容</p>
        </div>

        <!-- 空状态 - 无结果 -->
        <div v-else-if="!loading && noResults" class="empty-state">
          <svg width="64" height="64" viewBox="0 0 64 64" fill="none">
            <circle cx="26" cy="26" r="20" stroke="currentColor" stroke-width="2"/>
            <path d="M42 42l14 14" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
            <path d="M20 26h12" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
          </svg>
          <h3>未找到结果</h3>
          <p>尝试使用不同的关键词或过滤条件</p>
        </div>

        <!-- 项目结果 -->
        <div v-if="(activeTab === 'all' || activeTab === 'projects') && results.projects.length > 0" class="results-section">
          <h3 v-if="activeTab === 'all'" class="section-title">
            项目 ({{ results.projects.length }})
          </h3>
          <div class="result-list">
            <router-link
              v-for="project in results.projects"
              :key="project.id"
              :to="`/${project.owner_name}/${project.name}`"
              class="result-item project-item"
            >
              <div class="item-icon">
                <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
                  <rect x="3" y="3" width="14" height="14" rx="2" stroke="currentColor" stroke-width="1.5"/>
                </svg>
              </div>
              <div class="item-content">
                <div class="item-title">{{ project.owner_name }} / {{ project.name }}</div>
                <div class="item-meta">
                  <span class="visibility-badge" :class="project.visibility">
                    {{ visibilityLabel(project.visibility) }}
                  </span>
                  <span v-if="project.description" class="description">{{ project.description }}</span>
                </div>
              </div>
            </router-link>
          </div>
        </div>

        <!-- 群组结果 -->
        <div v-if="(activeTab === 'all' || activeTab === 'groups') && results.groups.length > 0" class="results-section">
          <h3 v-if="activeTab === 'all'" class="section-title">
            群组 ({{ results.groups.length }})
          </h3>
          <div class="result-list">
            <router-link
              v-for="group in results.groups"
              :key="group.id"
              :to="`/${group.path}`"
              class="result-item group-item"
            >
              <div class="item-icon">
                <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
                  <rect x="2" y="3" width="7" height="7" rx="1" stroke="currentColor" stroke-width="1.5"/>
                  <rect x="11" y="3" width="7" height="7" rx="1" stroke="currentColor" stroke-width="1.5"/>
                  <rect x="6.5" y="10" width="7" height="7" rx="1" stroke="currentColor" stroke-width="1.5"/>
                </svg>
              </div>
              <div class="item-content">
                <div class="item-title">{{ group.name }}</div>
                <div class="item-meta">
                  <span class="path">{{ group.path }}</span>
                  <span v-if="group.description" class="description">{{ group.description }}</span>
                </div>
              </div>
            </router-link>
          </div>
        </div>

        <!-- 用户结果 -->
        <div v-if="(activeTab === 'all' || activeTab === 'users') && results.users.length > 0" class="results-section">
          <h3 v-if="activeTab === 'all'" class="section-title">
            用户 ({{ results.users.length }})
          </h3>
          <div class="result-list">
            <router-link
              v-for="user in results.users"
              :key="user.id"
              :to="`/${user.username}`"
              class="result-item user-item"
            >
              <div class="item-avatar">
                <img v-if="user.avatar_url" :src="user.avatar_url" :alt="user.username" />
                <span v-else>{{ user.username.charAt(0).toUpperCase() }}</span>
              </div>
              <div class="item-content">
                <div class="item-title">{{ user.display_name || user.username }}</div>
                <div class="item-meta">
                  <span class="username">@{{ user.username }}</span>
                </div>
              </div>
            </router-link>
          </div>
        </div>

        <!-- Issue结果 -->
        <div v-if="(activeTab === 'all' || activeTab === 'issues') && results.issues.length > 0" class="results-section">
          <h3 v-if="activeTab === 'all'" class="section-title">
            Issue ({{ results.issues.length }})
          </h3>
          <div class="result-list">
            <router-link
              v-for="issue in results.issues"
              :key="issue.id"
              :to="`/${issue.namespace_path}/${issue.project_name}/-/issues/${issue.iid}`"
              class="result-item issue-item"
            >
              <div class="item-icon">
                <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
                  <circle cx="10" cy="10" r="7" stroke="currentColor" stroke-width="1.5"/>
                  <circle cx="10" cy="13" r="0.5" fill="currentColor"/>
                  <path d="M10 7v4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
                </svg>
              </div>
              <div class="item-content">
                <div class="item-title">{{ issue.title }}</div>
                <div class="item-meta">
                  <span class="issue-ref">#{{ issue.iid }}</span>
                  <span class="project-path">{{ issue.namespace_path }}/{{ issue.project_name }}</span>
                  <span class="state-badge" :class="issue.state">{{ issue.state === 'open' ? '开放' : '已关闭' }}</span>
                </div>
              </div>
            </router-link>
          </div>
        </div>

        <!-- MR结果 -->
        <div v-if="(activeTab === 'all' || activeTab === 'merge_requests') && results.merge_requests.length > 0" class="results-section">
          <h3 v-if="activeTab === 'all'" class="section-title">
            合并请求 ({{ results.merge_requests.length }})
          </h3>
          <div class="result-list">
            <router-link
              v-for="mr in results.merge_requests"
              :key="mr.id"
              :to="`/${mr.namespace_path}/${mr.project_name}/-/merge_requests/${mr.iid}`"
              class="result-item mr-item"
            >
              <div class="item-icon">
                <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
                  <circle cx="5" cy="5" r="2" stroke="currentColor" stroke-width="1.5"/>
                  <circle cx="15" cy="15" r="2" stroke="currentColor" stroke-width="1.5"/>
                  <path d="M5 7v6a2 2 0 002 2h6" stroke="currentColor" stroke-width="1.5"/>
                </svg>
              </div>
              <div class="item-content">
                <div class="item-title">{{ mr.title }}</div>
                <div class="item-meta">
                  <span class="mr-ref">!{{ mr.iid }}</span>
                  <span class="project-path">{{ mr.namespace_path }}/{{ mr.project_name }}</span>
                  <span class="state-badge" :class="mr.status">{{ mr.status === 'open' ? '开放' : '已合并' }}</span>
                </div>
              </div>
            </router-link>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import api from '@/api'
import type { Project, Group } from '@/types'

interface SearchUser {
  id: string
  username: string
  display_name?: string
  avatar_url?: string
}

interface SearchIssue {
  id: string
  project_id: string
  project_name: string
  namespace_path: string
  iid: number
  title: string
  description?: string
  state: string
  author_id: string
  author_username: string
  author_avatar?: string
  created_at: string
  updated_at: string
}

interface SearchMR {
  id: string
  project_id: string
  project_name: string
  namespace_path: string
  iid: number
  title: string
  description?: string
  status: string
  source_branch: string
  target_branch: string
  author_id: string
  author_username: string
  author_avatar?: string
  created_at: string
  updated_at: string
}

interface SearchResults {
  projects: Project[]
  groups: Group[]
  users: SearchUser[]
  issues: SearchIssue[]
  merge_requests: SearchMR[]
}

const route = useRoute()
const router = useRouter()

const searchQuery = ref('')
const activeTab = ref<SearchTab>('all')
const loading = ref(false)
const hasSearched = ref(false)

type SearchTab = 'all' | 'projects' | 'groups' | 'users' | 'issues' | 'merge_requests'

const searchTabs: Array<{ key: SearchTab; label: string }> = [
  { key: 'all', label: '全部' },
  { key: 'projects', label: '项目' },
  { key: 'groups', label: '群组' },
  { key: 'users', label: '用户' },
  { key: 'issues', label: 'Issue' },
  { key: 'merge_requests', label: '合并请求' }
]

const results = ref<SearchResults>({
  projects: [],
  groups: [],
  users: [],
  issues: [],
  merge_requests: []
})

const resultCounts = computed(() => ({
  all: results.value.projects.length + 
       results.value.groups.length + 
       results.value.users.length +
       results.value.issues.length +
       results.value.merge_requests.length,
  projects: results.value.projects.length,
  groups: results.value.groups.length,
  users: results.value.users.length,
  issues: results.value.issues.length,
  merge_requests: results.value.merge_requests.length
}))

const noResults = computed(() => resultCounts.value.all === 0)

async function performSearch() {
  if (!searchQuery.value.trim()) return

  loading.value = true
  hasSearched.value = true

  // 更新URL参数
  router.replace({
    query: {
      q: searchQuery.value,
      scope: activeTab.value !== 'all' ? activeTab.value : undefined
    }
  })

  try {
    // 使用新的search API
    const searchResults = await api.search.all(searchQuery.value, {
      scope: activeTab.value
    })

    results.value = searchResults

  } catch (error) {
    console.error('Search failed:', error)
    // 清空结果
    results.value = {
      projects: [],
      groups: [],
      users: [],
      issues: [],
      merge_requests: []
    }
  } finally {
    loading.value = false
  }
}

function visibilityLabel(v: string): string {
  const labels: Record<string, string> = {
    public: '公开',
    internal: '内部',
    private: '私有'
  }
  return labels[v] || v
}

// 从URL初始化搜索
onMounted(() => {
  if (route.query.q) {
    searchQuery.value = route.query.q as string
    const scope = route.query.scope as string
    activeTab.value = (scope && ['all', 'projects', 'groups', 'users', 'issues', 'merge_requests'].includes(scope)) 
      ? scope as SearchTab
      : 'all'
    performSearch()
  }
})

// 监听路由变化
watch(() => route.query, (newQuery) => {
  if (newQuery.q && newQuery.q !== searchQuery.value) {
    searchQuery.value = newQuery.q as string
    const scope = newQuery.scope as string
    activeTab.value = (scope && ['all', 'projects', 'groups', 'users', 'issues', 'merge_requests'].includes(scope))
      ? scope as SearchTab
      : 'all'
    performSearch()
  }
})
</script>

<style lang="scss" scoped>
@import '@/styles/variables.scss';

.search-page {
  min-height: 100vh;
  background: $bg-primary;
}

.search-content {
  max-width: 1200px;
  margin: 0 auto;
  padding: $spacing-6 $spacing-4;
}

.search-filters {
  margin-bottom: $spacing-6;
}

.filter-tabs {
  display: flex;
  gap: $spacing-2;
  border-bottom: 2px solid $border-color;
}

.filter-tab {
  padding: $spacing-3 $spacing-4;
  border: none;
  background: transparent;
  color: $text-secondary;
  font-weight: 500;
  cursor: pointer;
  border-bottom: 2px solid transparent;
  margin-bottom: -2px;
  display: flex;
  align-items: center;
  gap: $spacing-2;
  transition: all 0.2s;

  &:hover {
    color: $text-primary;
    background: $bg-secondary;
  }

  &.active {
    color: $color-primary;
    border-bottom-color: $color-primary;
  }

  .count {
    background: $bg-secondary;
    color: $text-muted;
    padding: 2px 8px;
    border-radius: $radius-full;
    font-size: $text-xs;
    font-weight: 600;
  }

  &.active .count {
    background: $color-primary;
    color: white;
  }
}

.search-results {
  min-height: 400px;
}

.loading-state,
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: $spacing-8 $spacing-4;
  color: $text-muted;

  svg {
    margin-bottom: $spacing-4;
    opacity: 0.5;
  }

  h3 {
    font-size: $text-xl;
    font-weight: 600;
    color: $text-secondary;
    margin-bottom: $spacing-2;
  }

  p {
    font-size: $text-sm;
  }
}

.spinner {
  width: 40px;
  height: 40px;
  border: 3px solid $border-color;
  border-top-color: $color-primary;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
  margin-bottom: $spacing-4;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.results-section {
  margin-bottom: $spacing-6;

  &:last-child {
    margin-bottom: 0;
  }
}

.section-title {
  font-size: $text-lg;
  font-weight: 600;
  color: $text-primary;
  margin-bottom: $spacing-4;
  padding-bottom: $spacing-2;
  border-bottom: 1px solid $border-color;
}

.result-list {
  display: flex;
  flex-direction: column;
  gap: $spacing-2;
}

.result-item {
  display: flex;
  align-items: flex-start;
  gap: $spacing-3;
  padding: $spacing-4;
  background: white;
  border: 1px solid $border-color;
  border-radius: $radius-md;
  text-decoration: none;
  color: $text-primary;
  transition: all 0.2s;

  &:hover {
    border-color: $color-primary;
    box-shadow: $shadow-sm;
    transform: translateY(-1px);
  }

  .item-icon {
    width: 40px;
    height: 40px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: $bg-secondary;
    border-radius: $radius-md;
    color: $text-muted;
    flex-shrink: 0;
  }

  .item-avatar {
    width: 40px;
    height: 40px;
    border-radius: 50%;
    overflow: hidden;
    background: $color-primary;
    color: white;
    display: flex;
    align-items: center;
    justify-content: center;
    font-weight: 600;
    flex-shrink: 0;

    img {
      width: 100%;
      height: 100%;
      object-fit: cover;
    }
  }

  .item-content {
    flex: 1;
    min-width: 0;
  }

  .item-title {
    font-size: $text-base;
    font-weight: 600;
    color: $text-primary;
    margin-bottom: $spacing-1;
  }

  .item-meta {
    display: flex;
    align-items: center;
    gap: $spacing-2;
    font-size: $text-sm;
    color: $text-muted;
    flex-wrap: wrap;
  }

  .visibility-badge,
  .state-badge {
    padding: 2px 8px;
    border-radius: $radius-sm;
    font-size: $text-xs;
    font-weight: 500;

    &.public {
      background: #e6f4ea;
      color: #137333;
    }

    &.internal {
      background: #fef7e0;
      color: #b95000;
    }

    &.private {
      background: #fce8e6;
      color: #c5221f;
    }

    &.open {
      background: #e6f4ea;
      color: #137333;
    }

    &.closed {
      background: #e8eaed;
      color: #5f6368;
    }

    &.merged {
      background: #e4e7ff;
      color: #2d3e9e;
    }
  }

  .description {
    color: $text-secondary;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .path,
  .username {
    color: $text-muted;
  }

  .issue-ref,
  .mr-ref {
    font-weight: 600;
    color: $text-secondary;
  }

  .project-path {
    color: $text-muted;
    font-size: $text-xs;
  }
}
</style>
