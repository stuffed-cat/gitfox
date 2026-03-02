<template>
  <div class="new-merge-request">
    <!-- Step 1: Branch Selection -->
    <div v-if="!showEditForm">
      <div class="page-header">
        <h2>新建合并请求</h2>
      </div>
      
      <div class="mr-form">
        <div v-if="error" class="alert alert-danger">{{ error }}</div>
        
        <div class="form-section branches-section">
          <div class="branch-selection-grid">
            <!-- Source -->
            <div class="branch-column">
              <label class="form-label">源分支</label>
              
              <select 
                v-model="selectedSourceProjectId" 
                class="form-select"
                @change="onSourceProjectChange"
              >
                <option :value="project?.id">{{ project?.owner_name }}/{{ project?.name }}</option>
                <option 
                  v-for="relatedProj in relatedProjects" 
                  :key="relatedProj.id" 
                  :value="relatedProj.id"
                >
                  {{ relatedProj.owner_name }}/{{ relatedProj.name }}
                </option>
              </select>
              
              <select 
                v-model="selectedSourceBranch" 
                class="form-select mt-2" 
                required
                :disabled="!sourceBranches.length"
              >
                <option value="">选择源分支</option>
                <option v-for="branch in sourceBranches" :key="branch.name" :value="branch.name">
                  {{ branch.name }}
                </option>
              </select>
            </div>
            
            <div class="arrow-column">
              <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M5 12h14M12 5l7 7-7 7"/>
              </svg>
            </div>
            
            <!-- Target -->
            <div class="branch-column">
              <label class="form-label">目标分支</label>
              
              <select 
                v-model="selectedTargetProjectId" 
                class="form-select"
                @change="onTargetProjectChange"
              >
                <option :value="project?.id">{{ project?.owner_name }}/{{ project?.name }}</option>
                <option 
                  v-for="relatedProj in relatedProjects" 
                  :key="relatedProj.id" 
                  :value="relatedProj.id"
                >
                  {{ relatedProj.owner_name }}/{{ relatedProj.name }}
                </option>
              </select>
              
              <select 
                v-model="selectedTargetBranch" 
                class="form-select mt-2" 
                required
                :disabled="!targetBranches.length"
              >
                <option value="">选择目标分支</option>
                <option v-for="branch in targetBranches" :key="branch.name" :value="branch.name">
                  {{ branch.name }}
                </option>
              </select>
            </div>
          </div>
          
          <div class="compare-action">
            <button 
              type="button" 
              class="btn btn-primary" 
              :disabled="!canCompare"
              @click="goToEditForm"
            >
              比较分支并继续
            </button>
          </div>
        </div>
      </div>
    </div>
    
    <!-- Step 2: Edit MR Details -->
    <div v-else>
      <div class="page-header">
        <h2>新建合并请求</h2>
        <p class="subtitle">
          从 {{ getProjectName(selectedSourceProjectId) }}:{{ selectedSourceBranch }} 
          到 {{ getProjectName(selectedTargetProjectId) }}:{{ selectedTargetBranch }}
        </p>
      </div>
      
      <form @submit.prevent="handleSubmit" class="mr-form">
        <div v-if="error" class="alert alert-danger">{{ error }}</div>
        
        <div class="form-section">
          <div class="input-group">
            <label class="input-label required" for="title">标题</label>
            <input
              id="title"
              v-model="form.title"
              type="text"
              class="form-control"
              placeholder="请输入合并请求标题"
              required
            />
          </div>
          
          <div class="input-group">
            <label class="input-label" for="description">描述</label>
            <textarea
              id="description"
              v-model="form.description"
              class="form-control textarea"
              placeholder="描述此合并请求的目的和更改内容..."
              rows="8"
            ></textarea>
          </div>
        </div>
        
        <div class="form-actions">
          <button type="submit" class="btn btn-primary" :disabled="loading">
            {{ loading ? '正在创建...' : '创建合并请求' }}
          </button>
          <button type="button" class="btn btn-default" @click="showEditForm = false">
            返回
          </button>
        </div>
      </form>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, computed } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import api from '@/api';
import type { Project, BranchInfo } from '@/types';

const route = useRoute();
const router = useRouter();

const namespace = computed(() => {
  const segments = route.params.pathSegments as string[]
  if (!segments || segments.length < 2) return ''
  return segments.slice(0, -1).join('/')
})
const projectName = computed(() => {
  const segments = route.params.pathSegments as string[]
  if (!segments || segments.length < 2) return ''
  return segments[segments.length - 1]
})

const project = ref<Project | null>(null);
const relatedProjects = ref<Project[]>([]);

const selectedSourceProjectId = ref<string>('');
const selectedTargetProjectId = ref<string>('');
const selectedSourceBranch = ref<string>('');
const selectedTargetBranch = ref<string>('');

const sourceBranches = ref<BranchInfo[]>([]);
const targetBranches = ref<BranchInfo[]>([]);

const showEditForm = ref(false);

const form = ref({
  title: '',
  description: ''
});

const loading = ref(false);
const error = ref('');

const canCompare = computed(() => {
  return selectedSourceBranch.value && 
         selectedTargetBranch.value &&
         (selectedSourceProjectId.value !== selectedTargetProjectId.value || 
          selectedSourceBranch.value !== selectedTargetBranch.value);
});

function getProjectName(projectId: string): string {
  if (projectId === project.value?.id) {
    return `${project.value.owner_name}/${project.value.name}`;
  }
  const proj = relatedProjects.value.find(p => p.id === projectId);
  return proj ? `${proj.owner_name}/${proj.name}` : '';
}

function goToEditForm() {
  if (!canCompare.value) return;
  showEditForm.value = true;
}

onMounted(async () => {
  try {
    project.value = await api.projects.get({ namespace, project: projectName });
    selectedSourceProjectId.value = project.value.id;
    selectedTargetProjectId.value = project.value.id;
    
    await Promise.all([
      loadBranches(project.value.id, 'source'),
      loadBranches(project.value.id, 'target'),
      loadRelatedProjects()
    ]);
  } catch (e: any) {
    console.error('Failed to load project:', e);
    error.value = e.response?.data?.error || e.message || '加载项目信息失败';
  }
});

async function loadRelatedProjects() {
  if (!project.value) return;
  
  try {
    // Get the entire fork network (tree) - includes parent, siblings, children, etc.
    const result = await api.projects.getForkNetwork({ namespace, project: projectName });
    // Filter out current project from the list
    relatedProjects.value = result.projects.filter(p => p.id !== project.value?.id);
  } catch (e) {
    console.error('Failed to load related projects:', e);
  }
}

async function loadBranches(projectId: string, type: 'source' | 'target') {
  const proj = projectId === project.value?.id 
    ? project.value 
    : relatedProjects.value.find(p => p.id === projectId);
  
  if (!proj || !proj.owner_name || !proj.name) return;
  
  try {
    const branches = await api.branches.list({ 
      namespace: proj.owner_name, 
      project: proj.name 
    });
    if (type === 'source') {
      sourceBranches.value = branches;
    } else {
      targetBranches.value = branches;
    }
  } catch (e) {
    console.error(`Failed to load ${type} branches:`, e);
  }
}

function onSourceProjectChange() {
  selectedSourceBranch.value = '';
  loadBranches(selectedSourceProjectId.value, 'source');
}

function onTargetProjectChange() {
  selectedTargetBranch.value = '';
  loadBranches(selectedTargetProjectId.value, 'target');
}

async function handleSubmit() {
  if (!form.value.title.trim()) {
    error.value = '请填写标题';
    return;
  }
  
  try {
    loading.value = true;
    error.value = '';
    
    const isCrossRepo = selectedSourceProjectId.value !== selectedTargetProjectId.value;
    
    const requestData: any = {
      title: form.value.title,
      description: form.value.description,
      source_branch: selectedSourceBranch.value,
      target_branch: selectedTargetBranch.value
    };
    
    if (isCrossRepo) {
      requestData.source_project_id = selectedSourceProjectId.value;
    }
    
    await api.mergeRequests.create(
      { namespace, project: projectName }, 
      requestData
    );
    
    router.push({
      name: 'merge-requests',
      params: { namespace, project: projectName }
    });
  } catch (e: any) {
    console.error('Failed to create MR:', e);
    error.value = e.response?.data?.error || e.message || '创建合并请求失败';
  } finally {
    loading.value = false;
  }
}
</script>

<style lang="scss" scoped>
@import '@/styles/variables.scss';

.new-merge-request {
  max-width: 1000px;
  margin: 0 auto;
  padding: $spacing-6;
}

.page-header {
  margin-bottom: $spacing-6;
  
  h2 {
    font-size: 24px;
    font-weight: 600;
    color: $text-primary;
    margin: 0;
  }
  
  .subtitle {
    font-size: 14px;
    color: $text-secondary;
    margin: $spacing-2 0 0 0;
  }
}

.mr-form {
  background: $bg-primary;
}

.form-section {
  background: $bg-primary;
  border: 1px solid $border-color;
  border-radius: $border-radius;
  padding: $spacing-5;
  margin-bottom: $spacing-4;
}

.branches-section {
  padding: $spacing-6;
}

.branch-selection-grid {
  display: grid;
  grid-template-columns: 1fr auto 1fr;
  gap: $spacing-4;
  align-items: start;
}

.branch-column {
  .form-label {
    display: block;
    font-size: 14px;
    font-weight: 600;
    color: $text-primary;
    margin-bottom: $spacing-3;
  }
}

.arrow-column {
  display: flex;
  align-items: center;
  justify-content: center;
  padding-top: 60px;
  color: $text-tertiary;
}

.compare-action {
  margin-top: $spacing-6;
  text-align: center;
  padding-top: $spacing-5;
  border-top: 1px solid $border-color;
}

.mt-2 {
  margin-top: $spacing-2;
}

.form-select,
.form-control {
  width: 100%;
  padding: $spacing-2 $spacing-3;
  font-size: 14px;
  line-height: 1.5;
  color: $text-primary;
  background-color: $bg-primary;
  border: 1px solid $border-color;
  border-radius: $border-radius-sm;
  transition: border-color 0.15s;
  
  &:hover {
    border-color: $border-color-dark;
  }
  
  &:focus {
    outline: none;
    border-color: $primary;
    box-shadow: 0 0 0 3px rgba(99, 102, 241, 0.1);
  }
  
  &:disabled {
    background-color: $bg-secondary;
    cursor: not-allowed;
    opacity: 0.6;
  }
}

.compare-action {
  margin-top: $spacing-5;
  text-align: center;
}

.btn-compare {
  padding: $spacing-2 $spacing-5;
  font-size: 14px;
  font-weight: 500;
  color: white;
  background-color: $primary;
  border: 1px solid $primary;
  border-radius: $border-radius-sm;
  cursor: pointer;
  transition: background-color 0.15s;
  
  &:hover {
    background-color: darken($primary, 8%);
  }
}

.input-group {
  margin-bottom: $spacing-4;
  
  .input-label {
    display: block;
    font-size: 14px;
    font-weight: 500;
    color: $text-primary;
    margin-bottom: $spacing-2;
    
    &.required::after {
      content: ' *';
      color: $danger;
    }
  }
}

.form-control.textarea {
  resize: vertical;
  font-family: inherit;
}

.alert {
  padding: $spacing-3 $spacing-4;
  border-radius: $border-radius-sm;
  margin-bottom: $spacing-4;
  font-size: 14px;
  
  &.alert-danger {
    background-color: $color-danger-light;
    border: 1px solid lighten($danger, 20%);
    color: darken($danger, 10%);
  }
}

.form-actions {
  display: flex;
  gap: $spacing-3;
  margin-top: $spacing-5;
}

.btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: $spacing-2 $spacing-4;
  font-size: 14px;
  font-weight: 500;
  line-height: 1.5;
  text-decoration: none;
  border: 1px solid transparent;
  border-radius: $border-radius-sm;
  cursor: pointer;
  transition: all 0.15s;
  
  &:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }
}

.btn-primary {
  color: white;
  background-color: $primary;
  border-color: $primary;
  
  &:hover:not(:disabled) {
    background-color: darken($primary, 8%);
  }
}

.btn-default {
  color: $text-primary;
  background-color: $bg-primary;
  border-color: $border-color;
  
  &:hover:not(:disabled) {
    background-color: $bg-secondary;
  }
}

@media (max-width: 768px) {
  .branch-selection-grid {
    grid-template-columns: 1fr;
  }
  
  .arrow-column {
    padding-top: 0;
    padding-bottom: $spacing-2;
    
    svg {
      transform: rotate(90deg);
    }
  }
}
</style>
