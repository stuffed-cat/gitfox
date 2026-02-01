<template>
  <div class="settings-page">
    <!-- 面包屑导航 -->
    <div class="breadcrumb">
      <router-link to="/-/profile">用户设置</router-link>
      <span class="separator">/</span>
      <span>偏好设置</span>
    </div>

    <!-- 页面标题 -->
    <div class="page-header">
      <h1>偏好设置</h1>
      <p class="description">自定义您的 GitFox 体验</p>
    </div>

    <div class="settings-content">

      <form @submit.prevent="savePreferences" class="preferences-form">
        <div class="form-section">
          <h3>外观</h3>
          
          <div class="form-group">
            <label>主题</label>
            <div class="theme-options">
              <label class="theme-option" :class="{ active: preferences.theme === 'light' }">
                <input type="radio" v-model="preferences.theme" value="light" />
                <div class="theme-preview light-theme">
                  <div class="preview-header"></div>
                  <div class="preview-content"></div>
                </div>
                <span>浅色</span>
              </label>
              <label class="theme-option" :class="{ active: preferences.theme === 'dark' }">
                <input type="radio" v-model="preferences.theme" value="dark" />
                <div class="theme-preview dark-theme">
                  <div class="preview-header"></div>
                  <div class="preview-content"></div>
                </div>
                <span>深色</span>
              </label>
              <label class="theme-option" :class="{ active: preferences.theme === 'system' }">
                <input type="radio" v-model="preferences.theme" value="system" />
                <div class="theme-preview system-theme">
                  <div class="preview-header"></div>
                  <div class="preview-content"></div>
                </div>
                <span>跟随系统</span>
              </label>
            </div>
          </div>
        </div>

        <div class="form-section">
          <h3>语言与地区</h3>
          
          <div class="form-group">
            <label for="language">语言</label>
            <select id="language" v-model="preferences.language" class="form-control">
              <option value="zh-CN">简体中文</option>
              <option value="en">English</option>
            </select>
          </div>
          
          <div class="form-group">
            <label for="timezone">时区</label>
            <select id="timezone" v-model="preferences.timezone" class="form-control">
              <option value="Asia/Shanghai">中国标准时间 (UTC+8)</option>
              <option value="UTC">协调世界时 (UTC)</option>
              <option value="America/New_York">美国东部时间 (UTC-5)</option>
              <option value="Europe/London">格林威治时间 (UTC+0)</option>
            </select>
          </div>
        </div>

        <div class="form-section">
          <h3>通知</h3>
          
          <div class="form-group checkbox-group">
            <label class="checkbox-label">
              <input type="checkbox" v-model="preferences.notifications.email" />
              <span>邮件通知</span>
            </label>
            <small class="form-text">接收项目更新、评论和合并请求的邮件通知</small>
          </div>
          
          <div class="form-group checkbox-group">
            <label class="checkbox-label">
              <input type="checkbox" v-model="preferences.notifications.web" />
              <span>网页通知</span>
            </label>
            <small class="form-text">在浏览器中接收实时通知</small>
          </div>
        </div>

        <div class="form-section">
          <h3>代码编辑器</h3>
          
          <div class="form-group">
            <label for="tabSize">Tab 大小</label>
            <select id="tabSize" v-model="preferences.editor.tabSize" class="form-control">
              <option :value="2">2 空格</option>
              <option :value="4">4 空格</option>
              <option :value="8">8 空格</option>
            </select>
          </div>
          
          <div class="form-group checkbox-group">
            <label class="checkbox-label">
              <input type="checkbox" v-model="preferences.editor.lineNumbers" />
              <span>显示行号</span>
            </label>
          </div>
          
          <div class="form-group checkbox-group">
            <label class="checkbox-label">
              <input type="checkbox" v-model="preferences.editor.wordWrap" />
              <span>自动换行</span>
            </label>
          </div>
        </div>

        <div class="form-actions">
          <button type="submit" class="btn btn-primary" :disabled="saving">
            {{ saving ? '保存中...' : '保存更改' }}
          </button>
        </div>

        <div v-if="message" :class="['alert', messageType === 'success' ? 'alert-success' : 'alert-danger']">
          {{ message }}
        </div>
      </form>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive } from 'vue'

const preferences = reactive({
  theme: 'light',
  language: 'zh-CN',
  timezone: 'Asia/Shanghai',
  notifications: {
    email: true,
    web: true
  },
  editor: {
    tabSize: 4,
    lineNumbers: true,
    wordWrap: false
  }
})

const saving = ref(false)
const message = ref('')
const messageType = ref<'success' | 'error'>('success')

const savePreferences = async () => {
  saving.value = true
  message.value = ''

  try {
    // Save to localStorage for now
    localStorage.setItem('user-preferences', JSON.stringify(preferences))
    
    message.value = '偏好设置已保存'
    messageType.value = 'success'
  } catch (error: any) {
    message.value = error.message || '保存失败'
    messageType.value = 'error'
  } finally {
    saving.value = false
  }
}
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

.settings-content {
  background: var(--bg-secondary, #161b22);
  border: 1px solid var(--border-color, #30363d);
  border-radius: 6px;
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

.preferences-form {
  .form-section {
    margin-bottom: 32px;
    padding: 24px;
    background: var(--bg-secondary, #161b22);
    border-radius: 8px;
    border: 1px solid var(--border-color, #30363d);
    
    h3 {
      font-size: 16px;
      font-weight: 600;
      margin: 0 0 20px 0;
      padding-bottom: 12px;
      border-bottom: 1px solid var(--border-color, #30363d);
      color: var(--text-primary, #c9d1d9);
    }
  }
}

.form-group {
  margin-bottom: 20px;
  
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
  
  .form-control {
    width: 100%;
    max-width: 300px;
    padding: 10px 12px;
    font-size: 14px;
    color: var(--text-primary, #c9d1d9);
    background: var(--bg-primary, #0d1117);
    border: 1px solid var(--border-color, #30363d);
    border-radius: 6px;
    
    &:focus {
      outline: none;
      border-color: var(--color-primary, #58a6ff);
      box-shadow: 0 0 0 3px rgba(88, 166, 255, 0.15);
    }
  }
  
  .form-text {
    display: block;
    margin-top: 6px;
    font-size: 12px;
    color: var(--text-secondary, #8b949e);
  }
}

.checkbox-group {
  .checkbox-label {
    display: flex;
    align-items: center;
    gap: 10px;
    cursor: pointer;
    
    input[type="checkbox"] {
      width: 18px;
      height: 18px;
      cursor: pointer;
    }
    
    span {
      font-weight: 400;
    }
  }
}

.theme-options {
  display: flex;
  gap: 16px;
  margin-top: 12px;
}

.theme-option {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 10px;
  cursor: pointer;
  
  input[type="radio"] {
    display: none;
  }
  
  .theme-preview {
    width: 80px;
    height: 60px;
    border-radius: 8px;
    border: 2px solid transparent;
    overflow: hidden;
    transition: border-color 0.2s;
    
    .preview-header {
      height: 12px;
    }
    
    .preview-content {
      height: 48px;
    }
  }
  
  &.active .theme-preview {
    border-color: var(--color-primary, #58a6ff);
  }
  
  .dark-theme {
    .preview-header { background: #161b22; }
    .preview-content { background: #0d1117; }
  }
  
  .light-theme {
    .preview-header { background: #f6f8fa; }
    .preview-content { background: #ffffff; }
  }
  
  .system-theme {
    .preview-header {
      background: linear-gradient(90deg, #161b22 50%, #f6f8fa 50%);
    }
    .preview-content {
      background: linear-gradient(90deg, #0d1117 50%, #ffffff 50%);
    }
  }
  
  span {
    font-size: 13px;
    color: var(--text-secondary, #8b949e);
  }
  
  &.active span {
    color: var(--text-primary, #c9d1d9);
    font-weight: 500;
  }
}

.form-actions {
  margin-top: 24px;
  
  .btn {
    padding: 10px 20px;
    font-size: 14px;
    font-weight: 500;
    border-radius: 6px;
    cursor: pointer;
    
    &.btn-primary {
      background: var(--color-primary, #238636);
      border: 1px solid var(--color-primary, #238636);
      color: white;
      
      &:hover:not(:disabled) {
        background: var(--color-primary-hover, #2ea043);
      }
      
      &:disabled {
        opacity: 0.6;
        cursor: not-allowed;
      }
    }
  }
}

.alert {
  margin-top: 16px;
  padding: 12px 16px;
  border-radius: 6px;
  font-size: 14px;
  
  &.alert-success {
    background: rgba(35, 134, 54, 0.15);
    border: 1px solid #238636;
    color: #3fb950;
  }
  
  &.alert-danger {
    background: rgba(248, 81, 73, 0.15);
    border: 1px solid #f85149;
    color: #f85149;
  }
}
</style>
