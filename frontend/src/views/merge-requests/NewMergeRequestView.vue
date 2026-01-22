<template>
  <div class="new-merge-request">
    <h2>创建合并请求</h2>
    
    <form @submit.prevent="handleSubmit" class="mr-form">
      <div v-if="error" class="alert alert-danger">{{ error }}</div>
      
      <div class="branch-selector">
        <div class="form-group">
          <label for="sourceBranch">源分支</label>
          <select id="sourceBranch" v-model="form.source_branch" class="form-control" required>
            <option value="">选择源分支</option>
            <option v-for="branch in branches" :key="branch.name" :value="branch.name">
              {{ branch.name }}
            </option>
          </select>
        </div>
        
        <div class="arrow">→</div>
        
        <div class="form-group">
          <label for="targetBranch">目标分支</label>
          <select id="targetBranch" v-model="form.target_branch" class="form-control" required>
            <option value="">选择目标分支</option>
            <option v-for="branch in branches" :key="branch.name" :value="branch.name">
              {{ branch.name }}
            </option>
          </select>
        </div>
      </div>
      
      <div class="form-group">
        <label for="title">标题</label>
        <input
          id="title"
          v-model="form.title"
          type="text"
          class="form-control"
          placeholder="合并请求标题"
          required
        />
      </div>
      
      <div class="form-group">
        <label for="description">描述</label>
        <textarea
          id="description"
          v-model="form.description"
          class="form-control"
          placeholder="描述更改内容..."
          rows="6"
        ></textarea>
      </div>
      
      <div class="form-actions">
        <router-link :to="`/projects/${project?.slug}/merge-requests`" class="btn btn-outline">
          取消
        </router-link>
        <button type="submit" class="btn btn-primary" :disabled="loading || !canSubmit">
          {{ loading ? '创建中...' : '创建合并请求' }}
        </button>
      </div>
    </form>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, watch } from 'vue'
import { useRouter } from 'vue-router'
import api from '@/api'
import type { Project, Branch } from '@/types'

const props = defineProps<{
  project?: Project
}>()

const router = useRouter()

const loading = ref(false)
const error = ref('')
const branches = ref<Branch[]>([])

const form = reactive({
  source_branch: '',
  target_branch: '',
  title: '',
  description: ''
})

const canSubmit = computed(() => {
  return form.source_branch && 
         form.target_branch && 
         form.title && 
         form.source_branch !== form.target_branch
})

async function loadBranches() {
  if (!props.project?.id) return
  
  try {
    const response = await api.getBranches(props.project.id)
    branches.value = response.data
    if (branches.value.length > 0 && !form.target_branch) {
      form.target_branch = props.project.default_branch || branches.value[0].name
    }
  } catch (e) {
    console.error('Failed to load branches:', e)
  }
}

async function handleSubmit() {
  if (!props.project?.id) return
  loading.value = true
  error.value = ''
  
  try {
    const response = await api.createMergeRequest(props.project.id, form)
    router.push(`/projects/${props.project.slug}/merge-requests/${response.data.iid}`)
  } catch (e: any) {
    error.value = e.response?.data?.message || '创建合并请求失败'
  } finally {
    loading.value = false
  }
}

watch(() => props.project?.id, () => {
  loadBranches()
}, { immediate: true })
</script>

<style lang="scss" scoped>
.new-merge-request {
  padding: $spacing-lg;
  max-width: 800px;
  
  h2 {
    margin-bottom: $spacing-lg;
  }
}

.branch-selector {
  display: flex;
  align-items: flex-end;
  gap: $spacing-md;
  margin-bottom: $spacing-lg;
  
  .form-group {
    flex: 1;
  }
  
  .arrow {
    padding-bottom: $spacing-md;
    font-size: $font-size-xl;
    color: $text-muted;
  }
}

.form-actions {
  display: flex;
  justify-content: flex-end;
  gap: $spacing-md;
  margin-top: $spacing-lg;
  padding-top: $spacing-lg;
  border-top: 1px solid $border-color;
}
</style>
