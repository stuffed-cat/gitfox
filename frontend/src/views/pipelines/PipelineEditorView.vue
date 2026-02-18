<template>
  <div class="ci-editor-page">
    <div class="editor-toolbar">
      <div class="branch-selector">
        <svg viewBox="0 0 16 16" width="14" height="14">
          <path :d="icons.branch" fill="currentColor"/>
        </svg>
        <span>{{ currentBranch }}</span>
        <svg viewBox="0 0 16 16" width="12" height="12">
          <path :d="icons.chevronDown" stroke="currentColor" stroke-width="1.5" fill="none"/>
        </svg>
      </div>
    </div>

    <div v-if="loading" class="loading-state">
      <div class="loading-spinner"></div>
    </div>

    <div v-else-if="!hasConfig" class="empty-state">
      <div class="empty-icon">
        <svg viewBox="0 0 64 64" width="80" height="80" fill="none">
          <circle cx="32" cy="32" r="28" fill="#ede8fb"/>
          <circle cx="20" cy="26" r="7" stroke="#6b4fbb" stroke-width="2"/>
          <circle cx="44" cy="26" r="7" stroke="#1a73e8" stroke-width="2" fill="white"/>
          <circle cx="32" cy="38" r="5" stroke="#6b4fbb" stroke-width="2" fill="white"/>
          <path d="M25 26h7M27 38h-4M37 38h4" stroke="#888" stroke-width="1.5"/>
        </svg>
      </div>
      <h2>Configure a pipeline to automate your<br/>builds, tests, and deployments</h2>
      <p>Create a <code>.gitfox/ci/</code> directory in your repository to configure and run your first pipeline.</p>
      <router-link :to="`/${project?.owner_name}/${project?.name}/-/pipelines`" class="btn btn-primary">
        配置流水线
      </router-link>
    </div>

    <div v-else class="config-viewer">
      <div class="config-info">
        <svg viewBox="0 0 16 16" width="14" height="14">
          <path :d="icons.file" fill="currentColor"/>
        </svg>
        <span>已找到 <strong>{{ configFiles.length }}</strong> 个 CI/CD 配置文件（<code>.gitfox/ci/</code>）</span>
      </div>
      <div class="config-files">
        <div v-for="file in configFiles" :key="file" class="config-file-item">
          <svg viewBox="0 0 16 16" width="14" height="14">
            <path :d="icons.file" fill="currentColor"/>
          </svg>
          <span>{{ file }}</span>
        </div>
      </div>
      <div class="config-actions">
        <router-link :to="`/${project?.owner_name}/${project?.name}/-/tree/.gitfox/ci`" class="btn">
          查看配置文件
        </router-link>
        <router-link :to="`/${project?.owner_name}/${project?.name}/-/pipelines`" class="btn btn-primary">
          运行流水线
        </router-link>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import api from '@/api'
import type { Project } from '@/types'
import { navIcons } from '@/navigation/icons'

const props = defineProps<{ project?: Project }>()
const icons = navIcons

const loading = ref(false)
const hasConfig = ref(false)
const configFiles = ref<string[]>([])
const currentBranch = ref('main')

async function loadEditorInfo() {
  if (!props.project?.owner_name || !props.project?.name) return
  loading.value = true
  try {
    // 尝试读取 .gitfox/ci 目录
    const files = await api.repository.browseTree(
      { namespace: props.project.owner_name, project: props.project.name },
      '.gitfox/ci',
      currentBranch.value
    )
    hasConfig.value = files && files.length > 0
    configFiles.value = (files || [])
      .filter((f: any) => f.type === 'blob' || f.entry_type === 'file')
      .map((f: any) => f.name)
  } catch {
    hasConfig.value = false
    configFiles.value = []
  } finally {
    loading.value = false
  }
}

watch([() => props.project?.owner_name, () => props.project?.name], () => {
  loadEditorInfo()
}, { immediate: true })
</script>

<style lang="scss" scoped>
.ci-editor-page { padding: 0; }

.editor-toolbar {
  padding: 12px 16px;
  border-bottom: 1px solid $border-color;
  display: flex;
  align-items: center;
}

.branch-selector {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 6px 12px;
  border: 1px solid $border-color;
  border-radius: 6px;
  font-size: 13px;
  color: $text-primary;
  cursor: pointer;
  background: $bg-primary;

  &:hover { background: $bg-secondary; }
}

.loading-state {
  display: flex;
  justify-content: center;
  padding: 80px;

  .loading-spinner {
    width: 32px;
    height: 32px;
    border: 3px solid $border-color;
    border-top-color: $primary-color;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }
}

@keyframes spin { to { transform: rotate(360deg); } }

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 80px 24px;
  text-align: center;

  .empty-icon { margin-bottom: 24px; }

  h2 {
    font-size: 20px;
    font-weight: 600;
    color: $text-primary;
    margin: 0 0 12px;
    line-height: 1.4;
  }

  p {
    font-size: 14px;
    color: $text-secondary;
    margin: 0 0 24px;
    line-height: 1.6;

    code {
      background: $bg-tertiary;
      padding: 1px 6px;
      border-radius: 4px;
      font-family: monospace;
      font-size: 13px;
    }
  }
}

.config-viewer {
  padding: 24px;

  .config-info {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 14px;
    color: $text-secondary;
    margin-bottom: 16px;

    code {
      background: $bg-tertiary;
      padding: 1px 6px;
      border-radius: 4px;
      font-family: monospace;
      font-size: 12px;
    }
  }

  .config-files {
    border: 1px solid $border-color;
    border-radius: 6px;
    overflow: hidden;
    margin-bottom: 20px;

    .config-file-item {
      display: flex;
      align-items: center;
      gap: 8px;
      padding: 10px 16px;
      font-size: 13px;
      color: $text-primary;
      border-bottom: 1px solid $border-color;
      font-family: monospace;

      &:last-child { border-bottom: none; }
    }
  }

  .config-actions {
    display: flex;
    gap: 12px;
  }
}
</style>
