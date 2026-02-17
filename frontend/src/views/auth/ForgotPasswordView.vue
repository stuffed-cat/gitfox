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
          <h2>忘记密码</h2>
          <p class="subtitle">输入您的邮箱地址，我们将发送密码重置链接。</p>
        </div>
        
        <div v-if="submitted" class="result-state success">
          <svg class="result-icon" width="64" height="64" viewBox="0 0 64 64" fill="none">
            <circle cx="32" cy="32" r="30" stroke="currentColor" stroke-width="3"/>
            <path d="M20 32l10 10 14-20" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
          <h3>邮件已发送</h3>
          <p>如果该邮箱已注册账户，您将收到密码重置链接。</p>
          <p class="hint">请检查您的收件箱和垃圾邮件文件夹。</p>
          <router-link to="/login" class="btn btn-primary btn-lg">
            返回登录
          </router-link>
        </div>
        
        <form v-else @submit.prevent="handleSubmit" class="auth-form">
          <div v-if="error" class="alert alert-danger">
            <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
              <circle cx="8" cy="8" r="7" stroke="currentColor" stroke-width="1.5"/>
              <path d="M8 5v4M8 11v.01" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
            </svg>
            <span>{{ error }}</span>
          </div>
          
          <div class="form-group">
            <label class="form-label" for="email">邮箱地址</label>
            <div class="input-wrapper">
              <svg class="input-icon" width="16" height="16" viewBox="0 0 16 16" fill="none">
                <rect x="1" y="3" width="14" height="10" rx="2" stroke="currentColor" stroke-width="1.5"/>
                <path d="M1 5l7 4 7-4" stroke="currentColor" stroke-width="1.5"/>
              </svg>
              <input
                id="email"
                v-model="email"
                type="email"
                class="form-control with-icon"
                placeholder="请输入注册时使用的邮箱"
                required
                autocomplete="email"
              />
            </div>
          </div>
          
          <button type="submit" class="btn btn-primary btn-lg w-full" :disabled="loading">
            <span v-if="loading" class="spinner-sm"></span>
            {{ loading ? '发送中...' : '发送重置链接' }}
          </button>
        </form>
      </div>
      
      <div class="auth-footer">
        <router-link to="/login">← 返回登录</router-link>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import api from '@/api'

const email = ref('')
const loading = ref(false)
const error = ref('')
const submitted = ref(false)

async function handleSubmit() {
  error.value = ''
  loading.value = true
  
  try {
    await api.auth.forgotPassword(email.value)
    submitted.value = true
  } catch (err: any) {
    error.value = err.response?.data?.error || '请求失败，请稍后重试'
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
    margin: 0 0 $spacing-2;
  }
  
  .subtitle {
    color: $text-secondary;
    font-size: $font-size-sm;
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
    margin: 0 0 $spacing-2;
  }
  
  .hint {
    font-size: $font-size-sm;
    margin-bottom: $spacing-6;
  }
  
  &.success {
    .result-icon {
      color: $color-success;
    }
    h3 {
      color: $color-success;
    }
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
  font-size: $font-size-base;
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

.auth-footer {
  text-align: center;
  margin-top: $spacing-6;
  font-size: $font-size-sm;
  color: $text-secondary;
  
  a {
    color: $brand-primary;
    text-decoration: none;
    
    &:hover {
      text-decoration: underline;
    }
  }
}

@keyframes spin {
  to { transform: rotate(360deg); }
}
</style>
