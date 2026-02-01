<template>
  <div class="settings-page">
    <!-- 面包屑导航 -->
    <div class="breadcrumb">
      <router-link to="/-/profile">用户设置</router-link>
      <span class="separator">/</span>
      <span>个人资料</span>
    </div>

    <!-- 页面标题 -->
    <div class="page-header">
      <h1>个人资料</h1>
      <p class="description">管理您的个人信息和账号设置</p>
    </div>

    <!-- 主要内容 -->
    <div class="settings-content">
      <form @submit.prevent="saveProfile" class="settings-form">
        <!-- 基本信息 -->
        <section class="form-section">
          <h2>基本信息</h2>
          
          <div class="form-group">
            <label for="username">用户名</label>
            <input
              id="username"
              v-model="profile.username"
              type="text"
              class="form-input"
              disabled
            />
            <p class="form-hint">用户名不可更改</p>
          </div>

          <div class="form-group">
            <label for="display_name">显示名称</label>
            <input
              id="display_name"
              v-model="profile.display_name"
              type="text"
              class="form-input"
              placeholder="您的显示名称"
            />
          </div>

          <div class="form-group">
            <label for="email">邮箱</label>
            <input
              id="email"
              v-model="profile.email"
              type="email"
              class="form-input"
              placeholder="your@email.com"
            />
          </div>

          <div class="form-group">
            <label for="bio">个人简介</label>
            <textarea
              id="bio"
              v-model="profile.bio"
              class="form-input"
              rows="4"
              placeholder="介绍一下自己..."
            ></textarea>
          </div>
        </section>

        <!-- 安全设置 -->
        <section class="form-section">
          <h2>安全设置</h2>
          
          <div class="form-group">
            <label for="current_password">当前密码</label>
            <input
              id="current_password"
              v-model="password.current"
              type="password"
              class="form-input"
              placeholder="输入当前密码以更改密码"
            />
          </div>

          <div class="form-group">
            <label for="new_password">新密码</label>
            <input
              id="new_password"
              v-model="password.new"
              type="password"
              class="form-input"
              placeholder="输入新密码"
            />
          </div>

          <div class="form-group">
            <label for="confirm_password">确认新密码</label>
            <input
              id="confirm_password"
              v-model="password.confirm"
              type="password"
              class="form-input"
              placeholder="再次输入新密码"
            />
          </div>
        </section>

        <!-- 提交按钮 -->
        <div class="form-actions">
          <button type="submit" class="btn btn-primary" :disabled="saving">
            {{ saving ? '保存中...' : '保存更改' }}
          </button>
        </div>

        <!-- 消息提示 -->
        <div v-if="message" :class="['alert', messageType === 'success' ? 'alert-success' : 'alert-error']">
          {{ message }}
        </div>
      </form>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { useAuthStore } from '@/stores/auth'
import apiClient from '@/api'

const authStore = useAuthStore()

const profile = reactive({
  username: '',
  display_name: '',
  email: '',
  bio: ''
})

const password = reactive({
  current: '',
  new: '',
  confirm: ''
})

const saving = ref(false)
const message = ref('')
const messageType = ref<'success' | 'error'>('success')

const loadProfile = () => {
  const user = authStore.user
  if (user) {
    profile.username = user.username || ''
    profile.display_name = user.display_name || ''
    profile.email = user.email || ''
    profile.bio = ''
  }
}

const saveProfile = async () => {
  saving.value = true
  message.value = ''

  try {
    // Validate password change
    if (password.new) {
      if (!password.current) {
        throw new Error('请输入当前密码')
      }
      if (password.new !== password.confirm) {
        throw new Error('新密码与确认密码不匹配')
      }
      if (password.new.length < 6) {
        throw new Error('新密码至少需要 6 个字符')
      }
    }

    // Update profile
    await apiClient.client.put('/user/profile', {
      display_name: profile.display_name,
      email: profile.email,
      bio: profile.bio
    })

    // Update password if provided
    if (password.new && password.current) {
      await apiClient.client.put('/user/password', {
        current_password: password.current,
        new_password: password.new
      })
      password.current = ''
      password.new = ''
      password.confirm = ''
    }

    message.value = '保存成功'
    messageType.value = 'success'
    
    // Refresh user data
    await authStore.fetchCurrentUser()
  } catch (error: any) {
    message.value = error.response?.data?.message || error.message || '保存失败'
    messageType.value = 'error'
  } finally {
    saving.value = false
  }
}

onMounted(() => {
  loadProfile()
})
</script>

<style lang="scss" scoped>
.settings-page {
  padding: 24px 32px;
  max-width: 900px;
}

.breadcrumb {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
  margin-bottom: 16px;
  
  a {
    color: var(--text-secondary, #8b949e);
    text-decoration: none;
    
    &:hover {
      color: var(--text-link, #58a6ff);
      text-decoration: underline;
    }
  }
  
  .separator {
    color: var(--text-secondary, #8b949e);
  }
  
  span:last-child {
    color: var(--text-primary, #c9d1d9);
  }
}

.page-header {
  margin-bottom: 24px;
  
  h1 {
    font-size: 24px;
    font-weight: 600;
    margin: 0 0 8px 0;
    color: var(--text-primary, #c9d1d9);
  }
  
  .description {
    font-size: 14px;
    color: var(--text-secondary, #8b949e);
    margin: 0;
  }
}

.settings-content {
  background: var(--bg-secondary, #161b22);
  border: 1px solid var(--border-color, #30363d);
  border-radius: 6px;
}

.settings-form {
  padding: 0;
}

.form-section {
  padding: 24px;
  border-bottom: 1px solid var(--border-color, #30363d);
  
  &:last-of-type {
    border-bottom: none;
  }
  
  h2 {
    font-size: 16px;
    font-weight: 600;
    margin: 0 0 16px 0;
    color: var(--text-primary, #c9d1d9);
  }
}

.form-group {
  margin-bottom: 16px;
  
  &:last-child {
    margin-bottom: 0;
  }
  
  label {
    display: block;
    font-size: 14px;
    font-weight: 500;
    color: var(--text-primary, #c9d1d9);
    margin-bottom: 8px;
  }
}

.form-input {
  width: 100%;
  max-width: 480px;
  padding: 8px 12px;
  font-size: 14px;
  color: var(--text-primary, #c9d1d9);
  background: var(--bg-primary, #0d1117);
  border: 1px solid var(--border-color, #30363d);
  border-radius: 6px;
  transition: border-color 0.2s, box-shadow 0.2s;
  
  &:focus {
    outline: none;
    border-color: var(--color-primary, #58a6ff);
    box-shadow: 0 0 0 3px rgba(88, 166, 255, 0.15);
  }
  
  &:disabled {
    opacity: 0.6;
    cursor: not-allowed;
    background: var(--bg-tertiary, #21262d);
  }
  
  &::placeholder {
    color: var(--text-muted, #484f58);
  }
}

textarea.form-input {
  resize: vertical;
  min-height: 80px;
}

.form-hint {
  margin: 6px 0 0 0;
  font-size: 12px;
  color: var(--text-secondary, #8b949e);
}

.form-actions {
  padding: 16px 24px;
  background: var(--bg-tertiary, #21262d);
  border-top: 1px solid var(--border-color, #30363d);
}

.btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 8px 16px;
  font-size: 14px;
  font-weight: 500;
  border-radius: 6px;
  cursor: pointer;
  transition: background 0.2s;
  
  &.btn-primary {
    background: var(--color-success, #238636);
    border: 1px solid var(--color-success, #238636);
    color: white;
    
    &:hover:not(:disabled) {
      background: var(--color-success-hover, #2ea043);
    }
    
    &:disabled {
      opacity: 0.6;
      cursor: not-allowed;
    }
  }
}

.alert {
  margin: 16px 24px;
  padding: 12px 16px;
  border-radius: 6px;
  font-size: 14px;
  
  &.alert-success {
    background: rgba(46, 160, 67, 0.15);
    border: 1px solid var(--color-success, #238636);
    color: #3fb950;
  }
  
  &.alert-error {
    background: rgba(248, 81, 73, 0.15);
    border: 1px solid #f85149;
    color: #f85149;
  }
}
</style>
