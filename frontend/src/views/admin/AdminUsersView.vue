<template>
  <div class="admin-users">
    <div class="page-header">
      <h1>用户管理</h1>
      <p class="page-description">管理系统中的所有用户账户</p>
    </div>

    <!-- Filters -->
    <div class="filters-bar">
      <div class="search-box">
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
          <circle cx="7" cy="7" r="5" stroke="currentColor" stroke-width="1.5"/>
          <path d="M11 11l3 3" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
        </svg>
        <input
          v-model="searchQuery"
          type="text"
          placeholder="搜索用户名、邮箱..."
          @input="debouncedSearch"
        />
      </div>

      <select v-model="filterRole" @change="loadUsers" class="filter-select">
        <option value="all">所有角色</option>
        <option value="admin">管理员</option>
        <option value="developer">开发者</option>
        <option value="viewer">观察者</option>
      </select>

      <select v-model="filterStatus" @change="loadUsers" class="filter-select">
        <option value="all">所有状态</option>
        <option value="active">活跃</option>
        <option value="blocked">已封禁</option>
      </select>
    </div>

    <!-- User table -->
    <div v-if="loading" class="loading-state">
      <div class="spinner"></div>
    </div>

    <div v-else class="table-container">
      <table class="users-table">
        <thead>
          <tr>
            <th>用户</th>
            <th>邮箱</th>
            <th>角色</th>
            <th>状态</th>
            <th>项目数</th>
            <th>注册时间</th>
            <th>操作</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="user in users" :key="user.id">
            <td class="user-cell">
              <span class="avatar avatar-sm">{{ getInitial(user) }}</span>
              <div class="user-names">
                <span class="username">{{ user.username }}</span>
                <span class="display-name" v-if="user.display_name">{{ user.display_name }}</span>
              </div>
            </td>
            <td class="email-cell">{{ user.email }}</td>
            <td>
              <span class="role-badge" :class="'role-' + user.role.toLowerCase()">
                {{ roleLabel(user.role) }}
              </span>
            </td>
            <td>
              <span class="status-badge" :class="user.is_active ? 'active' : 'blocked'">
                {{ user.is_active ? '活跃' : '已封禁' }}
              </span>
            </td>
            <td class="number-cell">{{ user.projects_count }}</td>
            <td class="date-cell">{{ formatDate(user.created_at) }}</td>
            <td class="actions-cell">
              <div class="actions-dropdown" :ref="el => setDropdownRef(user.id, el as HTMLElement | null)">
                <button class="action-btn" @click="toggleActions(user.id)" title="操作">
                  <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
                    <circle cx="8" cy="3" r="1.5" fill="currentColor"/>
                    <circle cx="8" cy="8" r="1.5" fill="currentColor"/>
                    <circle cx="8" cy="13" r="1.5" fill="currentColor"/>
                  </svg>
                </button>
                <Transition name="dropdown">
                  <div v-if="activeActions === user.id" class="dropdown-menu">
                    <template v-if="user.role.toLowerCase() !== 'admin'">
                      <button class="dropdown-item" @click="changeRole(user, 'admin')">设为管理员</button>
                    </template>
                    <template v-if="user.role.toLowerCase() !== 'developer'">
                      <button class="dropdown-item" @click="changeRole(user, 'developer')">设为开发者</button>
                    </template>
                    <template v-if="user.role.toLowerCase() !== 'viewer'">
                      <button class="dropdown-item" @click="changeRole(user, 'viewer')">设为观察者</button>
                    </template>
                    <div class="dropdown-divider"></div>
                    <button
                      v-if="user.is_active"
                      class="dropdown-item warning"
                      @click="toggleBlock(user)"
                    >封禁用户</button>
                    <button
                      v-else
                      class="dropdown-item success"
                      @click="toggleBlock(user)"
                    >解除封禁</button>
                    <div class="dropdown-divider"></div>
                    <button class="dropdown-item danger" @click="confirmDelete(user)">删除用户</button>
                  </div>
                </Transition>
              </div>
            </td>
          </tr>
          <tr v-if="users.length === 0">
            <td colspan="7" class="empty-cell">没有找到用户</td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- Pagination -->
    <div v-if="totalUsers > perPage" class="pagination">
      <button class="page-btn" :disabled="currentPage <= 1" @click="goToPage(currentPage - 1)">上一页</button>
      <span class="page-info">第 {{ currentPage }} 页 / 共 {{ totalPages }} 页 ({{ totalUsers }} 个用户)</span>
      <button class="page-btn" :disabled="currentPage >= totalPages" @click="goToPage(currentPage + 1)">下一页</button>
    </div>

    <!-- Delete confirmation modal -->
    <Teleport to="body">
      <div v-if="deleteTarget" class="modal-overlay" @click.self="deleteTarget = null">
        <div class="modal">
          <h3>确认删除用户</h3>
          <p>确定要删除用户 <strong>{{ deleteTarget.username }}</strong> 吗？此操作不可恢复。</p>
          <div class="modal-actions">
            <button class="btn btn-secondary" @click="deleteTarget = null">取消</button>
            <button class="btn btn-danger" @click="doDelete" :disabled="deleting">
              {{ deleting ? '删除中...' : '确认删除' }}
            </button>
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import api from '@/api'
import type { AdminUserInfo } from '@/types'

const users = ref<AdminUserInfo[]>([])
const totalUsers = ref(0)
const currentPage = ref(1)
const perPage = ref(20)
const loading = ref(true)
const searchQuery = ref('')
const filterRole = ref('all')
const filterStatus = ref('all')
const activeActions = ref<string | null>(null)
const deleteTarget = ref<AdminUserInfo | null>(null)
const deleting = ref(false)
const dropdownRefs: Record<string, HTMLElement | null> = {}

const totalPages = computed(() => Math.ceil(totalUsers.value / perPage.value))

function setDropdownRef(id: string, el: HTMLElement | null) {
  dropdownRefs[id] = el
}

let searchTimeout: ReturnType<typeof setTimeout>
function debouncedSearch() {
  clearTimeout(searchTimeout)
  searchTimeout = setTimeout(() => {
    currentPage.value = 1
    loadUsers()
  }, 300)
}

async function loadUsers() {
  loading.value = true
  try {
    const result = await api.admin.listUsers({
      page: currentPage.value,
      per_page: perPage.value,
      search: searchQuery.value || undefined,
      role: filterRole.value !== 'all' ? filterRole.value : undefined,
      status: filterStatus.value !== 'all' ? filterStatus.value : undefined,
    })
    users.value = result.users
    totalUsers.value = result.total
  } catch (err) {
    console.error('Failed to load users:', err)
  } finally {
    loading.value = false
  }
}

function goToPage(page: number) {
  currentPage.value = page
  loadUsers()
}

function getInitial(user: AdminUserInfo): string {
  return (user.display_name || user.username || 'U').charAt(0).toUpperCase()
}

function roleLabel(role: string): string {
  const labels: Record<string, string> = {
    admin: '管理员',
    developer: '开发者',
    viewer: '观察者',
  }
  return labels[role.toLowerCase()] || role
}

function formatDate(date: string): string {
  return new Date(date).toLocaleDateString('zh-CN', {
    year: 'numeric', month: '2-digit', day: '2-digit',
  })
}

function toggleActions(userId: string) {
  activeActions.value = activeActions.value === userId ? null : userId
}

async function changeRole(user: AdminUserInfo, role: 'admin' | 'developer' | 'viewer') {
  try {
    await api.admin.updateUser(user.id, { role })
    user.role = role
    activeActions.value = null
  } catch (err: any) {
    alert(err.response?.data?.message || '操作失败')
  }
}

async function toggleBlock(user: AdminUserInfo) {
  try {
    await api.admin.updateUser(user.id, { is_active: !user.is_active })
    user.is_active = !user.is_active
    activeActions.value = null
  } catch (err: any) {
    alert(err.response?.data?.message || '操作失败')
  }
}

function confirmDelete(user: AdminUserInfo) {
  activeActions.value = null
  deleteTarget.value = user
}

async function doDelete() {
  if (!deleteTarget.value) return
  deleting.value = true
  try {
    await api.admin.deleteUser(deleteTarget.value.id)
    users.value = users.value.filter(u => u.id !== deleteTarget.value!.id)
    totalUsers.value--
    deleteTarget.value = null
  } catch (err: any) {
    alert(err.response?.data?.message || '删除失败')
  } finally {
    deleting.value = false
  }
}

function handleClickOutside(e: MouseEvent) {
  if (activeActions.value) {
    const ref = dropdownRefs[activeActions.value]
    if (ref && !ref.contains(e.target as Node)) {
      activeActions.value = null
    }
  }
}

onMounted(() => {
  loadUsers()
  document.addEventListener('click', handleClickOutside)
})

onUnmounted(() => {
  document.removeEventListener('click', handleClickOutside)
})
</script>

<style lang="scss" scoped>
.admin-users {
  max-width: 1200px;
  margin: 0 auto;
  padding: $spacing-6;
}

.page-header {
  margin-bottom: $spacing-6;

  h1 {
    font-size: $font-size-2xl;
    font-weight: $font-weight-bold;
    color: $text-primary;
    margin: 0 0 $spacing-2;
  }

  .page-description {
    color: $text-secondary;
    font-size: $font-size-base;
    margin: 0;
  }
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
    background: none;
    border: none;
    color: $text-primary;
    font-size: $font-size-sm;
    width: 100%;
    outline: none;
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
    width: 32px;
    height: 32px;
    border: 3px solid $border-color;
    border-top-color: $brand-primary;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.table-container {
  overflow-x: auto;
  background: $bg-primary;
  border: 1px solid $border-color;
  border-radius: $border-radius-lg;
}

.users-table {
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

.user-cell {
  display: flex;
  align-items: center;
  gap: $spacing-3;
}

.avatar-sm {
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: $brand-gradient;
  color: white;
  font-weight: $font-weight-semibold;
  font-size: $font-size-xs;
  border-radius: 50%;
  flex-shrink: 0;
}

.user-names {
  display: flex;
  flex-direction: column;
  .username { color: $text-primary; font-weight: $font-weight-medium; }
  .display-name { font-size: $font-size-xs; color: $text-secondary; }
}

.email-cell { color: $text-secondary; }

.role-badge {
  display: inline-block;
  padding: 2px 8px;
  border-radius: 10px;
  font-size: $font-size-xs;
  font-weight: $font-weight-medium;

  &.role-admin { background: rgba(99, 102, 241, 0.1); color: #6366f1; }
  &.role-developer { background: rgba(16, 133, 72, 0.1); color: #108548; }
  &.role-viewer { background: rgba(107, 114, 128, 0.1); color: #6b7280; }
}

.status-badge {
  display: inline-block;
  padding: 2px 8px;
  border-radius: 10px;
  font-size: $font-size-xs;
  font-weight: $font-weight-medium;

  &.active { background: rgba(16, 133, 72, 0.1); color: #108548; }
  &.blocked { background: rgba(221, 43, 14, 0.1); color: #dd2b0e; }
}

.number-cell { color: $text-secondary; text-align: center; }
.date-cell { color: $text-secondary; white-space: nowrap; }
.actions-cell { position: relative; }
.actions-dropdown { position: relative; }

.action-btn {
  background: none;
  border: none;
  color: $text-muted;
  cursor: pointer;
  padding: $spacing-1;
  border-radius: $border-radius;
  &:hover { background: $bg-secondary; color: $text-primary; }
}

.dropdown-menu {
  position: absolute;
  right: 0;
  top: 100%;
  min-width: 160px;
  background: $bg-primary;
  border: 1px solid $border-color;
  border-radius: $border-radius;
  padding: $spacing-1;
  z-index: 100;
  box-shadow: $shadow-lg;
}

.dropdown-item {
  display: block;
  width: 100%;
  padding: $spacing-2 $spacing-3;
  background: none;
  border: none;
  color: $text-primary;
  font-size: $font-size-sm;
  cursor: pointer;
  border-radius: $border-radius-sm;
  text-align: left;

  &:hover { background: $bg-secondary; }
  &.warning:hover { background: rgba(171, 97, 0, 0.08); color: #ab6100; }
  &.success:hover { background: rgba(16, 133, 72, 0.08); color: #108548; }
  &.danger:hover { background: rgba(221, 43, 14, 0.08); color: #dd2b0e; }
}

.dropdown-divider { height: 1px; background: $border-color; margin: $spacing-1 0; }

.dropdown-enter-active, .dropdown-leave-active { transition: all 0.15s ease; }
.dropdown-enter-from, .dropdown-leave-to { opacity: 0; transform: translateY(-4px); }

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
.empty-cell { text-align: center !important; color: $text-secondary; padding: $spacing-8 !important; }

.modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.modal {
  background: $bg-primary;
  border: 1px solid $border-color;
  border-radius: $border-radius-lg;
  padding: $spacing-6;
  max-width: 480px;
  width: 90%;
  box-shadow: $shadow-xl;

  h3 { font-size: $font-size-lg; font-weight: $font-weight-semibold; color: $text-primary; margin: 0 0 $spacing-3; }
  p { font-size: $font-size-sm; color: $text-secondary; line-height: 1.6; margin: 0 0 $spacing-5; strong { color: $text-primary; } }
}

.modal-actions { display: flex; justify-content: flex-end; gap: $spacing-3; }

.btn {
  padding: $spacing-2 $spacing-4;
  border-radius: $border-radius;
  font-size: $font-size-sm;
  font-weight: $font-weight-medium;
  cursor: pointer;
  border: 1px solid transparent;
  &:disabled { opacity: 0.6; cursor: not-allowed; }
}

.btn-secondary {
  background: $bg-primary;
  border-color: $border-color;
  color: $text-primary;
  &:hover { background: $bg-secondary; }
}

.btn-danger {
  background: #dd2b0e;
  color: white;
  &:hover:not(:disabled) { background: #c91c00; }
}
</style>
