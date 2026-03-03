<template>
  <div class="pipeline-visualization">
    <!-- 错误状态 -->
    <div v-if="error" class="viz-error">
      <div class="error-icon">
        <svg viewBox="0 0 16 16" width="24" height="24">
          <path d="M6.457 1.047c.659-1.234 2.427-1.234 3.086 0l6.082 11.378A1.75 1.75 0 0 1 14.082 15H1.918a1.75 1.75 0 0 1-1.543-2.575Zm1.763.707a.25.25 0 0 0-.44 0L1.698 13.132a.25.25 0 0 0 .22.368h12.164a.25.25 0 0 0 .22-.368Zm.53 3.996v2.5a.75.75 0 0 1-1.5 0v-2.5a.75.75 0 0 1 1.5 0ZM9 11a1 1 0 1 1-2 0 1 1 0 0 1 2 0Z" fill="currentColor"/>
        </svg>
      </div>
      <h3>配置解析失败</h3>
      <p>{{ error }}</p>
    </div>

    <!-- 空状态 -->
    <div v-else-if="!config || !hasJobs" class="viz-empty">
      <div class="empty-icon">
        <svg viewBox="0 0 16 16" width="48" height="48">
          <path d="M8.5.75a.75.75 0 0 0-1.5 0v5.19L4.72 3.66a.75.75 0 0 0-1.06 1.06l3.5 3.5a.75.75 0 0 0 1.06 0l3.5-3.5a.75.75 0 0 0-1.06-1.06L8.5 5.94V.75ZM1.5 10a.75.75 0 0 0 0 1.5h13a.75.75 0 0 0 0-1.5h-13Z" fill="currentColor"/>
        </svg>
      </div>
      <h3>没有可视化的内容</h3>
      <p>在编辑器中添加 stages 和 jobs 来查看流水线可视化</p>
    </div>

    <!-- 可视化内容 -->
    <div v-else class="viz-content">
      <!-- Stages 横向布局 -->
      <div class="stages-container">
        <div 
          v-for="(stage, index) in stages" 
          :key="stage"
          class="stage-column"
        >
          <div class="stage-header">
            <span class="stage-name">{{ stage }}</span>
            <span class="stage-index">{{ index + 1 }}</span>
          </div>
          
          <div class="stage-jobs">
            <div 
              v-for="job in getJobsForStage(stage)" 
              :key="job.name"
              class="job-card"
              :class="[
                job.when === 'manual' ? 'manual' : '',
                job.allow_failure ? 'allow-failure' : ''
              ]"
            >
              <div class="job-header">
                <span class="job-name">{{ job.name }}</span>
                <span v-if="job.when === 'manual'" class="job-badge manual">手动</span>
                <span v-if="job.allow_failure" class="job-badge warning">允许失败</span>
              </div>
              
              <div class="job-details">
                <div v-if="job.needs?.length" class="job-needs">
                  <span class="detail-label">依赖:</span>
                  <span class="detail-value">{{ job.needs.join(', ') }}</span>
                </div>
                <div v-if="job.tags?.length" class="job-tags">
                  <span class="detail-label">标签:</span>
                  <span class="tag" v-for="tag in job.tags" :key="tag">{{ tag }}</span>
                </div>
                <div v-if="job.only?.length" class="job-only">
                  <span class="detail-label">仅:</span>
                  <span class="detail-value">{{ job.only.join(', ') }}</span>
                </div>
              </div>

              <div v-if="job.script?.length" class="job-script-preview">
                <code>{{ job.script[0] }}{{ job.script.length > 1 ? '...' : '' }}</code>
              </div>
            </div>
          </div>

          <!-- Stage 连接线 -->
          <div v-if="index < stages.length - 1" class="stage-connector">
            <svg width="40" height="24">
              <line x1="0" y1="12" x2="40" y2="12" stroke="currentColor" stroke-width="2"/>
              <polygon points="35,7 40,12 35,17" fill="currentColor"/>
            </svg>
          </div>
        </div>
      </div>

      <!-- 全局配置预览 -->
      <div v-if="hasGlobalConfig" class="global-config">
        <div class="config-section" v-if="config.variables && Object.keys(config.variables).length">
          <h4>全局变量</h4>
          <div class="config-items">
            <div v-for="(value, key) in config.variables" :key="key" class="config-item">
              <span class="key">{{ key }}</span>
              <span class="value">{{ value }}</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'

interface JobConfig {
  name: string
  stage?: string
  script?: string[]
  before_script?: string[]
  after_script?: string[]
  needs?: string[]
  tags?: string[]
  only?: string[]
  except?: string[]
  when?: string
  allow_failure?: boolean
  artifacts?: any
  cache?: any
  variables?: Record<string, string>
  timeout?: number
  retry?: number | { max?: number; when?: string[] }
  environment?: { name: string; url?: string }
}

interface Props {
  config: any
  error?: string
}

const props = defineProps<Props>()

// 提取 stages
const stages = computed<string[]>(() => {
  if (!props.config) return []
  
  // 如果配置中有 stages 字段
  if (Array.isArray(props.config.stages)) {
    return props.config.stages
  }
  
  // 从 jobs 中推断 stages
  const inferredStages = new Set<string>()
  for (const [key, value] of Object.entries(props.config)) {
    if (isJob(key, value)) {
      const job = value as any
      inferredStages.add(job.stage || 'test')
    }
  }
  
  // 默认顺序：build, test, deploy
  const defaultOrder = ['build', 'test', 'deploy']
  const sorted = [...inferredStages].sort((a, b) => {
    const indexA = defaultOrder.indexOf(a)
    const indexB = defaultOrder.indexOf(b)
    if (indexA === -1 && indexB === -1) return a.localeCompare(b)
    if (indexA === -1) return 1
    if (indexB === -1) return -1
    return indexA - indexB
  })
  
  return sorted
})

// 提取所有 jobs
const jobs = computed<JobConfig[]>(() => {
  if (!props.config) return []
  
  const result: JobConfig[] = []
  for (const [key, value] of Object.entries(props.config)) {
    if (isJob(key, value)) {
      result.push({
        name: key,
        ...(value as object)
      } as JobConfig)
    }
  }
  return result
})

const hasJobs = computed(() => jobs.value.length > 0)

const hasGlobalConfig = computed(() => {
  if (!props.config) return false
  return props.config.variables && Object.keys(props.config.variables).length > 0
})

// 判断是否是 job 配置
function isJob(key: string, value: any): boolean {
  // 排除保留字段
  const reserved = [
    'stages', 'variables', 'before_script', 'after_script',
    'image', 'services', 'cache', 'workflow', 'include', 'default'
  ]
  if (reserved.includes(key)) return false
  
  // job 必须是对象且包含 script 或 trigger
  if (!value || typeof value !== 'object') return false
  return Array.isArray(value.script) || value.trigger || value.extends
}

// 获取某个 stage 的所有 jobs
function getJobsForStage(stage: string): JobConfig[] {
  return jobs.value.filter(job => (job.stage || 'test') === stage)
}
</script>

<style lang="scss" scoped>
.pipeline-visualization {
  min-height: 400px;
}

// 错误状态
.viz-error {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 60px 24px;
  text-align: center;

  .error-icon {
    width: 48px;
    height: 48px;
    display: flex;
    align-items: center;
    justify-content: center;
    margin-bottom: 16px;
    background: $color-danger-light;
    border-radius: 50%;
    color: $color-danger;
  }

  h3 {
    font-size: 16px;
    font-weight: 600;
    color: $text-primary;
    margin: 0 0 8px;
  }

  p {
    font-size: 13px;
    color: $text-secondary;
    margin: 0;
    font-family: $font-mono;
    background: $bg-tertiary;
    padding: 8px 12px;
    border-radius: 6px;
    max-width: 500px;
    word-break: break-all;
  }
}

// 空状态
.viz-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 60px 24px;
  text-align: center;
  color: $text-secondary;

  .empty-icon {
    margin-bottom: 16px;
    opacity: 0.5;
  }

  h3 {
    font-size: 16px;
    font-weight: 600;
    color: $text-primary;
    margin: 0 0 8px;
  }

  p {
    font-size: 14px;
    color: $text-secondary;
    margin: 0;
  }
}

// 可视化内容
.viz-content {
  padding: 24px 0;
}

// Stages 容器
.stages-container {
  display: flex;
  align-items: flex-start;
  gap: 0;
  overflow-x: auto;
  padding: 0 24px 24px;
}

// Stage 列
.stage-column {
  position: relative;
  min-width: 220px;
  flex-shrink: 0;
}

.stage-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 16px;
  background: $bg-tertiary;
  border-radius: 8px;
  margin-bottom: 16px;

  .stage-name {
    font-size: 14px;
    font-weight: 600;
    color: $text-primary;
    text-transform: capitalize;
  }

  .stage-index {
    width: 24px;
    height: 24px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: $primary-color;
    color: white;
    border-radius: 50%;
    font-size: 12px;
    font-weight: 600;
  }
}

// Stage jobs
.stage-jobs {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

// Job 卡片
.job-card {
  background: $bg-primary;
  border: 1px solid $border-color;
  border-radius: 8px;
  padding: 12px;
  transition: all 0.2s;

  &:hover {
    border-color: $primary-color;
    box-shadow: 0 2px 8px rgba($primary-color, 0.15);
  }

  &.manual {
    border-left: 3px solid $color-info;
  }

  &.allow-failure {
    border-left: 3px solid $color-warning;
  }
}

.job-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 8px;

  .job-name {
    font-size: 13px;
    font-weight: 600;
    color: $text-primary;
    font-family: $font-mono;
  }

  .job-badge {
    padding: 2px 6px;
    font-size: 10px;
    font-weight: 500;
    border-radius: 4px;
    text-transform: uppercase;

    &.manual {
      background: $color-info-light;
      color: $color-info;
    }

    &.warning {
      background: $color-warning-light;
      color: $color-warning;
    }
  }
}

.job-details {
  display: flex;
  flex-direction: column;
  gap: 6px;
  font-size: 12px;

  .detail-label {
    color: $text-secondary;
    margin-right: 4px;
  }

  .detail-value {
    color: $text-primary;
    font-family: $font-mono;
    font-size: 11px;
  }

  .job-tags {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 4px;

    .tag {
      padding: 1px 6px;
      background: $bg-tertiary;
      border-radius: 4px;
      font-size: 11px;
      font-family: $font-mono;
      color: $text-secondary;
    }
  }
}

.job-script-preview {
  margin-top: 8px;
  padding: 6px 8px;
  background: $bg-tertiary;
  border-radius: 4px;

  code {
    font-size: 11px;
    font-family: $font-mono;
    color: $text-secondary;
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
}

// Stage 连接线
.stage-connector {
  position: absolute;
  right: -20px;
  top: 24px;
  color: $border-color;
}

// 全局配置
.global-config {
  margin-top: 32px;
  padding: 0 24px;
  border-top: 1px solid $border-color;
  padding-top: 24px;

  .config-section {
    h4 {
      font-size: 13px;
      font-weight: 600;
      color: $text-secondary;
      margin: 0 0 12px;
      text-transform: uppercase;
      letter-spacing: 0.5px;
    }
  }

  .config-items {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .config-item {
    display: inline-flex;
    align-items: center;
    padding: 4px 8px;
    background: $bg-tertiary;
    border-radius: 4px;
    font-size: 12px;
    font-family: $font-mono;

    .key {
      color: $purple-600;
      margin-right: 4px;

      &::after {
        content: ':';
        color: $text-secondary;
      }
    }

    .value {
      color: $text-primary;
    }
  }
}
</style>
