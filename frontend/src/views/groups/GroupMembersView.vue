<template>
  <div class="group-members-page">
    <div v-if="loading" class="loading-state">
      <div class="spinner"></div>
    </div>

    <template v-else-if="group">
      <div class="page-header">
        <div class="breadcrumb">
          <router-link :to="`/${group.path}`">{{ group.name }}</router-link>
          <span class="sep">/</span>
          <span>成员</span>
        </div>
        <div>
          <h1>群组成员</h1>
          <p class="subtitle">管理 <strong>{{ group?.name }}</strong> 的成员及其权限</p>
        </div>
      </div>

      <!-- 邀请成员区域 -->
      <div class="invite-section">
        <h3 class="section-title">邀请成员</h3>
        <div class="invite-form">
          <div class="invite-row">
            <div class="form-group flex-2">
              <label>用户名或邮箱</label>
              <input v-model="inviteForm.username" type="text" placeholder="搜索用户..." class="form-input" />
            </div>
            <div class="form-group flex-1">
              <label>角色</label>
              <select v-model="inviteForm.accessLevel" class="form-input">
                <option :value="10">访客</option>
                <option :value="20">报告者</option>
                <option :value="30">开发者</option>
                <option :value="40">维护者</option>
                <option :value="50">所有者</option>
              </select>
            </div>
            <div class="form-group flex-0">
              <label>&nbsp;</label>
              <button class="btn btn-confirm" @click="inviteMember" :disabled="!inviteForm.username || inviting">
                {{ inviting ? '邀请中...' : '邀请' }}
              </button>
            </div>
          </div>
          <div v-if="inviteError" class="alert alert-error">{{ inviteError }}</div>
          <div v-if="inviteSuccess" class="alert alert-success">{{ inviteSuccess }}</div>
        </div>
      </div>

      <!-- 现有成员列表 -->
      <div class="members-section">
        <div class="section-header">
          <h3 class="section-title">现有成员 <span class="count-badge">{{ members.length }}</span></h3>
          <input v-model="searchQuery" type="text" placeholder="搜索成员..." class="search-input" />
        </div>

        <div class="members-table">
          <div class="table-header">
            <div class="col-user">用户</div>
            <div class="col-role">角色</div>
            <div class="col-joined">加入日期</div>
            <div class="col-expiry">过期时间</div>
            <div class="col-actions"></div>
          </div>
          
          <div v-for="member in filteredMembers" :key="member.id" class="member-row">
            <div class="col-user">
              <div class="member-avatar">
                {{ (member.username || member.display_name || '?')[0].toUpperCase() }}
              </div>
              <div class="member-info">
                <span class="member-name">{{ member.display_name || member.username || `用户 #${member.user_id}` }}</span>
                <span v-if="member.username" class="member-username">@{{ member.username }}</span>
              </div>
            </div>
            <div class="col-role">
              <span class="role-badge" :class="getRoleClass(member.access_level)">
                {{ accessLevelLabel(member.access_level) }}
              </span>
            </div>
            <div class="col-joined">
              <span class="date-text">{{ formatDate(member.created_at) }}</span>
            </div>
            <div class="col-expiry">
              <span v-if="member.expires_at" class="date-text">{{ formatDate(member.expires_at) }}</span>
              <span v-else class="text-muted">无</span>
            </div>
            <div class="col-actions">
              <button 
                v-if="member.access_level < 50"
                class="btn btn-sm btn-danger-outline" 
                @click="removeMember(member)"
                title="移除成员"
              >
                <svg viewBox="0 0 16 16" fill="none"><path d="M4 4l8 8M4 12l8-8" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/></svg>
              </button>
            </div>
          </div>
          
          <div v-if="members.length === 0" class="empty-row">
            暂无成员
          </div>
        </div>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { useRoute } from 'vue-router'
import dayjs from 'dayjs'
import 'dayjs/locale/zh-cn'
import { api } from '@/api'
import { useNamespaceStore } from '@/stores/namespace'
import type { Group, GroupMember } from '@/types'
import { ACCESS_LEVEL_LABELS } from '@/types'

dayjs.locale('zh-cn')

const route = useRoute()
const namespaceStore = useNamespaceStore()

const loading = ref(true)
const group = ref<Group | null>(null)
const members = ref<GroupMember[]>([])
const searchQuery = ref('')
const inviting = ref(false)
const inviteError = ref('')
const inviteSuccess = ref('')

const inviteForm = ref({
  username: '',
  accessLevel: 30
})

const groupPath = computed(() => {
  const ns = route.params.namespace
  return Array.isArray(ns) ? ns.join('/') : ns as string
})

const filteredMembers = computed(() => {
  if (!searchQuery.value) return members.value
  const q = searchQuery.value.toLowerCase()
  return members.value.filter(m => 
    (m.username && m.username.toLowerCase().includes(q)) ||
    (m.display_name && m.display_name.toLowerCase().includes(q))
  )
})

async function loadData() {
  loading.value = true
  try {
    const [g, m] = await Promise.all([
      api.groups.get(groupPath.value),
      api.groups.listMembers(groupPath.value)
    ])
    group.value = g
    members.value = m
    // 设置群组上下文到 store
    namespaceStore.setNamespaceContext('group', groupPath.value, g)
  } catch (e) {
    console.error('Failed to load group:', e)
  } finally {
    loading.value = false
  }
}

function accessLevelLabel(level: number): string {
  return ACCESS_LEVEL_LABELS[level] || `级别 ${level}`
}

function getRoleClass(level: number): string {
  if (level >= 50) return 'owner'
  if (level >= 40) return 'maintainer'
  if (level >= 30) return 'developer'
  if (level >= 20) return 'reporter'
  return 'guest'
}

function formatDate(date: string) {
  return dayjs(date).format('YYYY-MM-DD')
}

async function inviteMember() {
  inviting.value = true
  inviteError.value = ''
  inviteSuccess.value = ''
  
  try {
    const users = await api.users.list()
    const user = users.find(u => u.username === inviteForm.value.username)
    if (!user) {
      inviteError.value = '未找到该用户'
      return
    }
    
    await api.groups.addMember(groupPath.value, {
      user_id: user.id,
      access_level: inviteForm.value.accessLevel
    })
    
    inviteSuccess.value = `已成功邀请 ${inviteForm.value.username}`
    inviteForm.value.username = ''
    await loadData()
  } catch (e: any) {
    inviteError.value = e.response?.data?.message || e.message || '邀请失败'
  } finally {
    inviting.value = false
  }
}

async function removeMember(member: GroupMember) {
  if (!confirm(`确定要移除成员 ${member.username || member.user_id} 吗？`)) return
  
  try {
    await api.groups.removeMember(groupPath.value, member.user_id)
    await loadData()
  } catch (e: any) {
    alert(e.response?.data?.message || '移除成员失败')
  }
}

onMounted(loadData)
watch(groupPath, loadData)
</script>

<style lang="scss" scoped>
.group-members-page {
  padding: $spacing-6;
  max-width: 1000px;
  margin: 0 auto;
}

.loading-state {
  display: flex;
  justify-content: center;
  padding: $spacing-12;
  
  .spinner {
    width: 32px;
    height: 32px;
    border: 3px solid $border-color;
    border-top-color: $color-primary;
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }
}

.breadcrumb {
  display: flex;
  align-items: center;
  gap: $spacing-2;
  font-size: $text-sm;
  margin-bottom: $spacing-3;
  
  a { color: $color-primary; text-decoration: none; &:hover { text-decoration: underline; } }
  .sep { color: $text-muted; }
}

.page-header {
  margin-bottom: $spacing-6;
  
  h1 {
    font-size: $text-2xl;
    font-weight: 600;
    margin-bottom: $spacing-1;
  }
  
  .subtitle {
    color: $text-secondary;
    font-size: $text-sm;
  }
}

.invite-section {
  background: $bg-secondary;
  border: 1px solid $border-color;
  border-radius: $radius-lg;
  padding: $spacing-5;
  margin-bottom: $spacing-6;
}

.section-title {
  font-size: $text-base;
  font-weight: 600;
  margin-bottom: $spacing-4;
  
  .count-badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 20px;
    height: 20px;
    padding: 0 6px;
    background: $bg-tertiary;
    border-radius: $radius-full;
    font-size: $text-xs;
    font-weight: 500;
    color: $text-secondary;
  }
}

.invite-row {
  display: flex;
  gap: $spacing-3;
  align-items: flex-end;
}

.form-group {
  display: flex;
  flex-direction: column;
  
  label {
    font-size: $text-xs;
    font-weight: 500;
    color: $text-secondary;
    margin-bottom: $spacing-1;
  }
  
  &.flex-2 { flex: 2; }
  &.flex-1 { flex: 1; }
  &.flex-0 { flex: 0 0 auto; }
}

.form-input {
  padding: $spacing-2 $spacing-3;
  border: 1px solid $border-color;
  border-radius: $radius-md;
  font-size: $text-sm;
  background: $bg-primary;
  color: $text-primary;
  
  &:focus {
    outline: none;
    border-color: $color-primary;
    box-shadow: $shadow-focus;
  }
}

.alert {
  padding: $spacing-2 $spacing-3;
  border-radius: $radius-md;
  font-size: $text-sm;
  margin-top: $spacing-3;
}

.alert-error {
  background: $color-danger-light;
  color: $color-danger;
  border: 1px solid rgba($color-danger, 0.2);
}

.alert-success {
  background: $color-success-light;
  color: darken($color-success, 10%);
  border: 1px solid rgba($color-success, 0.2);
}

.members-section {
  .section-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: $spacing-4;
  }
  
  .search-input {
    width: 250px;
    padding: $spacing-2 $spacing-3;
    border: 1px solid $border-color;
    border-radius: $radius-md;
    font-size: $text-sm;
    background: $bg-primary;
    color: $text-primary;
    
    &:focus {
      outline: none;
      border-color: $color-primary;
    }
  }
}

.members-table {
  border: 1px solid $border-color;
  border-radius: $radius-lg;
  overflow: hidden;
}

.table-header {
  display: flex;
  padding: $spacing-3 $spacing-4;
  background: $bg-tertiary;
  font-size: $text-xs;
  font-weight: 600;
  color: $text-secondary;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.member-row {
  display: flex;
  padding: $spacing-3 $spacing-4;
  align-items: center;
  border-top: 1px solid $border-color;
  transition: background $transition-fast;
  
  &:hover { background: $bg-secondary; }
}

.col-user {
  flex: 2;
  display: flex;
  align-items: center;
  gap: $spacing-3;
}

.col-role { flex: 1; }
.col-joined { flex: 1; }
.col-expiry { flex: 1; }
.col-actions { flex: 0 0 60px; text-align: right; }

.member-avatar {
  width: 36px;
  height: 36px;
  border-radius: 50%;
  background: linear-gradient(135deg, $brand-primary, $brand-secondary);
  color: white;
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 600;
  font-size: $text-sm;
  flex-shrink: 0;
}

.member-info {
  display: flex;
  flex-direction: column;
  
  .member-name {
    font-size: $text-sm;
    font-weight: 500;
    color: $text-primary;
  }
  
  .member-username {
    font-size: $text-xs;
    color: $text-muted;
  }
}

.role-badge {
  display: inline-block;
  padding: 2px 8px;
  border-radius: $radius-full;
  font-size: $text-xs;
  font-weight: 500;
  
  &.owner { background: rgba($color-danger, 0.1); color: $color-danger; }
  &.maintainer { background: rgba($color-warning, 0.1); color: darken($color-warning, 10%); }
  &.developer { background: rgba($color-primary, 0.1); color: $color-primary; }
  &.reporter { background: rgba($color-info, 0.1); color: $color-info; }
  &.guest { background: $bg-tertiary; color: $text-secondary; }
}

.date-text {
  font-size: $text-sm;
  color: $text-secondary;
}

.text-muted {
  font-size: $text-sm;
  color: $text-muted;
}

.empty-row {
  padding: $spacing-8;
  text-align: center;
  color: $text-secondary;
}

.btn {
  display: inline-flex;
  align-items: center;
  gap: $spacing-2;
  padding: $spacing-2 $spacing-4;
  border-radius: $radius-md;
  font-size: $text-sm;
  font-weight: 500;
  cursor: pointer;
  transition: all $transition-fast;
  text-decoration: none;
  border: 1px solid transparent;
  
  &:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
}

.btn-sm {
  padding: $spacing-1 $spacing-2;
  
  svg {
    width: 14px;
    height: 14px;
  }
}

.btn-confirm {
  background: $color-primary;
  color: white;
  
  &:hover:not(:disabled) { background: $color-primary-dark; }
}

.btn-danger-outline {
  background: transparent;
  color: $color-danger;
  border-color: $color-danger;
  
  &:hover:not(:disabled) {
    background: $color-danger;
    color: white;
  }
}

@keyframes spin { to { transform: rotate(360deg); } }
</style>
