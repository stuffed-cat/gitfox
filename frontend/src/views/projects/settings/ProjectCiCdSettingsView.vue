<template>
  <div class="project-settings-page">
    <div class="settings-header">
      <h2>CI/CD 设置</h2>
      <p class="description">管理流水线变量、Runner 和自动化配置</p>
    </div>
    
    <div v-if="loading" class="loading">
      <div class="loading-spinner"></div>
    </div>
    
    <template v-else>
      <!-- 通用 CI/CD 设置 -->
      <section class="settings-section">
        <h3>通用设置</h3>
        
        <div class="form-group">
          <label class="checkbox-label">
            <input type="checkbox" v-model="generalSettings.pipelines_enabled" @change="saveGeneralSettings">
            启用流水线
          </label>
          <p class="form-help">启用 CI/CD 流水线自动构建和部署</p>
        </div>
        
        <div class="form-group">
          <label for="pipeline-timeout">流水线默认超时时间</label>
          <div class="timeout-input">
            <input
              id="pipeline-timeout"
              v-model.number="generalSettings.default_timeout"
              type="number"
              class="form-control"
              min="0"
              max="86400"
            />
            <span class="timeout-unit">秒 (最大 24 小时)</span>
          </div>
          <button class="btn btn-primary mt-2" @click="saveGeneralSettings">保存</button>
        </div>
        
        <div class="form-group">
          <label class="checkbox-label">
            <input type="checkbox" v-model="generalSettings.auto_cancel_pending_pipelines" @change="saveGeneralSettings">
            自动取消冗余流水线
          </label>
          <p class="form-help">当同一分支有新推送时，自动取消旧的等待中的流水线</p>
        </div>
        
        <div class="form-group">
          <label class="checkbox-label">
            <input type="checkbox" v-model="generalSettings.skip_outdated_deployment_jobs" @change="saveGeneralSettings">
            跳过过时的部署作业
          </label>
          <p class="form-help">如果有更新的部署正在进行，跳过旧的部署作业</p>
        </div>
      </section>

      <!-- CI/CD 变量 -->
      <section class="settings-section">
        <h3>CI/CD 变量</h3>
        <p class="section-description">定义可在流水线中使用的环境变量</p>
        
        <div class="add-variable">
          <div class="variable-form">
            <div class="form-row">
              <div class="form-group">
                <label for="var-key">键</label>
                <input
                  id="var-key"
                  v-model="newVariable.key"
                  type="text"
                  class="form-control"
                  placeholder="VARIABLE_NAME"
                  @input="validateVariableKey"
                />
              </div>
              
              <div class="form-group flex-grow">
                <label for="var-value">值</label>
                <input
                  id="var-value"
                  v-model="newVariable.value"
                  :type="newVariable.masked ? 'password' : 'text'"
                  class="form-control"
                  placeholder="变量值"
                />
              </div>
            </div>
            
            <div class="form-row options-row">
              <label class="checkbox-label">
                <input type="checkbox" v-model="newVariable.protected">
                受保护
                <span class="option-hint">仅在受保护分支/标签上可用</span>
              </label>
              
              <label class="checkbox-label">
                <input type="checkbox" v-model="newVariable.masked">
                隐藏
                <span class="option-hint">在日志中隐藏变量值</span>
              </label>
              
              <label class="checkbox-label">
                <input type="checkbox" v-model="newVariable.file">
                文件
                <span class="option-hint">将值写入文件而非环境变量</span>
              </label>
            </div>
            
            <div class="form-group">
              <label for="var-env">环境作用域</label>
              <input
                id="var-env"
                v-model="newVariable.environment_scope"
                type="text"
                class="form-control"
                placeholder="* (所有环境) 或 production"
              />
            </div>
          </div>
          
          <button class="btn btn-primary" @click="addVariable" :disabled="!newVariable.key || !newVariable.value">
            添加变量
          </button>
        </div>
        
        <div class="variable-list">
          <div v-for="variable in variables" :key="variable.id" class="variable-item">
            <div class="variable-info">
              <div class="variable-key">
                <code>{{ variable.key }}</code>
                <span v-if="variable.protected" class="badge badge-info">受保护</span>
                <span v-if="variable.masked" class="badge badge-secondary">隐藏</span>
                <span v-if="variable.file" class="badge badge-secondary">文件</span>
              </div>
              <div class="variable-meta">
                <span v-if="variable.environment_scope !== '*'">
                  环境: {{ variable.environment_scope }}
                </span>
                <span>添加于 {{ formatDate(variable.created_at) }}</span>
              </div>
            </div>
            <div class="variable-actions">
              <button class="btn btn-outline btn-sm" @click="editVariable(variable)">
                编辑
              </button>
              <button class="btn btn-danger btn-sm" @click="deleteVariable(variable.id)">
                删除
              </button>
            </div>
          </div>
          
          <div v-if="variables.length === 0" class="empty-state">
            <p>暂无 CI/CD 变量</p>
            <p class="text-muted">添加变量以在流水线中使用敏感数据</p>
          </div>
        </div>
      </section>

      <!-- Runners -->
      <section class="settings-section">
        <h3>Runners</h3>
        <p class="section-description">管理可用于此项目的 CI/CD Runner</p>
        
        <div class="runner-settings">
          <div class="form-group">
            <label class="checkbox-label">
              <input type="checkbox" v-model="runnerSettings.shared_runners_enabled" @change="saveRunnerSettings">
              启用共享 Runner
            </label>
            <p class="form-help">允许使用实例级别的共享 Runner</p>
          </div>
          
          <div class="form-group">
            <label class="checkbox-label">
              <input type="checkbox" v-model="runnerSettings.group_runners_enabled" @change="saveRunnerSettings">
              启用群组 Runner
            </label>
            <p class="form-help">允许使用上级群组的 Runner</p>
          </div>
        </div>
        
        <h4>项目专用 Runner</h4>
        <p class="text-muted mb-3">这些 Runner 仅为此项目运行作业</p>
        
        <div class="runner-list">
          <div v-for="runner in projectRunners" :key="runner.id" class="runner-item">
            <div class="runner-info">
              <div class="runner-name">
                <span class="status-dot" :class="runner.status"></span>
                {{ runner.name }}
                <span v-if="runner.locked" class="badge badge-secondary">已锁定</span>
              </div>
              <div class="runner-meta">
                <span class="runner-token">#{{ runner.id.substring(0, 8) }}</span>
                <span v-if="runner.tags?.length">标签: {{ runner.tags.join(', ') }}</span>
                <span>{{ runner.status === 'running' ? '运行中' : runner.status === 'idle' ? '空闲' : '离线' }}</span>
              </div>
            </div>
            <div class="runner-actions">
              <router-link :to="`${projectPath}/-/runners`" class="btn btn-outline btn-sm">
                管理
              </router-link>
            </div>
          </div>
          
          <div v-if="projectRunners.length === 0" class="empty-state">
            <p>暂无项目专用 Runner</p>
            <router-link :to="`${projectPath}/-/runners`" class="btn btn-primary">
              配置 Runner
            </router-link>
          </div>
        </div>
      </section>

      <!-- Pipeline 触发器 -->
      <section class="settings-section">
        <h3>流水线触发器</h3>
        <p class="section-description">创建触发器以通过 API 运行流水线</p>
        
        <div class="add-trigger">
          <div class="form-group">
            <label for="trigger-description">描述</label>
            <input
              id="trigger-description"
              v-model="newTrigger.description"
              type="text"
              class="form-control"
              placeholder="例如：外部 CI 系统"
            />
          </div>
          
          <button class="btn btn-primary" @click="addTrigger" :disabled="!newTrigger.description">
            添加触发器
          </button>
        </div>
        
        <div class="trigger-list">
          <div v-for="trigger in triggers" :key="trigger.id" class="trigger-item">
            <div class="trigger-info">
              <div class="trigger-description">{{ trigger.description }}</div>
              <div class="trigger-token">
                <code>{{ trigger.token_preview }}...</code>
                <button class="btn-copy" @click="copyTriggerToken(trigger)" title="复制完整 Token">
                  📋
                </button>
              </div>
              <div class="trigger-meta">
                创建于 {{ formatDate(trigger.created_at) }}
                <span v-if="trigger.last_used_at">· 最后使用 {{ formatDate(trigger.last_used_at) }}</span>
              </div>
            </div>
            <div class="trigger-actions">
              <button class="btn btn-danger btn-sm" @click="deleteTrigger(trigger.id)">
                撤销
              </button>
            </div>
          </div>
          
          <div v-if="triggers.length === 0" class="empty-state">
            <p>暂无流水线触发器</p>
          </div>
        </div>
        
        <div class="trigger-usage" v-if="triggers.length > 0">
          <h4>使用方法</h4>
          <pre><code>curl -X POST \
  -F token=YOUR_TRIGGER_TOKEN \
  -F ref=main \
  {{ apiBaseUrl }}/api/v1/projects/{{ project?.owner_name }}/{{ project?.name }}/trigger/pipeline</code></pre>
        </div>
      </section>

      <!-- Auto DevOps -->
      <section class="settings-section">
        <h3>Auto DevOps</h3>
        <p class="section-description">自动配置 CI/CD 流水线</p>
        
        <div class="form-group">
          <label class="checkbox-label">
            <input type="checkbox" v-model="autoDevOps.enabled" @change="saveAutoDevOps">
            启用 Auto DevOps
          </label>
          <p class="form-help">自动检测项目类型并配置合适的流水线</p>
        </div>
        
        <template v-if="autoDevOps.enabled">
          <div class="form-group">
            <label for="deploy-strategy">部署策略</label>
            <select id="deploy-strategy" v-model="autoDevOps.deploy_strategy" class="form-control" @change="saveAutoDevOps">
              <option value="continuous">持续部署到生产环境</option>
              <option value="manual">手动部署到生产环境</option>
              <option value="timed_incremental">增量部署 (分批)</option>
            </select>
          </div>
        </template>
      </section>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, watch, computed } from 'vue'
import api from '@/api'
import type { Project, Runner } from '@/types'

interface CiVariable {
  id: string
  key: string
  value?: string
  protected: boolean
  masked: boolean
  file: boolean
  environment_scope: string
  created_at: string
}

interface PipelineTrigger {
  id: string
  description: string
  token_preview: string
  created_at: string
  last_used_at?: string
}

const props = defineProps<{
  project?: Project
}>()

const loading = ref(false)
const variables = ref<CiVariable[]>([])
const projectRunners = ref<Runner[]>([])
const triggers = ref<PipelineTrigger[]>([])

const apiBaseUrl = computed(() => window.location.origin)

const projectPath = computed(() => {
  if (!props.project?.owner_name || !props.project?.name) return ''
  return `/${props.project.owner_name}/${props.project.name}`
})

const generalSettings = reactive({
  pipelines_enabled: true,
  default_timeout: 3600,
  auto_cancel_pending_pipelines: true,
  skip_outdated_deployment_jobs: false
})

const runnerSettings = reactive({
  shared_runners_enabled: true,
  group_runners_enabled: true
})

const autoDevOps = reactive({
  enabled: false,
  deploy_strategy: 'continuous' as 'continuous' | 'manual' | 'timed_incremental'
})

const newVariable = reactive({
  key: '',
  value: '',
  protected: false,
  masked: false,
  file: false,
  environment_scope: '*'
})

const newTrigger = reactive({
  description: ''
})

function formatDate(dateStr: string): string {
  return new Date(dateStr).toLocaleDateString('zh-CN')
}

function validateVariableKey() {
  // 只允许大写字母、数字和下划线
  newVariable.key = newVariable.key.toUpperCase().replace(/[^A-Z0-9_]/g, '')
}

async function loadSettings() {
  if (!props.project?.owner_name || !props.project?.name) return
  loading.value = true
  
  const path = { namespace: props.project.owner_name, project: props.project.name }
  
  try {
    // 加载 runners
    const runners = await api.runners.projectList(path)
    projectRunners.value = runners
    
    // 变量和触发器需要对应的 API 端点
    // variables.value = await api.ciVariables.list(path)
    // triggers.value = await api.triggers.list(path)
  } catch (error) {
    console.error('Failed to load settings:', error)
  } finally {
    loading.value = false
  }
}

async function saveGeneralSettings() {
  // TODO: 后端 API 尚未支持 CI/CD 通用设置
  // 需要在后端 UpdateProjectRequest 添加相应字段
  console.log('General settings:', generalSettings)
  alert('CI/CD 通用设置即将支持')
}

async function saveRunnerSettings() {
  // TODO: 后端 API 尚未支持 Runner 设置
  console.log('Runner settings:', runnerSettings)
  alert('Runner 设置即将支持')
}

async function saveAutoDevOps() {
  // TODO: 后端 API 尚未支持 Auto DevOps 设置
  console.log('Auto DevOps settings:', autoDevOps)
  alert('Auto DevOps 设置即将支持')
}

async function addVariable() {
  if (!props.project?.owner_name || !props.project?.name) return
  if (!newVariable.key || !newVariable.value) return
  
  try {
    // await api.ciVariables.create(path, newVariable)
    alert('CI/CD 变量功能即将实现')
    
    // 重置表单
    newVariable.key = ''
    newVariable.value = ''
    newVariable.protected = false
    newVariable.masked = false
    newVariable.file = false
    newVariable.environment_scope = '*'
  } catch (error) {
    console.error('Failed to add variable:', error)
    alert('添加变量失败')
  }
}

function editVariable(variable: CiVariable) {
  newVariable.key = variable.key
  newVariable.protected = variable.protected
  newVariable.masked = variable.masked
  newVariable.file = variable.file
  newVariable.environment_scope = variable.environment_scope
}

async function deleteVariable(variableId: string) {
  if (!props.project?.owner_name || !props.project?.name) return
  if (!confirm('确定要删除此变量吗？')) return
  
  try {
    // await api.ciVariables.delete(path, variableId)
    variables.value = variables.value.filter(v => v.id !== variableId)
  } catch (error) {
    console.error('Failed to delete variable:', error)
  }
}

async function addTrigger() {
  if (!props.project?.owner_name || !props.project?.name) return
  if (!newTrigger.description) return
  
  try {
    // await api.triggers.create(path, newTrigger)
    alert('流水线触发器功能即将实现')
    newTrigger.description = ''
  } catch (error) {
    console.error('Failed to add trigger:', error)
    alert('添加触发器失败')
  }
}

async function deleteTrigger(triggerId: string) {
  if (!props.project?.owner_name || !props.project?.name) return
  if (!confirm('确定要撤销此触发器吗？')) return
  
  try {
    // await api.triggers.delete(path, triggerId)
    triggers.value = triggers.value.filter(t => t.id !== triggerId)
  } catch (error) {
    console.error('Failed to delete trigger:', error)
  }
}

function copyTriggerToken(trigger: PipelineTrigger) {
  // 实际需要先获取完整 token
  navigator.clipboard.writeText(trigger.token_preview)
  alert('已复制到剪贴板')
}

watch([() => props.project?.owner_name, () => props.project?.name], () => {
  loadSettings()
}, { immediate: true })
</script>

<style lang="scss" scoped>
.project-settings-page {
  padding: $spacing-lg;
  max-width: 900px;
}

.settings-header {
  margin-bottom: $spacing-xl;
  
  h2 {
    margin: 0 0 $spacing-xs 0;
  }
  
  .description {
    color: $text-muted;
    margin: 0;
  }
}

.settings-section {
  margin-bottom: $spacing-xl;
  padding-bottom: $spacing-xl;
  border-bottom: 1px solid $border-color;
  
  &:last-child {
    border-bottom: none;
  }
  
  h3 {
    margin-bottom: $spacing-sm;
  }
  
  h4 {
    margin: $spacing-lg 0 $spacing-sm 0;
    font-size: $font-size-base;
  }
  
  .section-description {
    color: $text-muted;
    margin-bottom: $spacing-lg;
  }
}

.form-group {
  margin-bottom: $spacing-md;
  
  label {
    display: block;
    margin-bottom: $spacing-xs;
    font-weight: 500;
  }
  
  .form-help {
    font-size: $font-size-sm;
    color: $text-muted;
    margin-top: $spacing-xs;
  }
}

.checkbox-label {
  display: flex;
  align-items: center;
  gap: $spacing-sm;
  cursor: pointer;
  font-weight: normal !important;
  
  .option-hint {
    font-size: $font-size-sm;
    color: $text-muted;
    margin-left: $spacing-xs;
  }
}

.timeout-input {
  display: flex;
  align-items: center;
  gap: $spacing-sm;
  
  input {
    width: 150px;
  }
  
  .timeout-unit {
    color: $text-muted;
  }
}

.add-variable, .add-trigger {
  margin-bottom: $spacing-lg;
  padding: $spacing-lg;
  background: $bg-secondary;
  border-radius: $border-radius;
}

.variable-form {
  margin-bottom: $spacing-md;
}

.form-row {
  display: flex;
  gap: $spacing-md;
  margin-bottom: $spacing-md;
  
  .form-group {
    margin-bottom: 0;
    
    &.flex-grow {
      flex: 1;
    }
  }
}

.options-row {
  flex-wrap: wrap;
  
  .checkbox-label {
    flex: 0 0 auto;
  }
}

.variable-list, .trigger-list, .runner-list {
  display: flex;
  flex-direction: column;
  gap: $spacing-sm;
}

.variable-item, .trigger-item, .runner-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: $spacing-md;
  background: $bg-secondary;
  border-radius: $border-radius;
}

.variable-info, .trigger-info, .runner-info {
  flex: 1;
}

.variable-key, .runner-name {
  display: flex;
  align-items: center;
  gap: $spacing-sm;
  margin-bottom: $spacing-xs;
  
  code {
    font-weight: 600;
    padding: 2px 6px;
    background: $bg-tertiary;
    border-radius: 3px;
  }
}

.variable-meta, .trigger-meta, .runner-meta {
  font-size: $font-size-sm;
  color: $text-muted;
  display: flex;
  gap: $spacing-md;
}

.trigger-description {
  font-weight: 500;
  margin-bottom: $spacing-xs;
}

.trigger-token {
  display: flex;
  align-items: center;
  gap: $spacing-sm;
  margin-bottom: $spacing-xs;
  
  code {
    font-family: monospace;
    padding: 2px 6px;
    background: $bg-tertiary;
    border-radius: 3px;
  }
}

.btn-copy {
  background: none;
  border: none;
  cursor: pointer;
  padding: 2px;
  
  &:hover {
    opacity: 0.7;
  }
}

.variable-actions, .trigger-actions, .runner-actions {
  display: flex;
  gap: $spacing-sm;
}

.badge {
  font-size: $font-size-xs;
  padding: 2px 6px;
  border-radius: 3px;
  
  &.badge-info {
    background: rgba($primary-color, 0.2);
    color: $primary-color;
  }
  
  &.badge-secondary {
    background: $bg-tertiary;
    color: $text-muted;
  }
}

.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  display: inline-block;
  
  &.online, &.idle {
    background: $success-color;
  }
  
  &.offline {
    background: $text-muted;
  }
  
  &.running {
    background: $primary-color;
  }
}

.runner-settings {
  margin-bottom: $spacing-lg;
}

.trigger-usage {
  margin-top: $spacing-lg;
  padding: $spacing-md;
  background: $bg-secondary;
  border-radius: $border-radius;
  
  h4 {
    margin-top: 0;
  }
  
  pre {
    margin: 0;
    padding: $spacing-md;
    background: $bg-tertiary;
    border-radius: $border-radius;
    overflow-x: auto;
    
    code {
      font-family: monospace;
      font-size: $font-size-sm;
    }
  }
}

.empty-state {
  padding: $spacing-lg;
  text-align: center;
  color: $text-muted;
  
  p {
    margin: $spacing-xs 0;
  }
  
  .btn {
    margin-top: $spacing-md;
  }
}

.text-muted {
  color: $text-muted;
}

.mb-3 {
  margin-bottom: $spacing-md;
}

.mt-2 {
  margin-top: $spacing-sm;
}
</style>
