<template>
  <div class="project-settings">
    <h2>项目设置</h2>
    
    <div v-if="loading" class="loading">
      <div class="loading-spinner"></div>
    </div>
    
    <template v-else>
      <!-- 基本信息 -->
      <section class="settings-section">
        <h3>基本信息</h3>
        <form @submit.prevent="saveBasicInfo" class="settings-form">
          <div class="form-group">
            <label for="name">项目名称</label>
            <input
              id="name"
              v-model="basicForm.name"
              type="text"
              class="form-control"
              required
            />
          </div>
          
          <div class="form-group">
            <label for="description">项目描述</label>
            <textarea
              id="description"
              v-model="basicForm.description"
              class="form-control"
              rows="3"
            ></textarea>
          </div>
          
          <div class="form-group">
            <label for="visibility">可见性</label>
            <select id="visibility" v-model="basicForm.visibility" class="form-control">
              <option value="private">私有</option>
              <option value="internal">内部</option>
              <option value="public">公开</option>
            </select>
          </div>
          
          <div class="form-group">
            <label for="defaultBranch">默认分支</label>
            <select id="defaultBranch" v-model="basicForm.default_branch" class="form-control">
              <option v-for="branch in branches" :key="branch.name" :value="branch.name">
                {{ branch.name }}
              </option>
            </select>
          </div>
          
          <button type="submit" class="btn btn-primary" :disabled="saving">
            {{ saving ? '保存中...' : '保存更改' }}
          </button>
        </form>
      </section>
      
      <!-- 成员管理 -->
      <section class="settings-section">
        <h3>项目成员</h3>
        
        <div class="add-member">
          <input
            v-model="newMemberUsername"
            type="text"
            class="form-control"
            placeholder="输入用户名"
          />
          <select v-model="newMemberRole" class="form-control">
            <option value="guest">访客</option>
            <option value="reporter">报告者</option>
            <option value="developer">开发者</option>
            <option value="maintainer">维护者</option>
          </select>
          <button class="btn btn-primary" @click="addMember" :disabled="!newMemberUsername">
            添加
          </button>
        </div>
        
        <div class="member-list">
          <div v-for="member in members" :key="member.user_id" class="member-item">
            <div class="member-info">
              <span class="member-avatar">{{ member.username?.charAt(0).toUpperCase() }}</span>
              <div class="member-detail">
                <strong>{{ member.display_name || member.username }}</strong>
                <span class="member-role">{{ roleText(member.role) }}</span>
              </div>
            </div>
            <div class="member-actions">
              <select
                :value="member.role"
                class="form-control form-control-sm"
                @change="updateMemberRole(member.user_id, ($event.target as HTMLSelectElement).value)"
              >
                <option value="guest">访客</option>
                <option value="reporter">报告者</option>
                <option value="developer">开发者</option>
                <option value="maintainer">维护者</option>
              </select>
              <button class="btn btn-danger btn-sm" @click="removeMember(member.user_id)">
                移除
              </button>
            </div>
          </div>
        </div>
      </section>
      
      <!-- Webhooks -->
      <section class="settings-section">
        <h3>Webhooks</h3>
        
        <div class="add-webhook">
          <input
            v-model="newWebhookUrl"
            type="url"
            class="form-control"
            placeholder="https://example.com/webhook"
          />
          <button class="btn btn-primary" @click="addWebhook" :disabled="!newWebhookUrl">
            添加
          </button>
        </div>
        
        <div class="webhook-list">
          <div v-for="webhook in webhooks" :key="webhook.id" class="webhook-item">
            <div class="webhook-info">
              <code>{{ webhook.url }}</code>
              <span class="webhook-status" :class="{ active: webhook.is_active }">
                {{ webhook.is_active ? '启用' : '禁用' }}
              </span>
            </div>
            <div class="webhook-actions">
              <button class="btn btn-outline btn-sm" @click="toggleWebhook(webhook)">
                {{ webhook.is_active ? '禁用' : '启用' }}
              </button>
              <button class="btn btn-danger btn-sm" @click="deleteWebhook(webhook.id)">
                删除
              </button>
            </div>
          </div>
        </div>
      </section>
      
      <!-- 危险区域 -->
      <section class="settings-section danger-zone">
        <h3>危险操作</h3>
        <div class="danger-item">
          <div class="danger-info">
            <strong>删除项目</strong>
            <p>删除项目后，所有相关数据将被永久删除，此操作不可撤销。</p>
          </div>
          <button class="btn btn-danger" @click="deleteProject">
            删除项目
          </button>
        </div>
      </section>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, watch } from 'vue'
import { useRouter } from 'vue-router'
import api from '@/api'
import type { Project, Branch } from '@/types'

interface Member {
  user_id: string
  username: string
  display_name?: string
  role: string
}

interface Webhook {
  id: string
  url: string
  is_active: boolean
  events: string[]
}

const props = defineProps<{
  project?: Project
}>()

const router = useRouter()

const loading = ref(false)
const saving = ref(false)
const branches = ref<Branch[]>([])
const members = ref<Member[]>([])
const webhooks = ref<Webhook[]>([])

const basicForm = reactive({
  name: '',
  description: '',
  visibility: 'private' as 'public' | 'private' | 'internal',
  default_branch: ''
})

const newMemberUsername = ref('')
const newMemberRole = ref('developer')
const newWebhookUrl = ref('')

function roleText(role: string) {
  const map: Record<string, string> = {
    guest: '访客',
    reporter: '报告者',
    developer: '开发者',
    maintainer: '维护者',
    owner: '所有者'
  }
  return map[role] || role
}

async function loadSettings() {
  if (!props.project?.id) return
  loading.value = true
  
  try {
    const [branchRes, memberRes, webhookRes] = await Promise.all([
      api.getBranches(props.project.id),
      api.getProjectMembers(props.project.id),
      api.getWebhooks(props.project.id)
    ])
    
    branches.value = branchRes.data
    members.value = memberRes.data
    webhooks.value = webhookRes.data
    
    // 初始化表单
    basicForm.name = props.project.name
    basicForm.description = props.project.description || ''
    basicForm.visibility = props.project.visibility
    basicForm.default_branch = props.project.default_branch
  } catch (error) {
    console.error('Failed to load settings:', error)
  } finally {
    loading.value = false
  }
}

async function saveBasicInfo() {
  if (!props.project?.id) return
  saving.value = true
  
  try {
    await api.updateProject(props.project.id, basicForm)
    alert('保存成功')
  } catch (error) {
    console.error('Failed to save:', error)
    alert('保存失败')
  } finally {
    saving.value = false
  }
}

async function addMember() {
  if (!props.project?.id || !newMemberUsername.value) return
  
  try {
    await api.addProjectMember(props.project.id, {
      username: newMemberUsername.value,
      role: newMemberRole.value
    })
    newMemberUsername.value = ''
    loadSettings()
  } catch (error) {
    console.error('Failed to add member:', error)
    alert('添加成员失败')
  }
}

async function updateMemberRole(userId: string, role: string) {
  if (!props.project?.id) return
  
  try {
    await api.updateProjectMember(props.project.id, userId, { role })
    loadSettings()
  } catch (error) {
    console.error('Failed to update member:', error)
  }
}

async function removeMember(userId: string) {
  if (!props.project?.id) return
  if (!confirm('确定要移除此成员吗？')) return
  
  try {
    await api.removeProjectMember(props.project.id, userId)
    loadSettings()
  } catch (error) {
    console.error('Failed to remove member:', error)
  }
}

async function addWebhook() {
  if (!props.project?.id || !newWebhookUrl.value) return
  
  try {
    await api.createWebhook(props.project.id, {
      url: newWebhookUrl.value,
      events: ['push', 'merge_request', 'pipeline']
    })
    newWebhookUrl.value = ''
    loadSettings()
  } catch (error) {
    console.error('Failed to add webhook:', error)
    alert('添加 Webhook 失败')
  }
}

async function toggleWebhook(webhook: Webhook) {
  if (!props.project?.id) return
  
  try {
    await api.updateWebhook(props.project.id, webhook.id, {
      is_active: !webhook.is_active
    })
    loadSettings()
  } catch (error) {
    console.error('Failed to toggle webhook:', error)
  }
}

async function deleteWebhook(webhookId: string) {
  if (!props.project?.id) return
  if (!confirm('确定要删除此 Webhook 吗？')) return
  
  try {
    await api.deleteWebhook(props.project.id, webhookId)
    loadSettings()
  } catch (error) {
    console.error('Failed to delete webhook:', error)
  }
}

async function deleteProject() {
  if (!props.project?.id) return
  
  const confirmed = prompt(`请输入项目名称 "${props.project.name}" 以确认删除：`)
  if (confirmed !== props.project.name) {
    alert('项目名称不匹配')
    return
  }
  
  try {
    await api.deleteProject(props.project.id)
    router.push('/projects')
  } catch (error) {
    console.error('Failed to delete project:', error)
    alert('删除项目失败')
  }
}

watch(() => props.project?.id, () => {
  loadSettings()
}, { immediate: true })
</script>

<style lang="scss" scoped>
.project-settings {
  padding: $spacing-lg;
  max-width: 800px;
  
  h2 {
    margin-bottom: $spacing-xl;
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
    margin-bottom: $spacing-lg;
  }
}

.settings-form {
  .form-group {
    margin-bottom: $spacing-md;
  }
  
  button {
    margin-top: $spacing-md;
  }
}

.add-member, .add-webhook {
  display: flex;
  gap: $spacing-md;
  margin-bottom: $spacing-lg;
  
  input {
    flex: 1;
  }
  
  select {
    width: auto;
  }
}

.member-list, .webhook-list {
  display: flex;
  flex-direction: column;
  gap: $spacing-sm;
}

.member-item, .webhook-item {
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
  width: 36px;
  height: 36px;
  border-radius: 50%;
  background: $primary-color;
  color: white;
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 600;
}

.member-detail {
  display: flex;
  flex-direction: column;
  
  .member-role {
    font-size: $font-size-sm;
    color: $text-muted;
  }
}

.member-actions, .webhook-actions {
  display: flex;
  align-items: center;
  gap: $spacing-sm;
}

.webhook-info {
  display: flex;
  align-items: center;
  gap: $spacing-md;
  
  code {
    font-size: $font-size-sm;
  }
}

.webhook-status {
  font-size: $font-size-xs;
  padding: 2px 6px;
  border-radius: 3px;
  background: $bg-tertiary;
  color: $text-muted;
  
  &.active {
    background: rgba($success-color, 0.2);
    color: $success-color;
  }
}

.danger-zone {
  h3 {
    color: $danger-color;
  }
}

.danger-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: $spacing-lg;
  background: rgba($danger-color, 0.1);
  border: 1px solid rgba($danger-color, 0.3);
  border-radius: $border-radius;
  
  p {
    margin: $spacing-xs 0 0;
    font-size: $font-size-sm;
    color: $text-muted;
  }
}
</style>
