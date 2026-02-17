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
          <h2>重置密码</h2>
        </div>
        
        <!-- Loading State -->
        <div v-if="verifying" class="loading-state">
          <div class="spinner"></div>
          <p>正在验证重置链接...</p>
        </div>
        
        <!-- Invalid Token -->
        <div v-else-if="!tokenValid" class="result-state error">
          <svg class="result-icon" width="64" height="64" viewBox="0 0 64 64" fill="none">
            <circle cx="32" cy="32" r="30" stroke="currentColor" stroke-width="3"/>
            <path d="M24 24l16 16M40 24L24 40" stroke="currentColor" stroke-width="3" stroke-linecap="round"/>
          </svg>
          <h3>链接无效</h3>
          <p>{{ tokenError || '密码重置链接无效或已过期' }}</p>
          <router-link to="/forgot-password" class="btn btn-primary btn-lg">
            重新获取重置链接
          </router-link>
        </div>
        
        <!-- Success State -->
        <div v-else-if="success" class="result-state success">
          <svg class="result-icon" width="64" height="64" viewBox="0 0 64 64" fill="none">
            <circle cx="32" cy="32" r="30" stroke="currentColor" stroke-width="3"/>
            <path d="M20 32l10 10 14-20" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
          <h3>密码重置成功!</h3>
          <p>您的密码已更新，现在可以使用新密码登录。</p>
          <router-link to="/login" class="btn btn-primary btn-lg">
            前往登录
          </router-link>
        </div>
        
        <!-- Reset Form -->
        <form v-else @submit.prevent="handleSubmit" class="auth-form">
          <p class="form-intro">
            您好 <strong>{{ username }}</strong>，请设置您的新密码。
          </p>
          
          <div v-if="error" class="alert alert-danger">
            <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
              <circle cx="8" cy="8" r="7" stroke="currentColor" stroke-width="1.5"/>
              <path d="M8 5v4M8 11v.01" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
            </svg>
            <span>{{ error }}</span>
          </div>
          
          <div class="form-group">
            <label class="form-label" for="password">新密码</label>
            <div class="input-wrapper">
              <svg class="input-icon" width="16" height="16" viewBox="0 0 16 16" fill="none">
                <rect x="3" y="7" width="10" height="7" rx="1" stroke="currentColor" stroke-width="1.5"/>
                <path d="M5 7V5a3 3 0 016 0v2" stroke="currentColor" stroke-width="1.5"/>
              </svg>
              <input
                id="password"
                v-model="password"
                :type="showPassword ? 'text' : 'password'"
                class="form-control with-icon"
                placeholder="请输入新密码（至少8位）"
                required
                minlength="8"
                autocomplete="new-password"
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
            <label class="form-label" for="confirmPassword">确认新密码</label>
            <div class="input-wrapper">
              <svg class="input-icon" width="16" height="16" viewBox="0 0 16 16" fill="none">
                <rect x="3" y="7" width="10" height="7" rx="1" stroke="currentColor" stroke-width="1.5"/>
                <path d="M5 7V5a3 3 0 016 0v2" stroke="currentColor" stroke-width="1.5"/>
              </svg>
              <input
                id="confirmPassword"
                v-model="confirmPassword"
                :type="showPassword ? 'text' : 'password'"
                class="form-control with-icon"
                placeholder="请再次输入新密码"
                required
                autocomplete="new-password"
              />
            </div>
          </div>
          
          <button type="submit" class="btn btn-primary btn-lg w-full" :disabled="loading || !isValid">
            <span v-if="loading" class="spinner-sm"></span>
            {{ loading ? '重置中...' : '重置密码' }}
          </button>
        </form>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import api from '@/api'

const route = useRoute()

const verifying = ref(true)
const tokenValid = ref(false)
const tokenError = ref('')
const username = ref('')
const token = ref('')

const password = ref('')
const confirmPassword = ref('')
const showPassword = ref(false)
const loading = ref(false)
const error = ref('')
const success = ref(false)

const isValid = computed(() => {
  return password.value.length >= 8 && password.value === confirmPassword.value
})

onMounted(async () => {
  token.value = route.query.token as string
  
  if (!token.value) {
    verifying.value = false
    tokenError.value = '缺少重置令牌'
    return
  }
  
  try {
    const result = await api.auth.verifyResetToken(token.value)
    tokenValid.value = result.valid
    username.value = result.username
  } catch (err: any) {
    tokenError.value = err.response?.data?.error || '链接无效或已过期'
  } finally {
    verifying.value = false
  }
})

async function handleSubmit() {
  error.value = ''
  
  if (password.value !== confirmPassword.value) {
    error.value = '两次输入的密码不一致'
    return
  }
  
  if (password.value.length < 8) {
    error.value = '密码至少需要8位'
    return
  }
  
  loading.value = true
  
  try {
    await api.auth.resetPassword(token.value, password.value)
    success.value = true
  } catch (err: any) {
    error.value = err.response?.data?.error || '重置失败，请稍后重试'
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
  background: $bg-secondary;
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
    margin: 0 auto $spacing-3;
    background: linear-gradient(135deg, $brand-primary 0%, $brand-secondary 100%);
    border-radius: $border-radius-xl;
    display: flex;
    align-items: center;
    justify-content: center;
    color: white;
  }
  
  .logo-text {
    font-size: $font-size-2xl;
    font-weight: $font-weight-bold;
    color: $text-primary;
    margin: 0;
  }
}

.auth-card {
  background: $bg-primary;
  border-radius: $border-radius-xl;
  padding: $spacing-8;
  box-shadow: $shadow-lg;
}

.auth-header {
  text-align: center;
  margin-bottom: $spacing-6;
  
  h2 {
    font-size: $font-size-xl;
    font-weight: $font-weight-semibold;
    color: $text-primary;
    margin: 0;
  }
}

.loading-state {
  text-align: center;
  padding: $spacing-8 0;
  
  .spinner {
    width: 48px;
    height: 48px;
    border: 3px solid $border-color;
    border-top-color: $brand-primary;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
    margin: 0 auto $spacing-4;
  }
  
  p {
    color: $text-secondary;
    margin: 0;
  }
}

.result-state {
  text-align: center;
  padding: $spacing-4 0;
  
  .result-icon {
    margin-bottom: $spacing-4;
  }
  
  h3 {
    font-size: $font-size-lg;
    font-weight: $font-weight-semibold;
    margin: 0 0 $spacing-2;
  }
  
  p {
    color: $text-secondary;
    margin: 0 0 $spacing-6;
  }
  
  &.success {
    .result-icon { color: $color-success; }
    h3 { color: $color-success; }
  }
  
  &.error {
    .result-icon { color: $color-danger; }
    h3 { color: $color-danger; }
  }
}

.form-intro {
  text-align: center;
  color: $text-secondary;
  margin-bottom: $spacing-4;
  
  strong {
    color: $text-primary;
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
  
  .input-wrapper {
    position: relative;
  }
  
  .input-icon {
    position: absolute;
    left: $spacing-3;
    top: 50%;
    transform: translateY(-50%);
    color: $text-muted;
  }
  
  .form-control {
    width: 100%;
    padding: $spacing-3;
    border: 1px solid $border-color;
    border-radius: $border-radius;
    font-size: $font-size-base;
    color: $text-primary;
    background: $bg-primary;
    transition: border-color $transition-fast;
    box-sizing: border-box;
    
    &.with-icon {
      padding-left: $spacing-10;
      padding-right: $spacing-10;
    }
    
    &:focus {
      outline: none;
      border-color: $brand-primary;
      box-shadow: $shadow-focus;
    }
    
    &::placeholder {
      color: $text-muted;
    }
  }
  
  .toggle-password {
    position: absolute;
    right: $spacing-3;
    top: 50%;
    transform: translateY(-50%);
    background: none;
    border: none;
    color: $text-muted;
    cursor: pointer;
    padding: 0;
    
    &:hover {
      color: $text-secondary;
    }
  }
}

.alert {
  display: flex;
  align-items: center;
  gap: $spacing-2;
  padding: $spacing-3;
  border-radius: $border-radius;
  font-size: $font-size-sm;
  margin-bottom: $spacing-4;
  
  &.alert-danger {
    background: rgba($color-danger, 0.1);
    color: $color-danger;
    border: 1px solid rgba($color-danger, 0.2);
  }
}

.btn {
  padding: $spacing-3 $spacing-6;
  border-radius: $border-radius;
  font-size: $font-size-base;
  font-weight: $font-weight-medium;
  cursor: pointer;
  border: 1px solid transparent;
  text-decoration: none;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: $spacing-2;
  transition: all $transition-fast;
  
  &:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }
}

.btn-primary {
  background: $brand-primary;
  color: white;
  &:hover:not(:disabled) { background: $primary-dark; }
}

.btn-lg {
  padding: $spacing-3 $spacing-8;
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
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}
</style>
