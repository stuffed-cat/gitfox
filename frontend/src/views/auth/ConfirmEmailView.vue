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
          <h2>确认邮箱</h2>
        </div>
        
        <div v-if="loading" class="loading-state">
          <div class="spinner"></div>
          <p>正在确认您的邮箱...</p>
        </div>
        
        <div v-else-if="success" class="result-state success">
          <svg class="result-icon" width="64" height="64" viewBox="0 0 64 64" fill="none">
            <circle cx="32" cy="32" r="30" stroke="currentColor" stroke-width="3"/>
            <path d="M20 32l10 10 14-20" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
          <h3>邮箱确认成功!</h3>
          <p>您的邮箱已成功确认，现在可以登录了。</p>
          <router-link to="/login" class="btn btn-primary btn-lg">
            前往登录
          </router-link>
        </div>
        
        <div v-else class="result-state error">
          <svg class="result-icon" width="64" height="64" viewBox="0 0 64 64" fill="none">
            <circle cx="32" cy="32" r="30" stroke="currentColor" stroke-width="3"/>
            <path d="M24 24l16 16M40 24L24 40" stroke="currentColor" stroke-width="3" stroke-linecap="round"/>
          </svg>
          <h3>确认失败</h3>
          <p>{{ error || '无效或已过期的确认链接' }}</p>
          <div class="action-buttons">
            <router-link to="/login" class="btn btn-secondary">
              前往登录
            </router-link>
            <router-link to="/register" class="btn btn-primary">
              重新注册
            </router-link>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import api from '@/api'

const route = useRoute()

const loading = ref(true)
const success = ref(false)
const error = ref('')

onMounted(async () => {
  const token = route.query.token as string
  
  if (!token) {
    loading.value = false
    error.value = '缺少确认令牌'
    return
  }
  
  try {
    await api.auth.confirmEmail(token)
    success.value = true
  } catch (err: any) {
    error.value = err.response?.data?.error || '确认失败，请重试'
  } finally {
    loading.value = false
  }
})
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
    .result-icon {
      color: $color-success;
    }
    h3 {
      color: $color-success;
    }
  }
  
  &.error {
    .result-icon {
      color: $color-danger;
    }
    h3 {
      color: $color-danger;
    }
  }
}

.action-buttons {
  display: flex;
  gap: $spacing-3;
  justify-content: center;
}

.btn {
  padding: $spacing-3 $spacing-6;
  border-radius: $border-radius;
  font-size: $font-size-base;
  font-weight: $font-weight-medium;
  cursor: pointer;
  border: 1px solid transparent;
  text-decoration: none;
  display: inline-block;
  transition: all $transition-fast;
}

.btn-primary {
  background: $brand-primary;
  color: white;
  &:hover { background: $primary-dark; }
}

.btn-secondary {
  background: $bg-secondary;
  color: $text-primary;
  border-color: $border-color;
  &:hover { background: $bg-tertiary; }
}

.btn-lg {
  padding: $spacing-3 $spacing-8;
  font-size: $font-size-base;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}
</style>
