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
          <h2>创建您的账户</h2>
          <p>开始您的 DevOps 之旅</p>
        </div>
        
        <form @submit.prevent="handleSubmit" class="auth-form">
          <div v-if="error" class="alert alert-danger">
            <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
              <circle cx="8" cy="8" r="7" stroke="currentColor" stroke-width="1.5"/>
              <path d="M8 5v4M8 11v.01" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
            </svg>
            <span>{{ error }}</span>
          </div>
          
          <div class="form-row">
            <div class="form-group">
              <label class="form-label" for="username">用户名 *</label>
              <input
                id="username"
                v-model="form.username"
                type="text"
                class="form-control"
                placeholder="用户名"
                required
                autocomplete="username"
              />
              <span class="form-hint">可用于登录和 @提及</span>
            </div>
            
            <div class="form-group">
              <label class="form-label" for="display_name">显示名称</label>
              <input
                id="display_name"
                v-model="form.display_name"
                type="text"
                class="form-control"
                placeholder="您的名字（可选）"
              />
            </div>
          </div>
          
          <div class="form-group">
            <label class="form-label" for="email">邮箱 *</label>
            <input
              id="email"
              v-model="form.email"
              type="email"
              class="form-control"
              placeholder="name@example.com"
              required
              autocomplete="email"
            />
          </div>
          
          <div class="form-group">
            <label class="form-label" for="password">密码 *</label>
            <div class="input-wrapper">
              <input
                id="password"
                v-model="form.password"
                :type="showPassword ? 'text' : 'password'"
                class="form-control"
                placeholder="至少 8 个字符"
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
            <div class="password-strength">
              <div class="strength-bar" :class="passwordStrength"></div>
              <span class="strength-text">{{ passwordStrengthText }}</span>
            </div>
          </div>
          
          <div class="form-group">
            <label class="form-label" for="confirm_password">确认密码 *</label>
            <input
              id="confirm_password"
              v-model="confirmPassword"
              type="password"
              class="form-control"
              placeholder="再次输入密码"
              required
              autocomplete="new-password"
            />
          </div>
          
          <div class="form-group">
            <label class="checkbox-wrapper">
              <input type="checkbox" v-model="agreeTerms" required />
              <span class="checkbox-label">我同意 <a href="#">服务条款</a> 和 <a href="#">隐私政策</a></span>
            </label>
          </div>
          
          <button type="submit" class="btn btn-primary btn-lg w-full" :disabled="loading || !agreeTerms">
            <span v-if="loading" class="spinner-sm"></span>
            {{ loading ? '注册中...' : '创建账户' }}
          </button>
        </form>
      </div>
      
      <div class="auth-footer">
        已有账户？<router-link to="/login">立即登录</router-link>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed } from 'vue'
import { useRouter } from 'vue-router'
import { useAuthStore } from '@/stores/auth'

const router = useRouter()
const authStore = useAuthStore()

const form = reactive({
  username: '',
  email: '',
  password: '',
  display_name: ''
})
const confirmPassword = ref('')
const loading = ref(false)
const error = ref('')
const showPassword = ref(false)
const agreeTerms = ref(false)

const passwordStrength = computed(() => {
  const pwd = form.password
  if (!pwd) return ''
  if (pwd.length < 8) return 'weak'
  if (pwd.length >= 12 && /[A-Z]/.test(pwd) && /[0-9]/.test(pwd) && /[^A-Za-z0-9]/.test(pwd)) return 'strong'
  if (pwd.length >= 8) return 'medium'
  return 'weak'
})

const passwordStrengthText = computed(() => {
  const map: Record<string, string> = {
    '': '',
    weak: '弱',
    medium: '中等',
    strong: '强'
  }
  return map[passwordStrength.value] || ''
})

async function handleSubmit() {
  if (form.password !== confirmPassword.value) {
    error.value = '两次输入的密码不一致'
    return
  }
  
  loading.value = true
  error.value = ''
  
  try {
    await authStore.register(form)
    router.push('/login')
  } catch (e: any) {
    error.value = e.response?.data?.message || '注册失败，请稍后重试'
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
  max-width: 480px;
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
    margin: 0 0 $spacing-2;
  }
  
  p {
    color: $text-secondary;
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
  
  .form-hint {
    display: block;
    font-size: $font-size-xs;
    color: $text-muted;
    margin-top: $spacing-1;
  }
}

.form-row {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: $spacing-4;
  
  @media (max-width: $breakpoint-sm) {
    grid-template-columns: 1fr;
  }
}

.input-wrapper {
  position: relative;
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

.password-strength {
  display: flex;
  align-items: center;
  gap: $spacing-2;
  margin-top: $spacing-2;
  
  .strength-bar {
    flex: 1;
    height: 4px;
    background: $gray-200;
    border-radius: 2px;
    position: relative;
    overflow: hidden;
    
    &::after {
      content: '';
      position: absolute;
      top: 0;
      left: 0;
      height: 100%;
      border-radius: 2px;
      transition: width $transition-normal, background $transition-normal;
    }
    
    &.weak::after {
      width: 33%;
      background: $color-danger;
    }
    
    &.medium::after {
      width: 66%;
      background: $color-warning;
    }
    
    &.strong::after {
      width: 100%;
      background: $color-success;
    }
  }
  
  .strength-text {
    font-size: $font-size-xs;
    color: $text-muted;
    min-width: 32px;
  }
}

.checkbox-wrapper {
  display: flex;
  align-items: flex-start;
  gap: $spacing-2;
  cursor: pointer;
  
  input[type="checkbox"] {
    width: 16px;
    height: 16px;
    margin-top: 2px;
    accent-color: $brand-primary;
  }
  
  .checkbox-label {
    font-size: $font-size-sm;
    color: $text-secondary;
    line-height: 1.4;
    
    a {
      color: $text-link;
      
      &:hover {
        text-decoration: underline;
      }
    }
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

