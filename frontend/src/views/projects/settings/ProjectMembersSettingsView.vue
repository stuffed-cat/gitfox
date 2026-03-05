<template>
  <div class="project-settings-page">
    <div class="settings-header">
      <h2>成员管理</h2>
      <p class="description">管理项目成员和访问权限</p>
    </div>
    
    <div v-if="loading" class="loading">
      <div class="loading-spinner"></div>
    </div>
    
    <template v-else>
      <!-- 邀请成员 -->
      <section class="settings-section">
        <h3>邀请成员</h3>
        
        <div class="invite-form">
          <div class="form-row">
            <div class="form-group flex-grow">
              <label for="invite-username">用户名或邮箱</label>
              <input
                id="invite-username"
                v-model="inviteForm.username"
                type="text"
                class="form-control"
                placeholder="搜索用户..."
                @input="searchUsers"
              />
              
              <!-- 用户搜索结果 -->
              <div v-if="searchResults.length > 0" class="search-dropdown">
                <div
                  v-for="user in searchResults"
                  :key="user.id"
                  class="search-result"
                  @click="selectUser(user)"
                >
                  <img :src="user.avatar_url || '/default-avatar.png'" :alt="user.username" class="user-avatar" />
                  <div class="user-info">
                    <span class="user-name">{{ user.display_name || user.username }}</span>
                    <span class="user-username">@{{ user.username }}</span>
                  </div>
                </div>
              </div>
            </div>
            
            <div class="form-group">
              <label for="invite-role">角色</label>
              <select id="invite-role" v-model="inviteForm.role" class="form-control">
                <option value="guest">访客 (Guest)</option>
                <option value="reporter">报告者 (Reporter)</option>
                <option value="developer">开发者 (Developer)</option>
                <option value="maintainer">维护者 (Maintainer)</option>
              </select>
            </div>
          </div>
          
          <div class="form-group">
            <label for="invite-expires">访问到期时间 (可选)</label>
            <input
              id="invite-expires"
              v-model="inviteForm.expires_at"
              type="date"
              class="form-control"
              :min="minDate"
            />
          </div>
          
          <button class="btn btn-primary" @click="inviteMember" :disabled="!inviteForm.username">
            添加成员
          </button>
        </div>
      </section>

      <!-- 成员列表 -->
      <section class="settings-section">
        <div class="section-header">
          <h3>项目成员</h3>
          <span class="member-count">{{ members.length }} 位成员</span>
        </div>
        
        <div class="filter-bar">
          <input
            v-model="filterQuery"
            type="text"
            class="form-control"
            placeholder="搜索成员..."
          />
          <select v-model="filterRole" class="form-control">
            <option value="">所有角色</option>
            <option value="owner">所有者</option>
            <option value="maintainer">维护者</option>
            <option value="developer">开发者</option>
            <option value="reporter">报告者</option>
            <option value="guest">访客</option>
          </select>
        </div>
        
        <div class="member-list">
          <div v-for="member in filteredMembers" :key="member.id" class="member-item">
            <div class="member-info">
              <img :src="member.avatar_url || '/default-avatar.png'" :alt="member.username" class="member-avatar" />
              <div class="member-detail">
                <div class="member-name">
                  <span>{{ member.display_name || member.username }}</span>
                  <span v-if="member.role === 'owner'" class="badge badge-primary">所有者</span>
                </div>
                <div class="member-username">@{{ member.username }}</div>
                <div class="member-meta">
                  加入于 {{ formatDate(member.created_at) }}
                  <span v-if="member.expires_at" class="expires-warning">
                    · 到期: {{ formatDate(member.expires_at) }}
                  </span>
                </div>
              </div>
            </div>
            
            <div class="member-actions">
              <select
                :value="member.role"
                class="form-control form-control-sm"
                @change="updateMemberRole(member, ($event.target as HTMLSelectElement).value)"
                :disabled="member.role === 'owner' || isCurrentUser(member)"
              >
                <option value="guest">访客</option>
                <option value="reporter">报告者</option>
                <option value="developer">开发者</option>
                <option value="maintainer">维护者</option>
                <option v-if="member.role === 'owner'" value="owner">所有者</option>
              </select>
              
              <button
                class="btn btn-danger btn-sm"
                @click="removeMember(member)"
                :disabled="member.role === 'owner' || isCurrentUser(member)"
                :title="member.role === 'owner' ? '无法移除所有者' : isCurrentUser(member) ? '无法移除自己' : '移除成员'"
              >
                移除
              </button>
            </div>
          </div>
          
          <div v-if="filteredMembers.length === 0" class="empty-state">
            <p v-if="filterQuery || filterRole">没有找到匹配的成员</p>
            <p v-else>暂无成员</p>
          </div>
        </div>
      </section>

      <!-- 角色权限说明 -->
      <section class="settings-section">
        <h3>角色权限说明</h3>
        
        <div class="roles-table">
          <table>
            <thead>
              <tr>
                <th>权限</th>
                <th>访客</th>
                <th>报告者</th>
                <th>开发者</th>
                <th>维护者</th>
                <th>所有者</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td>查看代码</td>
                <td>✓</td>
                <td>✓</td>
                <td>✓</td>
                <td>✓</td>
                <td>✓</td>
              </tr>
              <tr>
                <td>克隆仓库</td>
                <td>✓</td>
                <td>✓</td>
                <td>✓</td>
                <td>✓</td>
                <td>✓</td>
              </tr>
              <tr>
                <td>创建议题</td>
                <td>✓</td>
                <td>✓</td>
                <td>✓</td>
                <td>✓</td>
                <td>✓</td>
              </tr>
              <tr>
                <td>创建合并请求</td>
                <td></td>
                <td>✓</td>
                <td>✓</td>
                <td>✓</td>
                <td>✓</td>
              </tr>
              <tr>
                <td>推送代码</td>
                <td></td>
                <td></td>
                <td>✓</td>
                <td>✓</td>
                <td>✓</td>
              </tr>
              <tr>
                <td>合并代码</td>
                <td></td>
                <td></td>
                <td>✓</td>
                <td>✓</td>
                <td>✓</td>
              </tr>
              <tr>
                <td>管理分支保护</td>
                <td></td>
                <td></td>
                <td></td>
                <td>✓</td>
                <td>✓</td>
              </tr>
              <tr>
                <td>管理成员</td>
                <td></td>
                <td></td>
                <td></td>
                <td>✓</td>
                <td>✓</td>
              </tr>
              <tr>
                <td>项目设置</td>
                <td></td>
                <td></td>
                <td></td>
                <td>✓</td>
                <td>✓</td>
              </tr>
              <tr>
                <td>删除项目</td>
                <td></td>
                <td></td>
                <td></td>
                <td></td>
                <td>✓</td>
              </tr>
            </tbody>
          </table>
        </div>
      </section>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, watch, computed } from 'vue'
import api from '@/api'
import { useAuthStore } from '@/stores/auth'
import type { Project, User } from '@/types'

interface ProjectMember {
  id: string
  user_id: string
  username: string
  display_name?: string
  avatar_url?: string
  role: 'owner' | 'maintainer' | 'developer' | 'reporter' | 'guest'
  created_at: string
  expires_at?: string
}

const props = defineProps<{
  project?: Project
}>()

const authStore = useAuthStore()
const loading = ref(false)
const members = ref<ProjectMember[]>([])
const searchResults = ref<User[]>([])
const filterQuery = ref('')
const filterRole = ref('')

const inviteForm = reactive({
  username: '',
  role: 'developer',
  expires_at: ''
})

const minDate = computed(() => {
  const tomorrow = new Date()
  tomorrow.setDate(tomorrow.getDate() + 1)
  return tomorrow.toISOString().split('T')[0]
})

const filteredMembers = computed(() => {
  let result = members.value
  
  if (filterQuery.value) {
    const query = filterQuery.value.toLowerCase()
    result = result.filter(m =>
      m.username.toLowerCase().includes(query) ||
      (m.display_name && m.display_name.toLowerCase().includes(query))
    )
  }
  
  if (filterRole.value) {
    result = result.filter(m => m.role === filterRole.value)
  }
  
  // 按角色排序：owner > maintainer > developer > reporter > guest
  const roleOrder = { owner: 0, maintainer: 1, developer: 2, reporter: 3, guest: 4 }
  return result.sort((a, b) => roleOrder[a.role] - roleOrder[b.role])
})

function formatDate(dateStr: string): string {
  return new Date(dateStr).toLocaleDateString('zh-CN')
}

function isCurrentUser(member: ProjectMember): boolean {
  return member.user_id === authStore.user?.id
}

async function loadMembers() {
  if (!props.project?.owner_name || !props.project?.name) return
  loading.value = true
  
  const path = { namespace: props.project.owner_name, project: props.project.name }
  
  try {
    const data = await api.projects.getMembers(path)
    // 需要获取用户详细信息
    members.value = data.map(m => ({
      id: m.id,
      user_id: m.user_id,
      username: m.user_id.substring(0, 8), // 临时显示
      display_name: undefined,
      avatar_url: undefined,
      role: m.role as ProjectMember['role'],
      created_at: m.created_at,
      expires_at: undefined
    }))
  } catch (error) {
    console.error('Failed to load members:', error)
  } finally {
    loading.value = false
  }
}

let searchTimeout: number | null = null

async function searchUsers() {
  if (searchTimeout) {
    clearTimeout(searchTimeout)
  }
  
  if (!inviteForm.username || inviteForm.username.length < 2) {
    searchResults.value = []
    return
  }
  
  searchTimeout = window.setTimeout(async () => {
    try {
      // TODO: 后端 API 尚未支持用户搜索
      // 需要在后端添加 /users?search=xxx 支持
      // 目前使用 list 获取前几页用户进行本地过滤
      const users = await api.users.list(1, 50)
      // 本地过滤匹配用户名
      const searchTerm = inviteForm.username.toLowerCase()
      const filtered = users.filter(u => 
        u.username.toLowerCase().includes(searchTerm) ||
        u.email?.toLowerCase().includes(searchTerm) ||
        u.display_name?.toLowerCase().includes(searchTerm)
      )
      // 过滤已经是成员的用户
      const memberIds = new Set(members.value.map(m => m.user_id))
      searchResults.value = filtered.filter(u => !memberIds.has(u.id)).slice(0, 5)
    } catch (error) {
      console.error('Failed to search users:', error)
    }
  }, 300)
}

function selectUser(user: User) {
  inviteForm.username = user.username
  searchResults.value = []
}

async function inviteMember() {
  if (!props.project?.owner_name || !props.project?.name || !inviteForm.username) return
  
  const path = { namespace: props.project.owner_name, project: props.project.name }
  
  try {
    await api.projects.addMember(path, {
      username: inviteForm.username,
      role: inviteForm.role
    })
    
    // 重置表单
    inviteForm.username = ''
    inviteForm.role = 'developer'
    inviteForm.expires_at = ''
    
    // 重新加载成员列表
    loadMembers()
    
    alert('成员已添加')
  } catch (error) {
    console.error('Failed to add member:', error)
    alert('添加成员失败')
  }
}

async function updateMemberRole(member: ProjectMember, newRole: string) {
  if (!props.project?.owner_name || !props.project?.name) return
  
  try {
    // TODO: 后端 API 尚未支持更新成员角色
    // 需要添加 PUT /projects/{ns}/{proj}/members/{user_id} 端点
    // const path = { namespace: props.project.owner_name, project: props.project.name }
    // await api.projects.updateMember(path, member.user_id, { role: newRole })
    
    // 本地更新（演示用）
    member.role = newRole as ProjectMember['role']
    
    alert('角色已更新（仅本地预览，API 即将支持）')
  } catch (error) {
    console.error('Failed to update member role:', error)
    alert('更新角色失败')
    loadMembers() // 重新加载以恢复原状态
  }
}

async function removeMember(member: ProjectMember) {
  if (!props.project?.owner_name || !props.project?.name) return
  if (!confirm(`确定要移除成员 @${member.username} 吗？`)) return
  
  const path = { namespace: props.project.owner_name, project: props.project.name }
  
  try {
    await api.projects.removeMember(path, member.user_id)
    
    // 从列表中移除
    members.value = members.value.filter(m => m.id !== member.id)
    
    alert('成员已移除')
  } catch (error) {
    console.error('Failed to remove member:', error)
    alert('移除成员失败')
  }
}

watch([() => props.project?.owner_name, () => props.project?.name], () => {
  loadMembers()
}, { immediate: true })
</script>

<style lang="scss" scoped>
.project-settings-page {
  padding: $spacing-lg;
  max-width: 900px;
}

.settings-header {
  margin-bottom: $spacing-xl;
  
  h2 {
    margin: 0 0 $spacing-xs 0;
  }
  
  .description {
    color: $text-muted;
    margin: 0;
  }
}

.settings-section {
  margin-bottom: $spacing-xl;
  padding-bottom: $spacing-xl;
  border-bottom: 1px solid $border-color;
  
  &:last-child {
    border-bottom: none;
  }
  
  h3 {
    margin-bottom: $spacing-md;
  }
}

.section-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: $spacing-md;
  
  h3 {
    margin: 0;
  }
  
  .member-count {
    color: $text-muted;
    font-size: $font-size-sm;
  }
}

.invite-form {
  padding: $spacing-lg;
  background: $bg-secondary;
  border-radius: $border-radius;
}

.form-row {
  display: flex;
  gap: $spacing-md;
  margin-bottom: $spacing-md;
  
  .form-group {
    margin-bottom: 0;
    
    &.flex-grow {
      flex: 1;
      position: relative;
    }
  }
}

.form-group {
  margin-bottom: $spacing-md;
  
  label {
    display: block;
    margin-bottom: $spacing-xs;
    font-weight: 500;
  }
}

.search-dropdown {
  position: absolute;
  top: 100%;
  left: 0;
  right: 0;
  background: $bg-primary;
  border: 1px solid $border-color;
  border-radius: $border-radius;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
  z-index: 100;
  max-height: 200px;
  overflow-y: auto;
}

.search-result {
  display: flex;
  align-items: center;
  gap: $spacing-sm;
  padding: $spacing-sm $spacing-md;
  cursor: pointer;
  
  &:hover {
    background: $bg-secondary;
  }
  
  .user-avatar {
    width: 32px;
    height: 32px;
    border-radius: 50%;
  }
  
  .user-info {
    display: flex;
    flex-direction: column;
    
    .user-name {
      font-weight: 500;
    }
    
    .user-username {
      font-size: $font-size-sm;
      color: $text-muted;
    }
  }
}

.filter-bar {
  display: flex;
  gap: $spacing-md;
  margin-bottom: $spacing-lg;
  
  input {
    flex: 1;
  }
  
  select {
    width: 150px;
  }
}

.member-list {
  display: flex;
  flex-direction: column;
  gap: $spacing-sm;
}

.member-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: $spacing-md;
  background: $bg-secondary;
  border-radius: $border-radius;
}

.member-info {
  display: flex;
  align-items: center;
  gap: $spacing-md;
}

.member-avatar {
  width: 40px;
  height: 40px;
  border-radius: 50%;
}

.member-detail {
  .member-name {
    display: flex;
    align-items: center;
    gap: $spacing-sm;
    font-weight: 500;
  }
  
  .member-username {
    font-size: $font-size-sm;
    color: $text-muted;
  }
  
  .member-meta {
    font-size: $font-size-sm;
    color: $text-muted;
    margin-top: $spacing-xs;
  }
  
  .expires-warning {
    color: $warning-color;
  }
}

.member-actions {
  display: flex;
  align-items: center;
  gap: $spacing-sm;
  
  select {
    width: 120px;
  }
}

.badge {
  font-size: $font-size-xs;
  padding: 2px 6px;
  border-radius: 3px;
  
  &.badge-primary {
    background: $primary-color;
    color: white;
  }
}

.roles-table {
  overflow-x: auto;
  
  table {
    width: 100%;
    border-collapse: collapse;
    
    th, td {
      padding: $spacing-sm $spacing-md;
      border: 1px solid $border-color;
      text-align: center;
      
      &:first-child {
        text-align: left;
      }
    }
    
    th {
      background: $bg-secondary;
      font-weight: 500;
    }
    
    td {
      color: $text-muted;
      
      &:first-child {
        color: $text-primary;
      }
    }
  }
}

.empty-state {
  padding: $spacing-lg;
  text-align: center;
  color: $text-muted;
}
</style>
