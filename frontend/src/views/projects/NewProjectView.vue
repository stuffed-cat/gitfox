<template>
  <div class="new-project-page">
    <!-- Breadcrumb -->
    <div class="breadcrumb">
      <router-link to="/">你的工作</router-link>
      <span class="separator">/</span>
      <router-link to="/dashboard/projects">项目</router-link>
      <span class="separator">/</span>
      <span>新建项目</span>
    </div>
    
    <div v-if="step === 'choose'" class="choose-type">
      <h1 class="page-title">创建新项目</h1>
      
      <div class="project-types">
        <div class="type-card" @click="selectType('blank')">
          <div class="type-icon">
            <svg viewBox="0 0 64 64" fill="none">
              <rect x="12" y="8" width="40" height="48" rx="4" stroke="currentColor" stroke-width="2"/>
              <path d="M32 24v16M24 32h16" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
            </svg>
          </div>
          <div class="type-info">
            <h3>创建空白项目</h3>
            <p>创建一个空白项目来存放您的文件，规划您的工作，并在代码等方面进行协作。</p>
          </div>
        </div>
        
        <div class="type-card" @click="selectType('template')">
          <div class="type-icon">
            <svg viewBox="0 0 64 64" fill="none">
              <rect x="8" y="12" width="24" height="20" rx="2" stroke="currentColor" stroke-width="2"/>
              <rect x="32" y="12" width="24" height="20" rx="2" stroke="currentColor" stroke-width="2"/>
              <rect x="8" y="32" width="24" height="20" rx="2" stroke="currentColor" stroke-width="2"/>
              <path d="M44 42l8-8M44 42l-8-8" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
            </svg>
          </div>
          <div class="type-info">
            <h3>从模板创建</h3>
            <p>创建一个预先填充了必要文件的项目，以帮助您快速入门。</p>
          </div>
        </div>
        
        <div class="type-card" @click="selectType('import')">
          <div class="type-icon">
            <svg viewBox="0 0 64 64" fill="none">
              <path d="M16 32h32M32 16v32" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
              <circle cx="32" cy="32" r="20" stroke="currentColor" stroke-width="2"/>
              <path d="M24 24l8 8-8 8" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
          </div>
          <div class="type-info">
            <h3>导入项目</h3>
            <p>从外部源（如 GitHub、Bitbucket 或 GitLab 的其他实例）迁移数据。</p>
          </div>
        </div>
        
        <div class="type-card disabled">
          <div class="type-icon">
            <svg viewBox="0 0 64 64" fill="none">
              <circle cx="32" cy="32" r="16" stroke="currentColor" stroke-width="2"/>
              <path d="M32 24v12l8 4" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
              <path d="M12 32a20 20 0 0140 0" stroke="currentColor" stroke-width="2" stroke-dasharray="4 4"/>
            </svg>
          </div>
          <div class="type-info">
            <h3>为外部仓库运行 CI/CD</h3>
            <p>将外部仓库连接到 CI/CD（即将推出）</p>
          </div>
        </div>
      </div>
      
      <p class="command-hint">
        您也可以通过命令行来创建新项目。
        <a href="#" @click.prevent="showCommands = !showCommands">显示相关命令</a>
      </p>
      
      <div v-if="showCommands" class="command-box">
        <h4>命令行创建项目</h4>
        <pre><code>git clone {{ cloneUrl }}
cd my-project
git switch --create main
touch README.md
git add README.md
git commit -m "add README"
git push --set-upstream origin main</code></pre>
      </div>
    </div>
    
    <!-- Step 2: Create blank project form -->
    <div v-else-if="step === 'blank'" class="create-form">
      <div class="form-header">
        <button class="back-btn" @click="step = 'choose'">
          <svg viewBox="0 0 16 16" fill="none">
            <path d="M10 12L6 8l4-4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          </svg>
          返回
        </button>
        <h1>创建空白项目</h1>
        <p>创建一个空白项目来存放您的文件，规划您的工作，并在代码等方面进行协作。</p>
      </div>
      
      <!-- Info banner -->
      <div v-if="showBanner" class="info-banner">
        <svg viewBox="0 0 16 16" fill="none">
          <circle cx="8" cy="8" r="6" stroke="currentColor" stroke-width="1.5"/>
          <path d="M8 5v.5M8 7v4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
        </svg>
        <div>
          <strong>您正在创建一个新的顶级项目</strong>
          <p>成员、项目、试用和付费订阅与特定的顶级群组相关联。如果您已经是顶级群组的成员，您可以创建一个子组，以使您的新工作成为现有顶级群组的一部分。</p>
          <a href="#">了解更多关于子组的信息</a>
        </div>
        <button class="close-btn" @click="showBanner = false">×</button>
      </div>
      
      <form @submit.prevent="handleSubmit" class="project-form">
        <div v-if="error" class="alert alert-error">{{ error }}</div>
        
        <div class="form-group">
          <label for="name">项目名称 <span class="required">*</span></label>
          <input
            id="name"
            v-model="form.name"
            type="text"
            class="form-input"
            placeholder="My project"
            required
          />
          <span class="hint">必须以小写或大写字母、数字、表情符号或下划线开头。也可以包含点、加号、破折号或空格。</span>
        </div>
        
        <div class="form-row">
          <div class="form-group flex-1">
            <label>项目 URL</label>
            <div class="url-input">
              <span class="prefix">{{ baseUrl }}/</span>
              <select v-model="form.namespace" class="namespace-select">
                <option :value="currentUser?.username">{{ currentUser?.username }}</option>
              </select>
              <span class="separator">/</span>
            </div>
          </div>
          <div class="form-group flex-1">
            <label for="project-name">项目名称</label>
            <input
              id="project-name"
              v-model="form.name"
              type="text"
              class="form-input"
              placeholder="my-awesome-project"
            />
          </div>
        </div>
        
        <div class="form-group">
          <label>项目部署目标（可选）</label>
          <select v-model="form.deployTarget" class="form-input">
            <option value="">选择部署目标</option>
            <option value="kubernetes">Kubernetes</option>
            <option value="docker">Docker</option>
            <option value="none">无</option>
          </select>
        </div>
        
        <div class="form-group">
          <label>可见性级别</label>
          <div class="visibility-options">
            <label class="visibility-option" :class="{ active: form.visibility === 'private' }">
              <input type="radio" v-model="form.visibility" value="private" />
              <svg viewBox="0 0 16 16" fill="none">
                <rect x="4" y="7" width="8" height="6" rx="1" stroke="currentColor" stroke-width="1.5"/>
                <path d="M6 7V5a2 2 0 014 0v2" stroke="currentColor" stroke-width="1.5"/>
              </svg>
              <div>
                <strong>私有</strong>
                <span>项目访问权限必须明确授予每个用户。如果此项目是一个群组的一部分，访问权限将授予该群组的成员。</span>
              </div>
            </label>
            
            <label class="visibility-option" :class="{ active: form.visibility === 'internal' }">
              <input type="radio" v-model="form.visibility" value="internal" />
              <svg viewBox="0 0 16 16" fill="none">
                <path d="M8 1L1 5v6l7 4 7-4V5L8 1z" stroke="currentColor" stroke-width="1.5"/>
              </svg>
              <div>
                <strong>内部</strong>
                <span>除外部用户外，任何登录用户均可访问该项目。</span>
              </div>
            </label>
            
            <label class="visibility-option" :class="{ active: form.visibility === 'public' }">
              <input type="radio" v-model="form.visibility" value="public" />
              <svg viewBox="0 0 16 16" fill="none">
                <circle cx="8" cy="8" r="6" stroke="currentColor" stroke-width="1.5"/>
                <path d="M1 8h14M8 2c-2 2-3 4-3 6s1 4 3 6c2-2 3-4 3-6s-1-4-3-6" stroke="currentColor" stroke-width="1.5"/>
              </svg>
              <div>
                <strong>公开</strong>
                <span>无需任何身份验证即可访问项目。</span>
              </div>
            </label>
          </div>
        </div>
        
        <div class="form-section">
          <h3>项目配置</h3>
          
          <label class="checkbox-option">
            <input type="checkbox" v-model="form.initializeWithReadme" />
            <div>
              <strong>使用自述文件初始化仓库</strong>
              <span>允许您立即克隆这个项目的仓库。如果您计划推送一个现有的仓库，请跳过这个步骤。</span>
            </div>
          </label>
          
          <label class="checkbox-option">
            <input type="checkbox" v-model="form.enableSAST" />
            <div>
              <strong>启用静态应用安全测试 (SAST)</strong>
              <span>分析源代码查找已知安全漏洞。</span>
            </div>
          </label>
        </div>
        
        <div class="form-actions">
          <button type="button" class="btn btn-secondary" @click="step = 'choose'">取消</button>
          <button type="submit" class="btn btn-primary" :disabled="loading || !form.name">
            {{ loading ? '创建中...' : '创建项目' }}
          </button>
        </div>
      </form>
    </div>
    
    <!-- Step: Template selection (placeholder) -->
    <div v-else-if="step === 'template'" class="template-select">
      <div class="form-header">
        <button class="back-btn" @click="step = 'choose'">
          <svg viewBox="0 0 16 16" fill="none">
            <path d="M10 12L6 8l4-4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          </svg>
          返回
        </button>
        <h1>从模板创建</h1>
        <p>选择一个模板来快速开始</p>
      </div>
      
      <div class="empty-state">
        <p>模板功能即将推出</p>
        <button class="btn btn-secondary" @click="step = 'choose'">返回</button>
      </div>
    </div>
    
    <!-- Step: Import (placeholder) -->
    <div v-else-if="step === 'import'" class="import-project">
      <div class="form-header">
        <button class="back-btn" @click="step = 'choose'">
          <svg viewBox="0 0 16 16" fill="none">
            <path d="M10 12L6 8l4-4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          </svg>
          返回
        </button>
        <h1>导入项目</h1>
        <p>从外部源导入项目</p>
      </div>
      
      <div class="empty-state">
        <p>导入功能即将推出</p>
        <button class="btn btn-secondary" @click="step = 'choose'">返回</button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed } from 'vue'
import { useRouter } from 'vue-router'
import { useAuthStore } from '@/stores/auth'
import { useProjectStore } from '@/stores/project'

const router = useRouter()
const authStore = useAuthStore()
const projectStore = useProjectStore()

const currentUser = computed(() => authStore.user)
const baseUrl = window.location.origin
const cloneUrl = computed(() => `${baseUrl}/${currentUser.value?.username || 'username'}/my-project.git`)

const step = ref<'choose' | 'blank' | 'template' | 'import'>('choose')
const showCommands = ref(false)
const showBanner = ref(true)
const loading = ref(false)
const error = ref('')

const form = reactive({
  name: '',
  namespace: currentUser.value?.username || '',
  description: '',
  visibility: 'private' as 'public' | 'private' | 'internal',
  deployTarget: '',
  initializeWithReadme: true,
  enableSAST: false
})

function selectType(type: 'blank' | 'template' | 'import') {
  step.value = type
}

async function handleSubmit() {
  loading.value = true
  error.value = ''
  
  try {
    const project = await projectStore.createProject({
      name: form.name,
      description: form.description,
      visibility: form.visibility
    })
    router.push(`/${project.owner_name || currentUser.value?.username}/${project.name}`)
  } catch (e: any) {
    error.value = e.response?.data?.message || '创建项目失败，请重试'
  } finally {
    loading.value = false
  }
}
</script>

<style lang="scss" scoped>
.new-project-page {
  padding: $spacing-6;
  max-width: 900px;
  margin: 0 auto;
}

.breadcrumb {
  display: flex;
  align-items: center;
  gap: $spacing-2;
  margin-bottom: $spacing-6;
  font-size: $text-sm;
  color: $text-secondary;
  
  a {
    color: $color-primary;
    text-decoration: none;
    &:hover { text-decoration: underline; }
  }
  .separator { color: $text-muted; }
}

.page-title {
  font-size: $text-2xl;
  font-weight: 600;
  text-align: center;
  margin-bottom: $spacing-8;
}

.project-types {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: $spacing-4;
  margin-bottom: $spacing-6;
}

.type-card {
  display: flex;
  gap: $spacing-4;
  padding: $spacing-6;
  border: 1px solid $border-color;
  border-radius: $radius-lg;
  cursor: pointer;
  transition: all 0.2s;
  
  &:hover:not(.disabled) {
    border-color: $color-primary;
    background: rgba($color-primary, 0.02);
  }
  
  &.disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  
  .type-icon {
    width: 64px;
    height: 64px;
    flex-shrink: 0;
    color: $color-primary;
    
    svg {
      width: 100%;
      height: 100%;
    }
  }
  
  .type-info {
    h3 {
      font-size: $text-lg;
      font-weight: 600;
      margin-bottom: $spacing-2;
    }
    p {
      font-size: $text-sm;
      color: $text-secondary;
      line-height: 1.5;
    }
  }
}

.command-hint {
  text-align: center;
  color: $text-secondary;
  font-size: $text-sm;
  
  a {
    color: $color-primary;
    text-decoration: none;
    &:hover { text-decoration: underline; }
  }
}

.command-box {
  margin-top: $spacing-4;
  padding: $spacing-4;
  background: $bg-tertiary;
  border-radius: $radius-md;
  
  h4 {
    font-size: $text-sm;
    margin-bottom: $spacing-3;
  }
  
  pre {
    background: #1e1e1e;
    color: #d4d4d4;
    padding: $spacing-4;
    border-radius: $radius-md;
    overflow-x: auto;
    font-size: $text-sm;
  }
}

.form-header {
  margin-bottom: $spacing-6;
  
  .back-btn {
    display: inline-flex;
    align-items: center;
    gap: $spacing-2;
    background: none;
    border: none;
    color: $color-primary;
    cursor: pointer;
    margin-bottom: $spacing-4;
    font-size: $text-sm;
    
    svg {
      width: 16px;
      height: 16px;
    }
  }
  
  h1 {
    font-size: $text-2xl;
    font-weight: 600;
    margin-bottom: $spacing-2;
  }
  
  p {
    color: $text-secondary;
  }
}

.info-banner {
  display: flex;
  gap: $spacing-4;
  padding: $spacing-4;
  background: #e8f4fd;
  border: 1px solid #b8daff;
  border-radius: $radius-md;
  margin-bottom: $spacing-6;
  
  > svg {
    width: 20px;
    height: 20px;
    flex-shrink: 0;
    color: #0066cc;
  }
  
  strong {
    display: block;
    margin-bottom: $spacing-2;
  }
  
  p {
    font-size: $text-sm;
    color: $text-secondary;
    margin-bottom: $spacing-2;
  }
  
  a {
    color: $color-primary;
    font-size: $text-sm;
  }
  
  .close-btn {
    background: none;
    border: none;
    font-size: 20px;
    cursor: pointer;
    color: $text-muted;
    margin-left: auto;
  }
}

.project-form {
  .form-group {
    margin-bottom: $spacing-5;
    
    label {
      display: block;
      font-size: $text-sm;
      font-weight: 500;
      margin-bottom: $spacing-2;
      
      .required {
        color: $color-danger;
      }
    }
    
    .hint {
      display: block;
      font-size: $text-xs;
      color: $text-muted;
      margin-top: $spacing-2;
    }
  }
  
  .form-input {
    width: 100%;
    padding: $spacing-2 $spacing-3;
    border: 1px solid $border-color;
    border-radius: $radius-md;
    font-size: $text-sm;
    
    &:focus {
      outline: none;
      border-color: $color-primary;
    }
  }
  
  .form-row {
    display: flex;
    gap: $spacing-4;
    
    .flex-1 {
      flex: 1;
    }
  }
  
  .url-input {
    display: flex;
    align-items: center;
    border: 1px solid $border-color;
    border-radius: $radius-md;
    overflow: hidden;
    
    .prefix {
      padding: $spacing-2 $spacing-3;
      background: $bg-tertiary;
      color: $text-muted;
      font-size: $text-sm;
      white-space: nowrap;
    }
    
    .namespace-select {
      border: none;
      padding: $spacing-2;
      background: $bg-primary;
      flex: 1;
      min-width: 120px;
    }
    
    .separator {
      padding: 0 $spacing-2;
      color: $text-muted;
    }
  }
}

.visibility-options {
  display: flex;
  flex-direction: column;
  gap: $spacing-3;
}

.visibility-option {
  display: flex;
  align-items: flex-start;
  gap: $spacing-3;
  padding: $spacing-4;
  border: 1px solid $border-color;
  border-radius: $radius-md;
  cursor: pointer;
  
  &:hover {
    background: $bg-secondary;
  }
  
  &.active {
    border-color: $color-primary;
    background: rgba($color-primary, 0.02);
  }
  
  input[type="radio"] {
    display: none;
  }
  
  svg {
    width: 20px;
    height: 20px;
    flex-shrink: 0;
    color: $text-secondary;
  }
  
  div {
    strong {
      display: block;
      margin-bottom: $spacing-1;
    }
    span {
      font-size: $text-sm;
      color: $text-secondary;
    }
  }
}

.form-section {
  margin-top: $spacing-6;
  padding-top: $spacing-6;
  border-top: 1px solid $border-color;
  
  h3 {
    font-size: $text-base;
    font-weight: 600;
    margin-bottom: $spacing-4;
  }
}

.checkbox-option {
  display: flex;
  align-items: flex-start;
  gap: $spacing-3;
  padding: $spacing-3;
  cursor: pointer;
  
  input[type="checkbox"] {
    width: 18px;
    height: 18px;
    margin-top: 2px;
    accent-color: $color-primary;
  }
  
  div {
    strong {
      display: block;
      margin-bottom: $spacing-1;
    }
    span {
      font-size: $text-sm;
      color: $text-secondary;
    }
  }
}

.form-actions {
  display: flex;
  justify-content: flex-end;
  gap: $spacing-3;
  margin-top: $spacing-6;
  padding-top: $spacing-6;
  border-top: 1px solid $border-color;
}

.btn {
  padding: $spacing-2 $spacing-4;
  border-radius: $radius-md;
  font-size: $text-sm;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s;
  
  &:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
}

.btn-primary {
  background: $color-primary;
  color: white;
  border: none;
  
  &:hover:not(:disabled) {
    background: $color-primary-dark;
  }
}

.btn-secondary {
  background: $bg-primary;
  color: $text-primary;
  border: 1px solid $border-color;
  
  &:hover:not(:disabled) {
    background: $bg-secondary;
  }
}

.alert-error {
  padding: $spacing-3;
  background: $color-danger-light;
  color: $color-danger;
  border-radius: $radius-md;
  margin-bottom: $spacing-4;
}

.empty-state {
  text-align: center;
  padding: $spacing-12;
  color: $text-secondary;
  
  .btn {
    margin-top: $spacing-4;
  }
}
</style>
