<template>
  <div class="auth-page">
    <div class="auth-card">
      <div class="auth-header">
        <h1>注册</h1>
        <p>创建您的 DevOps 账户</p>
      </div>
      
      <form @submit.prevent="handleSubmit" class="auth-form">
        <div v-if="error" class="alert alert-danger">{{ error }}</div>
        
        <div class="form-group">
          <label for="username">用户名</label>
          <input
            id="username"
            v-model="form.username"
            type="text"
            class="form-control"
            placeholder="请输入用户名"
            required
          />
        </div>
        
        <div class="form-group">
          <label for="email">邮箱</label>
          <input
            id="email"
            v-model="form.email"
            type="email"
            class="form-control"
            placeholder="请输入邮箱"
            required
          />
        </div>
        
        <div class="form-group">
          <label for="display_name">显示名称（可选）</label>
          <input
            id="display_name"
            v-model="form.display_name"
            type="text"
            class="form-control"
            placeholder="请输入显示名称"
          />
        </div>
        
        <div class="form-group">
          <label for="password">密码</label>
          <input
            id="password"
            v-model="form.password"
            type="password"
            class="form-control"
            placeholder="请输入密码（至少8位）"
            required
            minlength="8"
          />
        </div>
        
        <div class="form-group">
          <label for="confirm_password">确认密码</label>
          <input
            id="confirm_password"
            v-model="confirmPassword"
            type="password"
            class="form-control"
            placeholder="请再次输入密码"
            required
          />
        </div>
        
        <button type="submit" class="btn btn-primary btn-lg w-full" :disabled="loading">
          {{ loading ? '注册中...' : '注册' }}
        </button>
      </form>
      
      <div class="auth-footer">
        已有账户？<router-link to="/login">立即登录</router-link>
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
  email: '',
  password: '',
  display_name: ''
})
const confirmPassword = ref('')
const loading = ref(false)
const error = ref('')

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
  background: linear-gradient(135deg, $bg-dark 0%, $bg-sidebar 100%);
}

.auth-card {
  width: 100%;
  max-width: 420px;
  background: $bg-primary;
  border-radius: $border-radius-lg;
  padding: $spacing-xl;
  box-shadow: $shadow-lg;
}

.auth-header {
  text-align: center;
  margin-bottom: $spacing-xl;
  
  h1 {
    font-size: $font-size-xxl;
    margin-bottom: $spacing-sm;
    color: $text-primary;
  }
  
  p {
    color: $text-muted;
  }
}

.w-full {
  width: 100%;
}

.auth-footer {
  text-align: center;
  margin-top: $spacing-lg;
  color: $text-secondary;
  
  a {
    color: $primary-color;
    font-weight: 500;
  }
}
</style>
