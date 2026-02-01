<template>
  <div class="profile-page">
    <!-- 面包屑导航 -->
    <div class="breadcrumb">
      <router-link to="/-/profile">用户设置</router-link>
      <span class="separator">/</span>
      <span>编辑个人资料</span>
    </div>

    <!-- 搜索框 -->
    <div class="search-box">
      <svg class="search-icon" viewBox="0 0 16 16" width="16" height="16" fill="none">
        <path d="M11.5 7a4.5 4.5 0 1 1-9 0 4.5 4.5 0 0 1 9 0ZM10.5 11a5.5 5.5 0 1 1 1-1l3 3a.75.75 0 1 1-1 1l-3-3Z" stroke="currentColor" stroke-width="1.2"/>
      </svg>
      <input type="text" placeholder="搜索页" v-model="searchQuery" />
    </div>

    <!-- 公开头像 -->
    <section class="profile-section">
      <h2>公开头像</h2>
      <p class="section-description">
        可以在这里上传您的头像
      </p>
      
      <div class="avatar-upload">
        <div class="avatar-preview">
          <img v-if="profile.avatar_url" :src="profile.avatar_url" alt="头像" />
          <div v-else class="avatar-placeholder">
            <svg viewBox="0 0 24 24" width="40" height="40" fill="none">
              <circle cx="12" cy="8" r="4" stroke="currentColor" stroke-width="1.5"/>
              <path d="M4 20c0-4 4-6 8-6s8 2 8 6" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
            </svg>
          </div>
        </div>
        <div class="avatar-actions">
          <h3>上传新头像</h3>
          <div class="file-input-wrapper">
            <label class="file-input-btn">
              选择文件...
              <input type="file" accept="image/*" @change="handleAvatarChange" />
            </label>
            <span class="file-name">{{ avatarFileName || '未选择文件。' }}</span>
          </div>
          <p class="upload-hint">理想的图像尺寸为 192 x 192 像素。允许的最大文件大小为 200 KiB。</p>
        </div>
      </div>
    </section>

    <!-- 当前状态 -->
    <section class="profile-section">
      <h2>当前状态</h2>
      <p class="section-description">此表情符号和消息会显示在您的个人资料和界面中。</p>
      
      <div class="status-input-wrapper">
        <button class="emoji-btn" @click="showEmojiPicker = !showEmojiPicker">
          <span v-if="profile.status_emoji">{{ profile.status_emoji }}</span>
          <svg v-else viewBox="0 0 16 16" width="16" height="16" fill="none">
            <circle cx="8" cy="8" r="7" stroke="currentColor" stroke-width="1.2"/>
            <circle cx="5.5" cy="6.5" r="1" fill="currentColor"/>
            <circle cx="10.5" cy="6.5" r="1" fill="currentColor"/>
            <path d="M5 10c.5 1 1.5 1.5 3 1.5s2.5-.5 3-1.5" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
          </svg>
        </button>
        <input 
          type="text" 
          v-model="profile.status_message"
          placeholder="您的状态是什么？"
          maxlength="100"
          class="status-input"
        />
        <button v-if="profile.status_message" class="clear-btn" @click="profile.status_message = ''">
          <svg viewBox="0 0 16 16" width="16" height="16" fill="none">
            <path d="M4 4l8 8M12 4l-8 8" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          </svg>
        </button>
      </div>
      <p class="char-count">剩余 {{ 100 - (profile.status_message?.length || 0) }} 个字符。</p>

      <div class="busy-checkbox">
        <label class="checkbox-label">
          <input type="checkbox" v-model="profile.busy" />
          <span class="checkbox-custom"></span>
          <div class="checkbox-text">
            <span class="checkbox-title">设置自己为忙碌中</span>
            <span class="checkbox-desc">显示您正忙或无法响应</span>
          </div>
        </label>
      </div>

      <div class="clear-status">
        <label>清除状态</label>
        <select v-model="profile.clear_status_after" class="form-select">
          <option value="never">从不</option>
          <option value="30m">30 分钟后</option>
          <option value="1h">1 小时后</option>
          <option value="4h">4 小时后</option>
          <option value="today">今天</option>
          <option value="1w">1 周后</option>
        </select>
      </div>
    </section>

    <hr class="divider" />

    <!-- 表单按钮 -->
    <div class="form-actions">
      <button type="button" class="btn btn-primary" @click="saveProfile" :disabled="saving">
        {{ saving ? '保存中...' : '更新个人资料设置' }}
      </button>
      <button type="button" class="btn btn-secondary" @click="resetForm">
        取消
      </button>
    </div>

    <div v-if="message" :class="['alert', messageType === 'success' ? 'alert-success' : 'alert-error']">
      {{ message }}
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { useAuthStore } from '@/stores/auth'
import apiClient from '@/api'

const authStore = useAuthStore()
const searchQuery = ref('')
const avatarFileName = ref('')
const showEmojiPicker = ref(false)

const profile = reactive({
  username: '',
  display_name: '',
  email: '',
  bio: '',
  avatar_url: '',
  status_emoji: '',
  status_message: '',
  busy: false,
  clear_status_after: 'never'
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

const handleAvatarChange = (event: Event) => {
  const target = event.target as HTMLInputElement
  if (target.files && target.files[0]) {
    avatarFileName.value = target.files[0].name
    // Handle avatar upload
  }
}

const saveProfile = async () => {
  saving.value = true
  message.value = ''

  try {
    await apiClient.client.put('/user/profile', {
      display_name: profile.display_name,
      email: profile.email,
      bio: profile.bio,
      status_message: profile.status_message,
      busy: profile.busy
    })

    message.value = '个人资料已更新'
    messageType.value = 'success'
    await authStore.fetchCurrentUser()
  } catch (error: any) {
    message.value = error.response?.data?.message || error.message || '保存失败'
    messageType.value = 'error'
  } finally {
    saving.value = false
  }
}

const resetForm = () => {
  loadProfile()
}

onMounted(() => {
  loadProfile()
})
</script>

<style lang="scss" scoped>
.profile-page {
  padding: 24px 40px;
  max-width: 1000px;
  background: #fff;
  min-height: 100vh;
}

.breadcrumb {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
  margin-bottom: 24px;
  color: #737278;
  
  a {
    color: #1f75cb;
    text-decoration: none;
    
    &:hover {
      text-decoration: underline;
    }
  }
  
  .separator {
    color: #737278;
  }
  
  span:last-child {
    color: #303030;
  }
}

.search-box {
  position: relative;
  margin-bottom: 32px;
  
  .search-icon {
    position: absolute;
    left: 12px;
    top: 50%;
    transform: translateY(-50%);
    color: #737278;
  }
  
  input {
    width: 100%;
    padding: 10px 12px 10px 40px;
    font-size: 14px;
    color: #303030;
    background: #fff;
    border: 1px solid #dcdcde;
    border-radius: 4px;
    
    &:focus {
      outline: none;
      border-color: #1f75cb;
      box-shadow: 0 0 0 3px rgba(31, 117, 203, 0.15);
    }
    
    &::placeholder {
      color: #737278;
    }
  }
}

.profile-section {
  margin-bottom: 32px;
  
  h2 {
    font-size: 20px;
    font-weight: 600;
    color: #303030;
    margin: 0 0 8px 0;
  }
  
  .section-description {
    font-size: 14px;
    color: #737278;
    margin: 0 0 20px 0;
    
    a {
      color: #1f75cb;
      text-decoration: none;
      
      &:hover {
        text-decoration: underline;
      }
    }
  }
}

.avatar-upload {
  display: flex;
  gap: 24px;
  align-items: flex-start;
}

.avatar-preview {
  width: 96px;
  height: 96px;
  border-radius: 50%;
  background: #f0f0f2;
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
  flex-shrink: 0;
  
  img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  
  .avatar-placeholder {
    color: #868686;
  }
}

.avatar-actions {
  h3 {
    font-size: 14px;
    font-weight: 600;
    color: #303030;
    margin: 0 0 12px 0;
  }
  
  .upload-hint {
    font-size: 12px;
    color: #737278;
    margin: 8px 0 0 0;
  }
}

.file-input-wrapper {
  display: flex;
  align-items: center;
  gap: 12px;
  
  .file-input-btn {
    display: inline-flex;
    align-items: center;
    padding: 8px 16px;
    font-size: 14px;
    color: #303030;
    background: #fff;
    border: 1px solid #dcdcde;
    border-radius: 4px;
    cursor: pointer;
    
    &:hover {
      background: #f0f0f2;
    }
    
    input {
      display: none;
    }
  }
  
  .file-name {
    font-size: 14px;
    color: #737278;
  }
}

.status-input-wrapper {
  display: flex;
  align-items: center;
  border: 1px solid #dcdcde;
  border-radius: 4px;
  background: #fff;
  
  &:focus-within {
    border-color: #1f75cb;
    box-shadow: 0 0 0 3px rgba(31, 117, 203, 0.15);
  }
}

.emoji-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 40px;
  height: 40px;
  background: none;
  border: none;
  cursor: pointer;
  color: #737278;
  
  &:hover {
    color: #303030;
  }
}

.status-input {
  flex: 1;
  padding: 10px 0;
  font-size: 14px;
  color: #303030;
  background: none;
  border: none;
  
  &:focus {
    outline: none;
  }
  
  &::placeholder {
    color: #737278;
  }
}

.clear-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 40px;
  height: 40px;
  background: none;
  border: none;
  cursor: pointer;
  color: #737278;
  
  &:hover {
    color: #303030;
  }
}

.char-count {
  font-size: 12px;
  color: #737278;
  margin: 8px 0 16px 0;
}

.busy-checkbox {
  margin-bottom: 16px;
  
  .checkbox-label {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    cursor: pointer;
    
    input[type="checkbox"] {
      display: none;
    }
    
    .checkbox-custom {
      width: 16px;
      height: 16px;
      border: 2px solid #868686;
      border-radius: 3px;
      flex-shrink: 0;
      margin-top: 2px;
      position: relative;
      
      &::after {
        content: '';
        position: absolute;
        top: 1px;
        left: 4px;
        width: 4px;
        height: 8px;
        border: solid #fff;
        border-width: 0 2px 2px 0;
        transform: rotate(45deg);
        opacity: 0;
      }
    }
    
    input[type="checkbox"]:checked + .checkbox-custom {
      background: #1f75cb;
      border-color: #1f75cb;
      
      &::after {
        opacity: 1;
      }
    }
    
    .checkbox-text {
      display: flex;
      flex-direction: column;
      gap: 2px;
    }
    
    .checkbox-title {
      font-size: 14px;
      color: #303030;
    }
    
    .checkbox-desc {
      font-size: 12px;
      color: #737278;
    }
  }
}

.clear-status {
  display: flex;
  flex-direction: column;
  gap: 8px;
  
  label {
    font-size: 14px;
    font-weight: 600;
    color: #303030;
  }
}

.form-select {
  width: 200px;
  padding: 8px 12px;
  font-size: 14px;
  color: #303030;
  background: #fff;
  border: 1px solid #dcdcde;
  border-radius: 4px;
  cursor: pointer;
  
  &:focus {
    outline: none;
    border-color: #1f75cb;
    box-shadow: 0 0 0 3px rgba(31, 117, 203, 0.15);
  }
}

.divider {
  border: none;
  border-top: 1px solid #dcdcde;
  margin: 32px 0;
}

.form-actions {
  display: flex;
  gap: 12px;
}

.btn {
  padding: 10px 16px;
  font-size: 14px;
  font-weight: 500;
  border-radius: 4px;
  cursor: pointer;
  border: none;
  
  &.btn-primary {
    background: #1f75cb;
    color: white;
    
    &:hover:not(:disabled) {
      background: #1068bf;
    }
    
    &:disabled {
      opacity: 0.6;
      cursor: not-allowed;
    }
  }
  
  &.btn-secondary {
    background: #fff;
    color: #303030;
    border: 1px solid #dcdcde;
    
    &:hover {
      background: #f0f0f2;
    }
  }
}

.alert {
  margin-top: 16px;
  padding: 12px 16px;
  border-radius: 4px;
  font-size: 14px;
  
  &.alert-success {
    background: #ecf4ee;
    border: 1px solid #108548;
    color: #108548;
  }
  
  &.alert-error {
    background: #fcf1ef;
    border: 1px solid #dd2b0e;
    color: #dd2b0e;
  }
}
</style>
