<template>
  <div class="oauth-callback">
    <div class="callback-container">
      <template v-if="loading">
        <div class="loading-spinner"></div>
        <h2>正在登录...</h2>
        <p>请稍候，正在完成 {{ providerName }} 认证</p>
      </template>
      
      <template v-else-if="error">
        <div class="error-icon">✕</div>
        <h2>登录失败</h2>
        <p class="error-message">{{ error }}</p>
        <router-link to="/login" class="btn btn-primary">
          返回登录页面
        </router-link>
      </template>
      
      <template v-else>
        <div class="success-icon">✓</div>
        <h2>登录成功</h2>
        <p>正在跳转...</p>
      </template>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, computed } from 'vue';
import { useRouter, useRoute } from 'vue-router';
import { useAuthStore } from '@/stores/auth';

const router = useRouter();
const route = useRoute();
const authStore = useAuthStore();

const loading = ref(true);
const error = ref<string | null>(null);

const providerName = computed(() => {
  const provider = route.query.provider as string;
  const names: Record<string, string> = {
    github: 'GitHub',
    gitlab: 'GitLab',
    google: 'Google',
    azure_ad: 'Microsoft',
    bitbucket: 'Bitbucket'
  };
  return names[provider] || provider || 'OAuth';
});

// 安全的 redirect 路径验证（防止 Open Redirect 攻击）
function isSafeRedirect(path: string): boolean {
  if (!path || typeof path !== 'string') return false
  
  // 只允许内部路径（以 / 开头，不包含协议）
  if (!path.startsWith('/')) return false
  if (path.includes('://')) return false
  if (path.startsWith('//')) return false // 防止 protocol-relative URL
  
  return true
}

onMounted(async () => {
  try {
    const token = route.query.token as string;
    const errorParam = route.query.error as string;
    const errorDesc = route.query.error_description as string;

    if (errorParam) {
      throw new Error(errorDesc || errorParam);
    }

    if (!token) {
      throw new Error('未收到认证令牌');
    }

    // Store the token and fetch user info
    authStore.setToken(token);
    
    // Fetch current user info
    await authStore.fetchCurrentUser();
    
    loading.value = false;
    
    // Redirect to intended page or dashboard (use login_redirect key, validate for security)
    const savedRedirect = sessionStorage.getItem('login_redirect');
    sessionStorage.removeItem('login_redirect');
    
    const redirectTo = (savedRedirect && isSafeRedirect(savedRedirect)) ? savedRedirect : '/';
    
    setTimeout(() => {
      router.push(redirectTo);
    }, 500);
  } catch (err) {
    loading.value = false;
    error.value = err instanceof Error ? err.message : '认证失败';
    authStore.logout();
  }
});
</script>

<style lang="scss" scoped>
.oauth-callback {
  min-height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--gl-background-color, #fafafa);
}

.callback-container {
  text-align: center;
  padding: 48px;
  background: white;
  border-radius: 8px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
  max-width: 400px;
  width: 100%;
  margin: 16px;

  h2 {
    margin: 16px 0 8px;
    color: var(--gl-text-color, #303030);
  }

  p {
    color: var(--gl-text-color-secondary, #666);
    margin: 0 0 24px;
  }
}

.loading-spinner {
  width: 48px;
  height: 48px;
  border: 4px solid #e4e7ed;
  border-top-color: var(--gl-primary-color, #1f75cb);
  border-radius: 50%;
  margin: 0 auto;
  animation: spin 1s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

.success-icon {
  width: 64px;
  height: 64px;
  border-radius: 50%;
  background: #108548;
  color: white;
  font-size: 32px;
  line-height: 64px;
  margin: 0 auto;
}

.error-icon {
  width: 64px;
  height: 64px;
  border-radius: 50%;
  background: #dd2b0e;
  color: white;
  font-size: 32px;
  line-height: 64px;
  margin: 0 auto;
}

.error-message {
  color: #dd2b0e !important;
}

.btn {
  display: inline-block;
  padding: 10px 16px;
  border-radius: 4px;
  text-decoration: none;
  font-weight: 500;
  transition: background-color 0.2s;

  &.btn-primary {
    background: var(--gl-primary-color, #1f75cb);
    color: white;

    &:hover {
      background: var(--gl-primary-color-dark, #1068bf);
    }
  }
}
</style>
