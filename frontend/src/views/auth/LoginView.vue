<template>
  <div class="auth-page">
    <div class="auth-container">
      <!-- Logo Section -->
      <div class="auth-logo">
        <div class="logo-icon">
          <svg width="48" height="48" viewBox="0 0 28 28" fill="none">
            <path d="M14 0L17.5 10.5H28L19.5 17L23 28L14 21.5L5 28L8.5 17L0 10.5H10.5L14 0Z" fill="currentColor"/>
          </svg>
        </div>
        <h1 class="logo-text">DevOps</h1>
      </div>
      
      <!-- Auth Card -->
      <div class="auth-card">
        <div class="auth-header">
          <h2>登录到 DevOps</h2>
        </div>
        
        <form @submit.prevent="handleSubmit" class="auth-form">
          <div v-if="error" class="alert alert-danger">
            <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
              <circle cx="8" cy="8" r="7" stroke="currentColor" stroke-width="1.5"/>
              <path d="M8 5v4M8 11v.01" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
            </svg>
            <span>{{ error }}</span>
          </div>
          
          <div class="form-group">
            <label class="form-label" for="username">用户名或邮箱</label>
            <div class="input-wrapper">
              <svg class="input-icon" width="16" height="16" viewBox="0 0 16 16" fill="none">
                <circle cx="8" cy="5" r="3" stroke="currentColor" stroke-width="1.5"/>
                <path d="M2 14a6 6 0 0112 0" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
              </svg>
              <input
                id="username"
                v-model="form.username"
                type="text"
                class="form-control with-icon"
                placeholder="请输入用户名或邮箱"
                required
                autocomplete="username"
              />
            </div>
          </div>
          
          <div class="form-group">
            <div class="label-row">
              <label class="form-label" for="password">密码</label>
              <a href="#" class="forgot-link">忘记密码？</a>
            </div>
            <div class="input-wrapper">
              <svg class="input-icon" width="16" height="16" viewBox="0 0 16 16" fill="none">
                <rect x="3" y="7" width="10" height="7" rx="1" stroke="currentColor" stroke-width="1.5"/>
                <path d="M5 7V5a3 3 0 016 0v2" stroke="currentColor" stroke-width="1.5"/>
              </svg>
              <input
                id="password"
                v-model="form.password"
                :type="showPassword ? 'text' : 'password'"
                class="form-control with-icon"
                placeholder="请输入密码"
                required
                autocomplete="current-password"
              />
              <button type="button" class="toggle-password" @click="showPassword = !showPassword">
                <svg v-if="!showPassword" width="16" height="16" viewBox="0 0 16 16" fill="none">
                  <path d="M1 8s2.5-5 7-5 7 5 7 5-2.5 5-7 5-7-5-7-5z" stroke="currentColor" stroke-width="1.5"/>
                  <circle cx="8" cy="8" r="2" stroke="currentColor" stroke-width="1.5"/>
                </svg>
                <svg v-else width="16" height="16" viewBox="0 0 16 16" fill="none">
                  <path d="M2 2l12 12M6.5 6.5A2 2 0 109.5 9.5M1 8s2.5-5 7-5c1.5 0 2.7.5 3.7 1.2M15 8s-1.2 2.5-3.3 3.8" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
                </svg>
              </button>
            </div>
          </div>
          
          <div class="form-group">
            <label class="checkbox-wrapper">
              <input type="checkbox" v-model="rememberMe" />
              <span class="checkbox-label">记住我</span>
            </label>
          </div>
          
          <button type="submit" class="btn btn-primary btn-lg w-full" :disabled="loading">
            <span v-if="loading" class="spinner-sm"></span>
            {{ loading ? '登录中...' : '登录' }}
          </button>
        </form>
        
        <div class="auth-divider">
          <span>或</span>
        </div>
        
        <div class="social-login">
          <button type="button" class="btn btn-social">
            <svg width="18" height="18" viewBox="0 0 18 18" fill="none">
              <path d="M9 0C4.03 0 0 4.03 0 9c0 3.98 2.58 7.35 6.16 8.54.45.08.62-.2.62-.43v-1.52c-2.5.54-3.03-1.2-3.03-1.2-.41-1.04-1-1.32-1-1.32-.82-.56.06-.55.06-.55.9.06 1.38.93 1.38.93.8 1.37 2.1.98 2.62.75.08-.58.31-.98.57-1.2-2-.23-4.1-1-4.1-4.45 0-.98.35-1.78.93-2.41-.09-.23-.4-1.14.09-2.38 0 0 .76-.24 2.48.92a8.64 8.64 0 014.52 0c1.72-1.16 2.48-.92 2.48-.92.49 1.24.18 2.15.09 2.38.58.63.92 1.43.92 2.41 0 3.46-2.1 4.22-4.11 4.44.32.28.61.83.61 1.67v2.47c0 .24.16.52.62.43A9 9 0 0018 9c0-4.97-4.03-9-9-9z" fill="currentColor"/>
            </svg>
            使用 GitHub 登录
          </button>
          <button type="button" class="btn btn-social">
            <svg width="18" height="18" viewBox="0 0 18 18" fill="none">
              <path d="M17.64 9.2c0-.64-.06-1.25-.16-1.84H9v3.48h4.84a4.14 4.14 0 01-1.8 2.72v2.26h2.91c1.7-1.57 2.69-3.88 2.69-6.62z" fill="#4285F4"/>
              <path d="M9 18c2.43 0 4.47-.8 5.96-2.18l-2.91-2.26c-.81.54-1.84.86-3.05.86-2.35 0-4.33-1.58-5.04-3.72H.96v2.33A9 9 0 009 18z" fill="#34A853"/>
              <path d="M3.96 10.7A5.4 5.4 0 013.68 9c0-.59.1-1.16.28-1.7V4.97H.96A9 9 0 000 9c0 1.45.35 2.82.96 4.03l3-2.33z" fill="#FBBC05"/>
              <path d="M9 3.58c1.32 0 2.5.45 3.44 1.35l2.58-2.58C13.46.89 11.42 0 9 0A9 9 0 00.96 4.97l3 2.33C4.67 5.16 6.65 3.58 9 3.58z" fill="#EA4335"/>
            </svg>
            使用 Google 登录
          </button>
        </div>
      </div>
      
      <div class="auth-footer">
        还没有账户？<router-link to="/register">立即注册</router-link>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive } from 'vue'
import { useRouter } from 'vue-router'
import { useAuthStore } from '@/stores/auth'

const router = useRouter()
const authStore = useAuthStore()

const form = reactive({
  username: '',
  password: ''
})
const loading = ref(false)
const error = ref('')
const showPassword = ref(false)
const rememberMe = ref(false)

async function handleSubmit() {
  loading.value = true
  error.value = ''
  
  try {
    await authStore.login(form)
    router.push('/')
  } catch (e: any) {
    error.value = e.response?.data?.message || '登录失败，请检查用户名和密码'
  } finally {
    loading.value = false
  }
}
</script>

<style lang="scss" scoped>
.auth-page {
  min-height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(135deg, #1f2937 0%, #111827 100%);
  padding: $spacing-6;
}

.auth-container {
  width: 100%;
  max-width: 400px;
}

.auth-logo {
  text-align: center;
  margin-bottom: $spacing-8;
  
  .logo-icon {
    width: 64px;
    height: 64px;
    margin: 0 auto $spacing-4;
    color: #e24329;
  }
  
  .logo-text {
    font-size: $font-size-3xl;
    font-weight: $font-weight-bold;
    color: $text-light;
    margin: 0;
  }
}

.auth-card {
  background: $bg-primary;
  border-radius: $border-radius-xl;
  padding: $spacing-8;
  box-shadow: $shadow-xl;
}

.auth-header {
  text-align: center;
  margin-bottom: $spacing-6;
  
  h2 {
    font-size: $font-size-2xl;
    font-weight: $font-weight-semibold;
    color: $text-primary;
    margin: 0;
  }
}

.auth-form {
  .form-group {
    margin-bottom: $spacing-5;
  }
  
  .form-label {
    display: block;
    font-size: $font-size-sm;
    font-weight: $font-weight-medium;
    color: $text-primary;
    margin-bottom: $spacing-2;
  }
  
  .label-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: $spacing-2;
  }
  
  .forgot-link {
    font-size: $font-size-sm;
    color: $text-link;
    
    &:hover {
      text-decoration: underline;
    }
  }
}

.input-wrapper {
  position: relative;
}

.input-icon {
  position: absolute;
  left: $spacing-3;
  top: 50%;
  transform: translateY(-50%);
  color: $text-muted;
  pointer-events: none;
}

.form-control.with-icon {
  padding-left: 40px;
  padding-right: 40px;
}

.toggle-password {
  position: absolute;
  right: $spacing-3;
  top: 50%;
  transform: translateY(-50%);
  background: transparent;
  border: none;
  color: $text-muted;
  cursor: pointer;
  padding: $spacing-1;
  
  &:hover {
    color: $text-primary;
  }
}

.checkbox-wrapper {
  display: flex;
  align-items: center;
  gap: $spacing-2;
  cursor: pointer;
  
  input[type="checkbox"] {
    width: 16px;
    height: 16px;
    accent-color: $brand-primary;
  }
  
  .checkbox-label {
    font-size: $font-size-sm;
    color: $text-primary;
  }
}

.w-full {
  width: 100%;
}

.spinner-sm {
  width: 16px;
  height: 16px;
  border: 2px solid rgba(255, 255, 255, 0.3);
  border-top-color: white;
  border-radius: 50%;
  animation: spin 0.6s linear infinite;
  margin-right: $spacing-2;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.auth-divider {
  display: flex;
  align-items: center;
  margin: $spacing-6 0;
  
  &::before,
  &::after {
    content: '';
    flex: 1;
    height: 1px;
    background: $border-color;
  }
  
  span {
    padding: 0 $spacing-4;
    font-size: $font-size-sm;
    color: $text-muted;
  }
}

.social-login {
  display: flex;
  flex-direction: column;
  gap: $spacing-3;
}

.btn-social {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: $spacing-3;
  width: 100%;
  padding: $spacing-3 $spacing-4;
  font-size: $font-size-base;
  font-weight: $font-weight-medium;
  color: $text-primary;
  background: $bg-primary;
  border: 1px solid $border-color;
  border-radius: $border-radius;
  cursor: pointer;
  transition: all $transition-fast;
  
  &:hover {
    background: $bg-secondary;
    border-color: $border-color-dark;
  }
}

.auth-footer {
  text-align: center;
  margin-top: $spacing-6;
  color: $gray-400;
  font-size: $font-size-sm;
  
  a {
    color: $text-light;
    font-weight: $font-weight-medium;
    
    &:hover {
      text-decoration: underline;
    }
  }
}

.alert {
  display: flex;
  align-items: center;
  gap: $spacing-2;
  padding: $spacing-3 $spacing-4;
  margin-bottom: $spacing-5;
  border-radius: $border-radius;
  font-size: $font-size-sm;
  
  &-danger {
    background: $color-danger-light;
    color: darken($color-danger, 15%);
    border: 1px solid rgba($color-danger, 0.2);
  }
}
</style>

