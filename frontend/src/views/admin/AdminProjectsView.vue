<template>
  <div class="admin-projects">
    <div class="page-header">
      <h1>项目管理</h1>
      <p class="page-description">查看和管理系统中的所有项目</p>
    </div>

    <div class="filters-bar">
      <div class="search-box">
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
          <circle cx="7" cy="7" r="5" stroke="currentColor" stroke-width="1.5"/>
          <path d="M11 11l3 3" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
        </svg>
        <input
          v-model="searchQuery"
          type="text"
          placeholder="搜索项目名称..."
          @input="debouncedSearch"
        />
      </div>

      <select v-model="filterVisibility" @change="loadProjects" class="filter-select">
        <option value="all">所有可见性</option>
        <option value="public">公开</option>
        <option value="internal">内部</option>
        <option value="private">私有</option>
      </select>
    </div>

    <div v-if="loading" class="loading-state">
      <div class="spinner"></div>
    </div>

    <div v-else class="table-container">
      <table class="data-table">
        <thead>
          <tr>
            <th>项目</th>
            <th>可见性</th>
            <th>所有者</th>
            <th>Stars</th>
            <th>Forks</th>
            <th>创建时间</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="project in projects" :key="project.id">
            <td class="project-cell">
              <div class="project-info">
                <router-link :to="`/${project.owner_name || ''}/${project.name}`" class="project-name">
                  {{ project.owner_name }} / {{ project.name }}
                </router-link>
                <span class="project-desc" v-if="project.description">{{ project.description }}</span>
              </div>
            </td>
            <td>
              <span class="visibility-badge" :class="project.visibility">
                {{ visibilityLabel(project.visibility) }}
              </span>
            </td>
            <td class="text-secondary">{{ project.owner_name }}</td>
            <td class="number-cell">{{ project.stars_count ?? 0 }}</td>
            <td class="number-cell">{{ project.forks_count ?? 0 }}</td>
            <td class="date-cell">{{ formatDate(project.created_at) }}</td>
          </tr>
          <tr v-if="projects.length === 0">
            <td colspan="6" class="empty-cell">没有找到项目</td>
          </tr>
        </tbody>
      </table>
    </div>

    <div v-if="totalProjects > perPage" class="pagination">
      <button class="page-btn" :disabled="currentPage <= 1" @click="goToPage(currentPage - 1)">上一页</button>
      <span class="page-info">第 {{ currentPage }} 页 / 共 {{ totalPages }} 页 ({{ totalProjects }} 个项目)</span>
      <button class="page-btn" :disabled="currentPage >= totalPages" @click="goToPage(currentPage + 1)">下一页</button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import api from '@/api'
import type { Project } from '@/types'

const projects = ref<Project[]>([])
const totalProjects = ref(0)
const currentPage = ref(1)
const perPage = ref(20)
const loading = ref(true)
const searchQuery = ref('')
const filterVisibility = ref('all')

const totalPages = computed(() => Math.ceil(totalProjects.value / perPage.value))

let searchTimeout: ReturnType<typeof setTimeout>
function debouncedSearch() {
  clearTimeout(searchTimeout)
  searchTimeout = setTimeout(() => {
    currentPage.value = 1
    loadProjects()
  }, 300)
}

async function loadProjects() {
  loading.value = true
  try {
    const result = await api.projects.list(currentPage.value, perPage.value)
    let items: Project[] = Array.isArray(result) ? result : []

    // Client-side filtering (until admin-specific list endpoint is available)
    if (searchQuery.value) {
      const q = searchQuery.value.toLowerCase()
      items = items.filter(p =>
        p.name.toLowerCase().includes(q) || p.owner_name?.toLowerCase().includes(q)
      )
    }
    if (filterVisibility.value !== 'all') {
      items = items.filter(p => p.visibility === filterVisibility.value)
    }

    projects.value = items
    totalProjects.value = items.length
  } catch (err) {
    console.error('Failed to load projects:', err)
    projects.value = []
  } finally {
    loading.value = false
  }
}

function goToPage(page: number) {
  currentPage.value = page
  loadProjects()
}

function visibilityLabel(v: string): string {
  const labels: Record<string, string> = { public: '公开', internal: '内部', private: '私有' }
  return labels[v] || v
}

function formatDate(date: string): string {
  return new Date(date).toLocaleDateString('zh-CN', {
    year: 'numeric', month: '2-digit', day: '2-digit',
  })
}

onMounted(loadProjects)
</script>

<style lang="scss" scoped>
.admin-projects {
  max-width: 1200px;
  margin: 0 auto;
  padding: $spacing-6;
}

.page-header {
  margin-bottom: $spacing-6;
  h1 { font-size: $font-size-2xl; font-weight: $font-weight-bold; color: $text-primary; margin: 0 0 $spacing-2; }
  .page-description { color: $text-secondary; font-size: $font-size-base; margin: 0; }
}

.filters-bar {
  display: flex;
  gap: $spacing-3;
  margin-bottom: $spacing-5;
  flex-wrap: wrap;
}

.search-box {
  display: flex;
  align-items: center;
  gap: $spacing-2;
  background: $bg-primary;
  border: 1px solid $border-color;
  border-radius: $border-radius;
  padding: $spacing-2 $spacing-3;
  flex: 1;
  min-width: 200px;
  svg { color: $text-muted; flex-shrink: 0; }
  input {
    background: none; border: none; color: $text-primary;
    font-size: $font-size-sm; width: 100%; outline: none;
    &::placeholder { color: $text-muted; }
  }
  &:focus-within { border-color: $brand-primary; box-shadow: $shadow-focus; }
}

.filter-select {
  background: $bg-primary;
  border: 1px solid $border-color;
  border-radius: $border-radius;
  padding: $spacing-2 $spacing-3;
  color: $text-primary;
  font-size: $font-size-sm;
  cursor: pointer;
  outline: none;
  &:focus { border-color: $brand-primary; }
}

.loading-state {
  display: flex;
  justify-content: center;
  padding: $spacing-12;
  .spinner {
    width: 32px; height: 32px;
    border: 3px solid $border-color;
    border-top-color: $brand-primary;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }
}

@keyframes spin { to { transform: rotate(360deg); } }

.table-container {
  overflow-x: auto;
  background: $bg-primary;
  border: 1px solid $border-color;
  border-radius: $border-radius-lg;
}

.data-table {
  width: 100%;
  border-collapse: collapse;
  th, td {
    padding: $spacing-3 $spacing-4;
    text-align: left;
    border-bottom: 1px solid $border-color;
    font-size: $font-size-sm;
  }
  th {
    background: $bg-secondary;
    color: $text-secondary;
    font-weight: $font-weight-semibold;
    text-transform: uppercase;
    font-size: $font-size-xs;
    letter-spacing: 0.5px;
    white-space: nowrap;
  }
  tbody tr {
    transition: background $transition-fast;
    &:hover { background: $bg-secondary; }
    &:last-child td { border-bottom: none; }
  }
}

.project-cell .project-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.project-name {
  color: $brand-primary;
  font-weight: $font-weight-medium;
  text-decoration: none;
  &:hover { text-decoration: underline; }
}

.project-desc {
  font-size: $font-size-xs;
  color: $text-secondary;
  max-width: 400px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.visibility-badge {
  display: inline-block;
  padding: 2px 8px;
  border-radius: 10px;
  font-size: $font-size-xs;
  font-weight: $font-weight-medium;
  &.public { background: rgba(16, 133, 72, 0.1); color: #108548; }
  &.internal { background: rgba(171, 97, 0, 0.1); color: #ab6100; }
  &.private { background: rgba(107, 114, 128, 0.1); color: #6b7280; }
}

.text-secondary { color: $text-secondary; }
.number-cell { color: $text-secondary; text-align: center; }
.date-cell { color: $text-secondary; white-space: nowrap; }
.empty-cell { text-align: center !important; color: $text-secondary; padding: $spacing-8 !important; }

.pagination {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: $spacing-4;
  margin-top: $spacing-5;
}

.page-btn {
  padding: $spacing-2 $spacing-4;
  background: $bg-primary;
  border: 1px solid $border-color;
  border-radius: $border-radius;
  color: $text-primary;
  font-size: $font-size-sm;
  cursor: pointer;
  &:hover:not(:disabled) { border-color: $brand-primary; }
  &:disabled { opacity: 0.5; cursor: not-allowed; }
}

.page-info { font-size: $font-size-sm; color: $text-secondary; }
</style>
