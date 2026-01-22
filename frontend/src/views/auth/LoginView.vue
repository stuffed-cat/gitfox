<template>
  <div class="auth-page">
    <div class="auth-card">
      <div class="auth-header">
        <h1>登录</h1>
        <p>登录您的 DevOps 账户</p>
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
          <label for="password">密码</label>
          <input
            id="password"
            v-model="form.password"
            type="password"
            class="form-control"
            placeholder="请输入密码"
            required
          />
        </div>
        
        <button type="submit" class="btn btn-primary btn-lg w-full" :disabled="loading">
          {{ loading ? '登录中...' : '登录' }}
        </button>
      </form>
      
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

.auth-form {
  .form-group {
    margin-bottom: $spacing-md;
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
