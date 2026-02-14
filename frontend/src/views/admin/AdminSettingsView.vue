<template>
  <div class="admin-settings">
    <div class="page-header">
      <h1>常规设置</h1>
      <p class="page-description">管理实例的全局配置</p>
    </div>

    <div v-if="loading" class="loading-state">
      <div class="spinner"></div>
    </div>

    <template v-else>
      <!-- General Section -->
      <div class="settings-section">
        <div class="section-header" @click="toggleSection('general')">
          <div class="section-title">
            <h2>通用设置</h2>
            <p>站点名称、描述等基础配置</p>
          </div>
          <svg class="chevron" :class="{ expanded: expandedSections.general }" width="16" height="16" viewBox="0 0 16 16">
            <path d="M6 4l4 4-4 4" stroke="currentColor" stroke-width="1.5" fill="none" stroke-linecap="round"/>
          </svg>
        </div>
        <div v-show="expandedSections.general" class="section-body">
          <div class="form-group">
            <label for="site_name">站点名称</label>
            <input id="site_name" v-model="form.site_name" type="text" placeholder="GitFox" />
            <p class="form-hint">显示在页面标题和邮件通知中</p>
          </div>
          <div class="form-group">
            <label for="site_description">站点描述</label>
            <textarea id="site_description" v-model="form.site_description" rows="3" placeholder="GitFox DevSecOps Platform"></textarea>
            <p class="form-hint">显示在探索页面和搜索引擎结果中</p>
          </div>
          <div class="section-actions">
            <button class="btn btn-primary" @click="saveSection('general')" :disabled="saving">
              {{ saving ? '保存中...' : '保存更改' }}
            </button>
          </div>
        </div>
      </div>

      <!-- Sign-up Section -->
      <div class="settings-section">
        <div class="section-header" @click="toggleSection('signup')">
          <div class="section-title">
            <h2>注册限制</h2>
            <p>控制用户注册和账户创建</p>
          </div>
          <svg class="chevron" :class="{ expanded: expandedSections.signup }" width="16" height="16" viewBox="0 0 16 16">
            <path d="M6 4l4 4-4 4" stroke="currentColor" stroke-width="1.5" fill="none" stroke-linecap="round"/>
          </svg>
        </div>
        <div v-show="expandedSections.signup" class="section-body">
          <div class="form-group checkbox-group">
            <label class="checkbox-label">
              <input type="checkbox" v-model="form.signup_enabled" />
              <span>启用注册</span>
            </label>
            <p class="form-hint">关闭后，新用户只能由管理员创建</p>
          </div>
          <div class="form-group checkbox-group">
            <label class="checkbox-label">
              <input type="checkbox" v-model="form.require_email_confirmation" />
              <span>要求邮箱确认</span>
            </label>
            <p class="form-hint">新用户需要验证邮箱后才能登录</p>
          </div>
          <div class="form-group">
            <label for="user_default_role">新用户默认角色</label>
            <select id="user_default_role" v-model="form.user_default_role">
              <option value="developer">开发者</option>
              <option value="viewer">观察者</option>
            </select>
          </div>
          <div class="section-actions">
            <button class="btn btn-primary" @click="saveSection('signup')" :disabled="saving">
              {{ saving ? '保存中...' : '保存更改' }}
            </button>
          </div>
        </div>
      </div>

      <!-- Project Defaults Section -->
      <div class="settings-section">
        <div class="section-header" @click="toggleSection('project')">
          <div class="section-title">
            <h2>项目默认设置</h2>
            <p>新建项目的默认配置</p>
          </div>
          <svg class="chevron" :class="{ expanded: expandedSections.project }" width="16" height="16" viewBox="0 0 16 16">
            <path d="M6 4l4 4-4 4" stroke="currentColor" stroke-width="1.5" fill="none" stroke-linecap="round"/>
          </svg>
        </div>
        <div v-show="expandedSections.project" class="section-body">
          <div class="form-group">
            <label for="default_project_visibility">默认项目可见性</label>
            <select id="default_project_visibility" v-model="form.default_project_visibility">
              <option value="private">私有</option>
              <option value="internal">内部</option>
              <option value="public">公开</option>
            </select>
            <p class="form-hint">新项目创建时的默认可见性级别</p>
          </div>
          <div class="form-group">
            <label for="max_attachment_size_mb">最大附件大小 (MB)</label>
            <input id="max_attachment_size_mb" v-model.number="form.max_attachment_size_mb" type="number" min="1" max="1024" />
          </div>
          <div class="section-actions">
            <button class="btn btn-primary" @click="saveSection('project')" :disabled="saving">
              {{ saving ? '保存中...' : '保存更改' }}
            </button>
          </div>
        </div>
      </div>

      <!-- Appearance Section -->
      <div class="settings-section">
        <div class="section-header" @click="toggleSection('appearance')">
          <div class="section-title">
            <h2>外观与行为</h2>
            <p>头像、跳转路径等设置</p>
          </div>
          <svg class="chevron" :class="{ expanded: expandedSections.appearance }" width="16" height="16" viewBox="0 0 16 16">
            <path d="M6 4l4 4-4 4" stroke="currentColor" stroke-width="1.5" fill="none" stroke-linecap="round"/>
          </svg>
        </div>
        <div v-show="expandedSections.appearance" class="section-body">
          <div class="form-group checkbox-group">
            <label class="checkbox-label">
              <input type="checkbox" v-model="form.gravatar_enabled" />
              <span>启用 Gravatar</span>
            </label>
            <p class="form-hint">使用 Gravatar 服务显示用户头像</p>
          </div>
          <div class="form-group">
            <label for="after_sign_in_path">登录后跳转路径</label>
            <input id="after_sign_in_path" v-model="form.after_sign_in_path" type="text" placeholder="/" />
            <p class="form-hint">用户登录成功后默认跳转的页面路径</p>
          </div>
          <div class="form-group">
            <label for="home_page_url">自定义首页 URL</label>
            <input id="home_page_url" v-model="form.home_page_url" type="text" placeholder="" />
            <p class="form-hint">留空则使用默认首页</p>
          </div>
          <div class="section-actions">
            <button class="btn btn-primary" @click="saveSection('appearance')" :disabled="saving">
              {{ saving ? '保存中...' : '保存更改' }}
            </button>
          </div>
        </div>
      </div>

      <!-- Terms Section -->
      <div class="settings-section">
        <div class="section-header" @click="toggleSection('terms')">
          <div class="section-title">
            <h2>服务条款</h2>
            <p>用户需同意的服务条款内容</p>
          </div>
          <svg class="chevron" :class="{ expanded: expandedSections.terms }" width="16" height="16" viewBox="0 0 16 16">
            <path d="M6 4l4 4-4 4" stroke="currentColor" stroke-width="1.5" fill="none" stroke-linecap="round"/>
          </svg>
        </div>
        <div v-show="expandedSections.terms" class="section-body">
          <div class="form-group">
            <label for="terms_of_service">服务条款 (Markdown)</label>
            <textarea id="terms_of_service" v-model="form.terms_of_service" rows="8" placeholder="请输入服务条款内容..."></textarea>
          </div>
          <div class="section-actions">
            <button class="btn btn-primary" @click="saveSection('terms')" :disabled="saving">
              {{ saving ? '保存中...' : '保存更改' }}
            </button>
          </div>
        </div>
      </div>
    </template>

    <!-- Success toast -->
    <Transition name="fade">
      <div v-if="successMsg" class="success-toast">
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
          <path d="M3 8l4 4 6-8" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
        {{ successMsg }}
      </div>
    </Transition>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import api from '@/api'

const loading = ref(true)
const saving = ref(false)
const successMsg = ref('')

const expandedSections = reactive({
  general: true,
  signup: false,
  project: false,
  appearance: false,
  terms: false,
})

const form = reactive({
  site_name: '',
  site_description: '',
  signup_enabled: true,
  require_email_confirmation: false,
  user_default_role: 'developer',
  default_project_visibility: 'private',
  max_attachment_size_mb: 10,
  gravatar_enabled: true,
  home_page_url: '',
  after_sign_in_path: '/',
  terms_of_service: '',
})

function toggleSection(section: keyof typeof expandedSections) {
  expandedSections[section] = !expandedSections[section]
}

const sectionKeys: Record<string, string[]> = {
  general: ['site_name', 'site_description'],
  signup: ['signup_enabled', 'require_email_confirmation', 'user_default_role'],
  project: ['default_project_visibility', 'max_attachment_size_mb'],
  appearance: ['gravatar_enabled', 'home_page_url', 'after_sign_in_path'],
  terms: ['terms_of_service'],
}

async function loadConfigs() {
  try {
    const configs = await api.admin.getConfigs()
    for (const [key, value] of Object.entries(configs)) {
      if (key in form) {
        ;(form as any)[key] = value
      }
    }
  } catch (err) {
    console.error('Failed to load configs:', err)
  } finally {
    loading.value = false
  }
}

async function saveSection(section: string) {
  saving.value = true
  try {
    const keys = sectionKeys[section] || []
    const configs = keys.map(key => ({
      key,
      value: (form as any)[key],
    }))
    await api.admin.updateConfigs(configs)
    showSuccess('设置已保存')
  } catch (err) {
    console.error('Failed to save configs:', err)
    alert('保存失败，请重试')
  } finally {
    saving.value = false
  }
}

function showSuccess(msg: string) {
  successMsg.value = msg
  setTimeout(() => { successMsg.value = '' }, 3000)
}

onMounted(loadConfigs)
</script>

<style lang="scss" scoped>
.admin-settings {
  max-width: 900px;
  margin: 0 auto;
  padding: $spacing-6;
}

.page-header {
  margin-bottom: $spacing-6;
  h1 { font-size: $font-size-2xl; font-weight: $font-weight-bold; color: $text-primary; margin: 0 0 $spacing-2; }
  .page-description { color: $text-secondary; font-size: $font-size-base; margin: 0; }
}

.settings-section {
  background: $bg-primary;
  border: 1px solid $border-color;
  border-radius: $border-radius-lg;
  margin-bottom: $spacing-4;
}

.section-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: $spacing-5 $spacing-6;
  cursor: pointer;
  user-select: none;
  transition: background $transition-fast;

  &:hover { background: $bg-secondary; border-radius: $border-radius-lg; }

  .section-title {
    h2 {
      font-size: $font-size-lg;
      font-weight: $font-weight-semibold;
      color: $text-primary;
      margin: 0 0 $spacing-1;
    }
    p {
      color: $text-secondary;
      font-size: $font-size-sm;
      margin: 0;
    }
  }

  .chevron {
    color: $text-muted;
    transition: transform $transition-normal;
    flex-shrink: 0;
    &.expanded { transform: rotate(90deg); }
  }
}

.section-body {
  padding: 0 $spacing-6 $spacing-6;
  border-top: 1px solid $border-color;
  padding-top: $spacing-5;
}

.form-group {
  margin-bottom: $spacing-5;

  > label {
    display: block;
    font-size: $font-size-sm;
    font-weight: $font-weight-semibold;
    color: $text-primary;
    margin-bottom: $spacing-2;
  }

  input[type="text"],
  input[type="number"],
  select,
  textarea {
    width: 100%;
    padding: $spacing-2 $spacing-3;
    border: 1px solid $border-color;
    border-radius: $border-radius;
    font-size: $font-size-sm;
    color: $text-primary;
    background: $bg-primary;
    transition: border-color $transition-fast;
    box-sizing: border-box;

    &:focus {
      outline: none;
      border-color: $brand-primary;
      box-shadow: $shadow-focus;
    }
    &::placeholder { color: $text-muted; }
  }

  textarea {
    resize: vertical;
    font-family: inherit;
    line-height: $line-height-normal;
  }

  select { cursor: pointer; }
}

.form-hint {
  font-size: $font-size-xs;
  color: $text-muted;
  margin: $spacing-1 0 0;
}

.checkbox-group {
  .checkbox-label {
    display: inline-flex;
    align-items: center;
    gap: $spacing-2;
    cursor: pointer;
    font-weight: $font-weight-normal;

    input[type="checkbox"] {
      width: 16px;
      height: 16px;
      accent-color: $brand-primary;
      cursor: pointer;
    }

    span {
      font-size: $font-size-sm;
      color: $text-primary;
    }
  }
}

.section-actions {
  padding-top: $spacing-4;
  border-top: 1px solid $border-color-light;
}

.btn {
  padding: $spacing-2 $spacing-4;
  border-radius: $border-radius;
  font-size: $font-size-sm;
  font-weight: $font-weight-medium;
  cursor: pointer;
  border: 1px solid transparent;
  transition: all $transition-fast;
  &:disabled { opacity: 0.6; cursor: not-allowed; }
}

.btn-primary {
  background: $brand-primary;
  color: white;
  &:hover:not(:disabled) { background: $primary-dark; }
}

.success-toast {
  position: fixed;
  bottom: $spacing-6;
  right: $spacing-6;
  display: flex;
  align-items: center;
  gap: $spacing-2;
  padding: $spacing-3 $spacing-5;
  background: $color-success;
  color: white;
  border-radius: $border-radius;
  font-size: $font-size-sm;
  font-weight: $font-weight-medium;
  box-shadow: $shadow-lg;
  z-index: $z-tooltip;
}

.fade-enter-active, .fade-leave-active { transition: opacity $transition-slow; }
.fade-enter-from, .fade-leave-to { opacity: 0; }

.loading-state {
  display: flex;
  justify-content: center;
  padding: $spacing-12;

  .spinner {
    width: 32px;
    height: 32px;
    border: 3px solid $border-color;
    border-top-color: $brand-primary;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }
}

@keyframes spin { to { transform: rotate(360deg); } }
</style>
