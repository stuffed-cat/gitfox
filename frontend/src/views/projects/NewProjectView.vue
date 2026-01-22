<template>
  <div class="new-project-page">
    <h1>创建新项目</h1>
    
    <div class="card">
      <form @submit.prevent="handleSubmit" class="project-form">
        <div v-if="error" class="alert alert-danger">{{ error }}</div>
        
        <div class="form-group">
          <label for="name">项目名称 *</label>
          <input
            id="name"
            v-model="form.name"
            type="text"
            class="form-control"
            placeholder="输入项目名称"
            required
          />
        </div>
        
        <div class="form-group">
          <label for="description">项目描述</label>
          <textarea
            id="description"
            v-model="form.description"
            class="form-control"
            placeholder="输入项目描述（可选）"
            rows="4"
          ></textarea>
        </div>
        
        <div class="form-group">
          <label for="visibility">可见性</label>
          <select id="visibility" v-model="form.visibility" class="form-control">
            <option value="private">私有 - 仅项目成员可见</option>
            <option value="internal">内部 - 登录用户可见</option>
            <option value="public">公开 - 所有人可见</option>
          </select>
        </div>
        
        <div class="form-group">
          <label for="default_branch">默认分支</label>
          <input
            id="default_branch"
            v-model="form.default_branch"
            type="text"
            class="form-control"
            placeholder="main"
          />
        </div>
        
        <div class="form-actions">
          <router-link to="/projects" class="btn btn-outline">取消</router-link>
          <button type="submit" class="btn btn-primary" :disabled="loading">
            {{ loading ? '创建中...' : '创建项目' }}
          </button>
        </div>
      </form>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive } from 'vue'
import { useRouter } from 'vue-router'
import { useProjectStore } from '@/stores/project'

const router = useRouter()
const projectStore = useProjectStore()

const form = reactive({
  name: '',
  description: '',
  visibility: 'private' as 'public' | 'private' | 'internal',
  default_branch: 'main'
})

const loading = ref(false)
const error = ref('')

async function handleSubmit() {
  loading.value = true
  error.value = ''
  
  try {
    const project = await projectStore.createProject(form)
    router.push(`/projects/${project.slug}`)
  } catch (e: any) {
    error.value = e.response?.data?.message || '创建项目失败'
  } finally {
    loading.value = false
  }
}
</script>

<style lang="scss" scoped>
.new-project-page {
  max-width: 600px;
  margin: 0 auto;
  
  h1 {
    font-size: $font-size-xxl;
    margin-bottom: $spacing-lg;
  }
}

.project-form {
  padding: $spacing-lg;
}

.form-actions {
  display: flex;
  gap: $spacing-md;
  justify-content: flex-end;
  margin-top: $spacing-lg;
  padding-top: $spacing-lg;
  border-top: 1px solid $border-color;
}
</style>
