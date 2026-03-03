<template>
  <div class="ci-editor-page">
    <!-- 顶部工具栏 -->
    <div class="editor-toolbar">
      <div class="toolbar-left">
        <!-- 分支选择器 -->
        <div class="branch-selector" @click="showBranchDropdown = !showBranchDropdown">
          <svg viewBox="0 0 16 16" width="14" height="14">
            <path :d="icons.branch" fill="currentColor"/>
          </svg>
          <span>{{ currentBranch }}</span>
          <svg viewBox="0 0 16 16" width="12" height="12">
            <path :d="icons.chevronDown" stroke="currentColor" stroke-width="1.5" fill="none"/>
          </svg>
          <div v-if="showBranchDropdown" class="dropdown-menu" @click.stop>
            <div class="dropdown-search">
              <input v-model="branchSearch" placeholder="搜索分支..." @input="filterBranches" />
            </div>
            <div class="dropdown-items">
              <div 
                v-for="branch in filteredBranches" 
                :key="branch.name" 
                class="dropdown-item"
                :class="{ active: branch.name === currentBranch }"
                @click="selectBranch(branch.name)"
              >
                {{ branch.name }}
                <span v-if="branch.is_default" class="badge">默认</span>
              </div>
            </div>
          </div>
        </div>

        <!-- 文件选择器 -->
        <div v-if="hasConfig" class="file-selector" @click="showFileDropdown = !showFileDropdown">
          <svg viewBox="0 0 16 16" width="14" height="14">
            <path :d="icons.file" fill="currentColor"/>
          </svg>
          <span>{{ currentFile || '选择文件' }}</span>
          <svg viewBox="0 0 16 16" width="12" height="12">
            <path :d="icons.chevronDown" stroke="currentColor" stroke-width="1.5" fill="none"/>
          </svg>
          <div v-if="showFileDropdown" class="dropdown-menu" @click.stop>
            <div class="dropdown-items">
              <div 
                v-for="file in configFiles" 
                :key="file" 
                class="dropdown-item"
                :class="{ active: file === currentFile }"
                @click="selectFile(file)"
              >
                {{ file }}
              </div>
              <div class="dropdown-divider"></div>
              <div class="dropdown-item create-new" @click="showCreateFileModal = true">
                <svg viewBox="0 0 16 16" width="14" height="14">
                  <path d="M8 2v12M2 8h12" stroke="currentColor" stroke-width="2" fill="none"/>
                </svg>
                新建配置文件
              </div>
            </div>
          </div>
        </div>
      </div>

      <div class="toolbar-right">
        <!-- 验证状态 -->
        <div v-if="validationStatus" class="validation-status" :class="validationStatus">
          <svg v-if="validationStatus === 'valid'" viewBox="0 0 16 16" width="14" height="14">
            <path d="M13.78 4.22a.75.75 0 0 1 0 1.06l-7.25 7.25a.75.75 0 0 1-1.06 0L2.22 9.28a.75.75 0 0 1 1.06-1.06L6 10.94l6.72-6.72a.75.75 0 0 1 1.06 0Z" fill="currentColor"/>
          </svg>
          <svg v-else-if="validationStatus === 'invalid'" viewBox="0 0 16 16" width="14" height="14">
            <path d="M3.72 3.72a.75.75 0 0 1 1.06 0L8 6.94l3.22-3.22a.75.75 0 1 1 1.06 1.06L9.06 8l3.22 3.22a.75.75 0 1 1-1.06 1.06L8 9.06l-3.22 3.22a.75.75 0 0 1-1.06-1.06L6.94 8 3.72 4.78a.75.75 0 0 1 0-1.06Z" fill="currentColor"/>
          </svg>
          <span>{{ validationMessage }}</span>
        </div>

        <!-- 模板按钮 -->
        <button class="btn btn-secondary" @click="showTemplateModal = true">
          <svg viewBox="0 0 16 16" width="14" height="14">
            <path d="M2 3.5A1.5 1.5 0 0 1 3.5 2h2.764c.958 0 1.76.56 2.311 1.184C9.126 3.81 9.979 4 10.5 4h1a1 1 0 1 1 0 2h-1c-.554 0-1.474-.204-2.299-1.034C7.724 4.479 7.315 4 6.264 4H3.5a.5.5 0 0 0-.5.5v7a.5.5 0 0 0 .5.5h9a.5.5 0 0 0 .5-.5V8a1 1 0 1 1 2 0v3.5a2.5 2.5 0 0 1-2.5 2.5h-9A2.5 2.5 0 0 1 1 11.5v-8Z" fill="currentColor"/>
          </svg>
          模板
        </button>

        <!-- 保存按钮 -->
        <button 
          class="btn btn-primary" 
          :disabled="!hasChanges || saving"
          @click="saveFile"
        >
          <svg v-if="saving" class="spinner" viewBox="0 0 16 16" width="14" height="14">
            <circle cx="8" cy="8" r="6" stroke="currentColor" stroke-width="2" fill="none" stroke-dasharray="30" stroke-linecap="round"/>
          </svg>
          <span v-else>{{ isNewFile ? '创建文件' : '提交更改' }}</span>
        </button>
      </div>
    </div>

    <!-- 标签页 -->
    <div class="editor-tabs">
      <button 
        v-for="tab in tabs" 
        :key="tab.id"
        class="tab-btn"
        :class="{ active: activeTab === tab.id }"
        @click="activeTab = tab.id"
      >
        {{ tab.label }}
      </button>
    </div>

    <!-- 内容区域 -->
    <div class="editor-content">
      <!-- 加载状态 -->
      <div v-if="loading" class="loading-state">
        <div class="loading-spinner"></div>
        <span>加载中...</span>
      </div>

      <!-- 空状态：没有 CI 配置 -->
      <div v-else-if="!hasConfig && !isNewFile" class="empty-state">
        <div class="empty-icon">
          <svg viewBox="0 0 64 64" width="80" height="80" fill="none">
            <circle cx="32" cy="32" r="28" fill="#ede8fb"/>
            <circle cx="20" cy="26" r="7" stroke="#6b4fbb" stroke-width="2"/>
            <circle cx="44" cy="26" r="7" stroke="#1a73e8" stroke-width="2" fill="white"/>
            <circle cx="32" cy="38" r="5" stroke="#6b4fbb" stroke-width="2" fill="white"/>
            <path d="M25 26h7M27 38h-4M37 38h4" stroke="#888" stroke-width="1.5"/>
          </svg>
        </div>
        <h2>配置流水线以自动化您的<br/>构建、测试和部署</h2>
        <p>在仓库中创建 <code>.gitfox/ci/</code> 目录来配置和运行您的第一个流水线。</p>
        <div class="empty-actions">
          <button class="btn btn-primary" @click="createFirstConfig">
            创建 CI 配置
          </button>
          <button class="btn btn-secondary" @click="showTemplateModal = true">
            从模板开始
          </button>
        </div>
      </div>

      <!-- 编辑器主体内容 -->
      <template v-else>
        <!-- 编辑标签页 -->
        <div v-show="activeTab === 'edit'" class="tab-content edit-tab">
          <div class="editor-wrapper">
            <MonacoEditor
              ref="editorRef"
              v-model="editorContent"
              language="yaml"
              :height="editorHeight"
              :minimap="true"
              @change="onEditorChange"
              @save="saveFile"
            />
          </div>
        </div>

        <!-- 可视化标签页 -->
        <div v-show="activeTab === 'visualize'" class="tab-content visualize-tab">
          <PipelineVisualization 
            :config="parsedConfig" 
            :error="parseError"
          />
        </div>

        <!-- 完整配置标签页 -->
        <div v-show="activeTab === 'merged'" class="tab-content merged-tab">
          <div class="merged-config-header">
            <svg viewBox="0 0 16 16" width="14" height="14">
              <path d="M8 1.5a6.5 6.5 0 1 0 0 13 6.5 6.5 0 0 0 0-13ZM0 8a8 8 0 1 1 16 0A8 8 0 0 1 0 8Zm6.5-.25A.75.75 0 0 1 7.25 7h1a.75.75 0 0 1 .75.75v2.75h.25a.75.75 0 0 1 0 1.5h-2a.75.75 0 0 1 0-1.5h.25v-2h-.25a.75.75 0 0 1-.75-.75ZM8 6a1 1 0 1 1 0-2 1 1 0 0 1 0 2Z" fill="currentColor"/>
            </svg>
            <span>这是所有配置文件合并后的完整视图（只读）</span>
          </div>
          <div class="merged-editor-wrapper">
            <MonacoEditor
              v-model="mergedConfig"
              language="yaml"
              :height="editorHeight"
              :readonly="true"
              :minimap="true"
            />
          </div>
        </div>
      </template>
    </div>

    <!-- 创建文件模态框 -->
    <div v-if="showCreateFileModal" class="modal-overlay" @click.self="showCreateFileModal = false">
      <div class="modal-content">
        <div class="modal-header">
          <h3>创建新的 CI 配置文件</h3>
          <button class="close-btn" @click="showCreateFileModal = false">&times;</button>
        </div>
        <div class="modal-body">
          <div class="form-group">
            <label>文件名</label>
            <div class="input-with-prefix">
              <span class="input-prefix">.gitfox/ci/</span>
              <input v-model="newFileName" placeholder="例如: build.yml" />
            </div>
            <p class="help-text">文件名应以 .yml 或 .yaml 结尾</p>
          </div>
        </div>
        <div class="modal-footer">
          <button class="btn btn-secondary" @click="showCreateFileModal = false">取消</button>
          <button class="btn btn-primary" @click="createNewFile" :disabled="!isValidFileName">创建</button>
        </div>
      </div>
    </div>

    <!-- 模板选择模态框 -->
    <div v-if="showTemplateModal" class="modal-overlay" @click.self="showTemplateModal = false">
      <div class="modal-content modal-large">
        <div class="modal-header">
          <h3>选择 CI/CD 模板</h3>
          <button class="close-btn" @click="showTemplateModal = false">&times;</button>
        </div>
        <div class="modal-body">
          <div class="template-grid">
            <div 
              v-for="template in ciTemplates" 
              :key="template.id"
              class="template-card"
              :class="{ selected: selectedTemplate === template.id }"
              @click="selectedTemplate = template.id"
            >
              <div class="template-icon">
                <svg v-if="template.icon === 'rust'" viewBox="0 0 24 24" width="32" height="32">
                  <path d="M23.687 11.709l-.995-.616a13.559 13.559 0 0 0-.028-.281l.855-.779a.249.249 0 0 0-.066-.394l-.951-.606-.062-.281a.248.248 0 0 0-.227-.308l-1.23-.092a13.38 13.38 0 0 0-.161-.265l.467-1.105a.248.248 0 0 0-.187-.34l-1.168-.257a10.23 10.23 0 0 0-.172-.25l.201-.995a.248.248 0 0 0-.247-.251l-1.119-.015a9.32 9.32 0 0 0-.209-.217l-.126-1.08a.248.248 0 0 0-.297-.157l-1.028.172-.246-.187-.453-1.006a.248.248 0 0 0-.344-.066l-.889.403-.274-.136-.633-.823a.248.248 0 0 0-.382.023l-.728.678-.292-.07-.752-.652a.248.248 0 0 0-.386.045l-.547.833-.296-.009-.867-.441a.248.248 0 0 0-.359.139l-.339.994-.292.09-.97-.29a.248.248 0 0 0-.307.198l-.117 1.026-.279.132-1.032-.116a.248.248 0 0 0-.235.24l.106 1.036-.261.167-1.052.07A.248.248 0 0 0 1.69 7.35l.32.998-.236.215-.72.27a.248.248 0 0 0-.1.366l.463.924-.205.255-.22.135a.248.248 0 0 0-.032.387l.615.8-.16.29-.035.154a.248.248 0 0 0 .114.38l.815.594-.1.322.065.144a.248.248 0 0 0 .207.34l1.038.33-.023.333.159.136a.248.248 0 0 0 .257.307l1.066.054.065.338.131.127a.248.248 0 0 0 .285.26l1.065-.1.151.318.1.106a.248.248 0 0 0 .299.2l1.032-.26.222.278.111.067a.248.248 0 0 0 .303.14l.964-.407.277.221.058.03a.248.248 0 0 0 .291.073l.866-.527.312.153.004.009a.248.248 0 0 0 .268.003l.74-.603.32.079a.248.248 0 0 0 .253-.073l.589-.643.319.013a.248.248 0 0 0 .22-.131l.422-.713.311-.065a.248.248 0 0 0 .186-.175l.237-.808.303-.165a.248.248 0 0 0 .141-.214l.035-.836.287-.252a.248.248 0 0 0 .084-.24l-.199-.826.266-.326a.248.248 0 0 0 .023-.258l-.44-.756.236-.386a.248.248 0 0 0-.035-.266l-.582-.639.196-.432a.248.248 0 0 0-.088-.268l-.797-.455.148-.464a.248.248 0 0 0-.134-.261l-.955-.251.093-.485a.248.248 0 0 0-.174-.248l-1.049-.031.034-.495a.248.248 0 0 0-.207-.228l-1.09.195-.032-.491a.248.248 0 0 0-.23-.204l-1.08.382-.133-.451a.248.248 0 0 0-.243-.177l-1.016.543-.198-.394a.248.248 0 0 0-.247-.151l-.91.665-.245-.328a.248.248 0 0 0-.24-.128l-.77.742L12 7.194l-.587.776a7.285 7.285 0 0 0-3.9 6.47 7.285 7.285 0 0 0 6.61 7.24v-1.21a.238.238 0 0 0-.106-.197l-.618-.439a1.026 1.026 0 0 1-.424-1.019l.088-.482-.477-.38a.238.238 0 0 1-.048-.286l.362-.544-.308-.536a.238.238 0 0 1 .085-.313l.538-.355-.13-.603a.238.238 0 0 1 .202-.27l.611-.087.057-.591a.238.238 0 0 1 .289-.215l.595.107.227-.55a.238.238 0 0 1 .351-.118l.524.308.377-.448a.238.238 0 0 1 .38-.002l.378.447.524-.307a.238.238 0 0 1 .35.117l.228.55.595-.106a.238.238 0 0 1 .289.214l.058.59.61.088a.238.238 0 0 1 .202.269l-.13.604.538.354a.238.238 0 0 1 .084.314l-.307.536.362.544a.238.238 0 0 1-.048.285l-.478.38.089.482a1.026 1.026 0 0 1-.425 1.02l-.618.438a.238.238 0 0 0-.106.197v1.21a7.285 7.285 0 0 0 6.614-7.244 7.257 7.257 0 0 0-.31-2.104Z" fill="currentColor"/>
                </svg>
                <svg v-else-if="template.icon === 'node'" viewBox="0 0 24 24" width="32" height="32">
                  <path d="M11.998 24c-.321 0-.641-.084-.922-.247l-2.936-1.737c-.438-.245-.224-.332-.08-.383.585-.203.703-.25 1.328-.604.065-.037.151-.023.218.017l2.256 1.339c.082.045.198.045.275 0l8.795-5.076c.082-.047.134-.14.134-.238V6.921c0-.099-.053-.19-.137-.242l-8.791-5.072c-.081-.047-.189-.047-.271 0L3.075 6.68c-.085.049-.14.143-.14.242v10.15c0 .097.055.189.139.235l2.409 1.392c1.307.653 2.108-.116 2.108-.89V7.787c0-.142.114-.253.256-.253h1.115c.139 0 .255.112.255.253v10.021c0 1.745-.95 2.745-2.604 2.745-.509 0-.909 0-2.026-.551l-2.305-1.328c-.57-.329-.922-.943-.922-1.596V6.921c0-.653.352-1.267.922-1.596l8.795-5.082c.557-.318 1.296-.318 1.848 0l8.794 5.082c.57.329.924.943.924 1.596v10.15c0 .653-.354 1.264-.924 1.596l-8.794 5.078c-.28.163-.6.247-.922.247zm2.722-6.98c-3.867 0-4.677-1.776-4.677-3.265 0-.141.114-.253.256-.253h1.136c.126 0 .232.091.252.212.171 1.156.681 1.741 3.003 1.741 1.848 0 2.634-.418 2.634-1.399 0-.564-.223-.983-3.096-1.266-2.402-.236-3.888-.768-3.888-2.69 0-1.772 1.493-2.827 3.999-2.827 2.812 0 4.204.977 4.38 3.073a.257.257 0 0 1-.064.192.253.253 0 0 1-.184.081h-1.142a.253.253 0 0 1-.246-.196c-.274-1.216-.938-1.605-2.744-1.605-2.022 0-2.258.705-2.258 1.233 0 .64.278.826 3.001 1.187 2.701.358 3.985.864 3.985 2.754-.002 1.912-1.597 3.009-4.383 3.009z" fill="#68A063"/>
                </svg>
                <svg v-else-if="template.icon === 'python'" viewBox="0 0 24 24" width="32" height="32">
                  <path d="M14.25.18l.9.2.73.26.59.3.45.32.34.34.25.34.16.33.1.3.04.26.02.2-.01.13V8.5l-.05.63-.13.55-.21.46-.26.38-.3.31-.33.25-.35.19-.35.14-.33.1-.3.07-.26.04-.21.02H8.77l-.69.05-.59.14-.5.22-.41.27-.33.32-.27.35-.2.36-.15.37-.1.35-.07.32-.04.27-.02.21v3.06H3.17l-.21-.03-.28-.07-.32-.12-.35-.18-.36-.26-.36-.36-.35-.46-.32-.59-.28-.73-.21-.88-.14-1.05-.05-1.23.06-1.22.16-1.04.24-.87.32-.71.36-.57.4-.44.42-.33.42-.24.4-.16.36-.1.32-.05.24-.01h.16l.06.01h8.16v-.83H6.18l-.01-2.75-.02-.37.05-.34.11-.31.17-.28.25-.26.31-.23.38-.2.44-.18.51-.15.58-.12.64-.1.71-.06.77-.04.84-.02 1.27.05zm-6.3 1.98l-.23.33-.08.41.08.41.23.34.33.22.41.09.41-.09.33-.22.23-.34.08-.41-.08-.41-.23-.33-.33-.22-.41-.09-.41.09zm13.09 3.95l.28.06.32.12.35.18.36.27.36.35.35.47.32.59.28.73.21.88.14 1.04.05 1.23-.06 1.23-.16 1.04-.24.86-.32.71-.36.57-.4.45-.42.33-.42.24-.4.16-.36.09-.32.05-.24.02-.16-.01h-8.22v.82h5.84l.01 2.76.02.36-.05.34-.11.31-.17.29-.25.25-.31.24-.38.2-.44.17-.51.15-.58.13-.64.09-.71.07-.77.04-.84.01-1.27-.04-1.07-.14-.9-.2-.73-.25-.59-.3-.45-.33-.34-.34-.25-.34-.16-.33-.1-.3-.04-.25-.02-.2.01-.13v-5.34l.05-.64.13-.54.21-.46.26-.38.3-.32.33-.24.35-.2.35-.14.33-.1.3-.06.26-.04.21-.02.13-.01h5.84l.69-.05.59-.14.5-.21.41-.28.33-.32.27-.35.2-.36.15-.36.1-.35.07-.32.04-.28.02-.21V6.07h2.09l.14.01zm-6.47 14.25l-.23.33-.08.41.08.41.23.33.33.23.41.08.41-.08.33-.23.23-.33.08-.41-.08-.41-.23-.33-.33-.23-.41-.08-.41.08z" fill="#3776AB"/>
                </svg>
                <svg v-else viewBox="0 0 24 24" width="32" height="32">
                  <path d="M10.024 7.5H3.5a1 1 0 0 0-1 1v7a1 1 0 0 0 1 1h6.524a1.5 1.5 0 0 1 1.366.882l.11.244.11-.244A1.5 1.5 0 0 1 12.976 16.5H20.5a1 1 0 0 0 1-1v-7a1 1 0 0 0-1-1h-7.524a1.5 1.5 0 0 1-1.366-.882L11.5 6.374l-.11.244A1.5 1.5 0 0 1 10.024 7.5Z" stroke="currentColor" stroke-width="1.5" fill="none"/>
                </svg>
              </div>
              <div class="template-info">
                <h4>{{ template.name }}</h4>
                <p>{{ template.description }}</p>
              </div>
              <div v-if="selectedTemplate === template.id" class="template-check">
                <svg viewBox="0 0 16 16" width="16" height="16">
                  <path d="M13.78 4.22a.75.75 0 0 1 0 1.06l-7.25 7.25a.75.75 0 0 1-1.06 0L2.22 9.28a.75.75 0 0 1 1.06-1.06L6 10.94l6.72-6.72a.75.75 0 0 1 1.06 0Z" fill="currentColor"/>
                </svg>
              </div>
            </div>
          </div>
        </div>
        <div class="modal-footer">
          <button class="btn btn-secondary" @click="showTemplateModal = false">取消</button>
          <button class="btn btn-primary" @click="applyTemplate" :disabled="!selectedTemplate">应用模板</button>
        </div>
      </div>
    </div>

    <!-- 提交消息模态框 -->
    <div v-if="showCommitModal" class="modal-overlay" @click.self="showCommitModal = false">
      <div class="modal-content">
        <div class="modal-header">
          <h3>{{ isNewFile ? '创建文件' : '提交更改' }}</h3>
          <button class="close-btn" @click="showCommitModal = false">&times;</button>
        </div>
        <div class="modal-body">
          <div class="form-group">
            <label>提交消息</label>
            <input v-model="commitMessage" :placeholder="defaultCommitMessage" />
          </div>
          <div class="form-group">
            <label>目标分支</label>
            <input v-model="targetBranch" :placeholder="currentBranch" />
            <p class="help-text">更改将提交到此分支</p>
          </div>
        </div>
        <div class="modal-footer">
          <button class="btn btn-secondary" @click="showCommitModal = false">取消</button>
          <button class="btn btn-primary" @click="confirmCommit" :disabled="saving">
            {{ saving ? '提交中...' : '提交' }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted, nextTick } from 'vue'
import api from '@/api'
import type { Project, BranchInfo } from '@/types'
import { navIcons } from '@/navigation/icons'
import MonacoEditor from '@/components/editor/MonacoEditor.vue'
import PipelineVisualization from './components/PipelineVisualization.vue'
import YAML from 'yaml'

const props = defineProps<{ project?: Project }>()
const icons = navIcons

// 状态
const loading = ref(false)
const saving = ref(false)
const hasConfig = ref(false)
const configFiles = ref<string[]>([])
const currentBranch = ref('')
const currentFile = ref('')
const editorContent = ref('')
const originalContent = ref('')
const mergedConfig = ref('')
const branches = ref<BranchInfo[]>([])
const filteredBranches = ref<BranchInfo[]>([])
const branchSearch = ref('')
const editorRef = ref<InstanceType<typeof MonacoEditor>>()
const isNewFile = ref(false)

// UI 状态
const showBranchDropdown = ref(false)
const showFileDropdown = ref(false)
const showCreateFileModal = ref(false)
const showTemplateModal = ref(false)
const showCommitModal = ref(false)
const activeTab = ref('edit')
const newFileName = ref('')
const commitMessage = ref('')
const targetBranch = ref('')
const selectedTemplate = ref('')

// 验证状态
const validationStatus = ref<'valid' | 'invalid' | null>(null)
const validationMessage = ref('')
const parseError = ref('')
const parsedConfig = ref<any>(null)

// 标签页配置
const tabs = [
  { id: 'edit', label: '编辑' },
  { id: 'visualize', label: '可视化' },
  { id: 'merged', label: '完整配置' }
]

// CI 模板
const ciTemplates = [
  {
    id: 'rust',
    name: 'Rust',
    description: '构建、测试和发布 Rust 项目',
    icon: 'rust',
    content: `# Rust CI/CD 配置
stages:
  - build
  - test

variables:
  CARGO_HOME: \${CI_PROJECT_DIR}/.cargo
  RUST_BACKTRACE: 1

build:
  stage: build
  script:
    - cargo build --release
  artifacts:
    paths:
      - target/release/
    expire_in: 1 day
  cache:
    key: cargo-\${CI_COMMIT_REF_SLUG}
    paths:
      - .cargo/
      - target/

test:
  stage: test
  script:
    - cargo test --all
  needs:
    - build
`
  },
  {
    id: 'node',
    name: 'Node.js',
    description: '构建、测试和部署 Node.js 应用',
    icon: 'node',
    content: `# Node.js CI/CD 配置
stages:
  - install
  - build
  - test

variables:
  NODE_ENV: production

install:
  stage: install
  script:
    - npm ci
  cache:
    key: npm-\${CI_COMMIT_REF_SLUG}
    paths:
      - node_modules/

build:
  stage: build
  script:
    - npm run build
  artifacts:
    paths:
      - dist/
  needs:
    - install

test:
  stage: test
  script:
    - npm run test
  needs:
    - install
`
  },
  {
    id: 'python',
    name: 'Python',
    description: '测试和部署 Python 应用',
    icon: 'python',
    content: `# Python CI/CD 配置
stages:
  - test
  - build

variables:
  PIP_CACHE_DIR: "\${CI_PROJECT_DIR}/.pip-cache"

test:
  stage: test
  script:
    - pip install -r requirements.txt
    - pytest
  cache:
    key: pip-\${CI_COMMIT_REF_SLUG}
    paths:
      - .pip-cache/

build:
  stage: build
  script:
    - pip install build
    - python -m build
  artifacts:
    paths:
      - dist/
  needs:
    - test
`
  },
  {
    id: 'docker',
    name: 'Docker',
    description: '构建和推送 Docker 镜像',
    icon: 'docker',
    content: `# Docker CI/CD 配置
stages:
  - build
  - push

variables:
  IMAGE_NAME: \${CI_REGISTRY_IMAGE}
  IMAGE_TAG: \${CI_COMMIT_REF_SLUG}

build:
  stage: build
  script:
    - docker build -t \${IMAGE_NAME}:\${IMAGE_TAG} .
  tags:
    - docker

push:
  stage: push
  script:
    - docker push \${IMAGE_NAME}:\${IMAGE_TAG}
  needs:
    - build
  only:
    - main
    - tags
`
  },
  {
    id: 'basic',
    name: '基础模板',
    description: '简单的构建和测试配置',
    icon: 'default',
    content: `# 基础 CI/CD 配置
stages:
  - build
  - test
  - deploy

build:
  stage: build
  script:
    - echo "Building..."
    # 添加构建命令

test:
  stage: test
  script:
    - echo "Testing..."
    # 添加测试命令
  needs:
    - build

deploy:
  stage: deploy
  script:
    - echo "Deploying..."
    # 添加部署命令
  when: manual
  only:
    - main
`
  }
]

// 计算属性
const hasChanges = computed(() => editorContent.value !== originalContent.value)
const editorHeight = computed(() => 'calc(100vh - 200px)')

const isValidFileName = computed(() => {
  const name = newFileName.value.trim()
  return name.length > 0 && (name.endsWith('.yml') || name.endsWith('.yaml'))
})

const defaultCommitMessage = computed(() => {
  if (isNewFile.value) {
    return `创建 ${currentFile.value || newFileName.value}`
  }
  return `更新 ${currentFile.value}`
})

// 方法
async function loadBranches() {
  if (!props.project?.owner_name || !props.project?.name) return
  try {
    const result = await api.branches.list({
      namespace: props.project.owner_name,
      project: props.project.name
    })
    branches.value = result
    filteredBranches.value = result
    
    // 设置默认分支
    const defaultBranch = result.find(b => b.is_default)
    if (defaultBranch) {
      currentBranch.value = defaultBranch.name
    } else if (result.length > 0) {
      currentBranch.value = result[0].name
    }
    
    // 分支加载完成后再加载 CI 配置
    await loadEditorInfo()
  } catch (e) {
    console.error('Failed to load branches:', e)
  }
}

function filterBranches() {
  const search = branchSearch.value.toLowerCase()
  filteredBranches.value = branches.value.filter(b => 
    b.name.toLowerCase().includes(search)
  )
}

async function loadEditorInfo() {
  if (!props.project?.owner_name || !props.project?.name) return
  loading.value = true
  try {
    // 使用当前分支，如果为空则 API 会使用仓库默认分支
    const files = await api.repository.browseTree(
      { namespace: props.project.owner_name, project: props.project.name },
      '.gitfox/ci',
      currentBranch.value || undefined
    )
    hasConfig.value = files && files.length > 0
    configFiles.value = (files || [])
      .filter((f: any) => f.entry_type === 'File' && (f.name.endsWith('.yml') || f.name.endsWith('.yaml')))
      .map((f: any) => f.name)

    if (configFiles.value.length > 0 && !currentFile.value) {
      await selectFile(configFiles.value[0])
    }
    
    // 加载所有配置用于合并视图
    await loadMergedConfig()
  } catch (e) {
    hasConfig.value = false
    configFiles.value = []
  } finally {
    loading.value = false
  }
}

async function loadMergedConfig() {
  if (!props.project?.owner_name || !props.project?.name || !hasConfig.value) {
    mergedConfig.value = ''
    return
  }
  
  try {
    const configs: any[] = []
    for (const file of configFiles.value) {
      try {
        const content = await api.repository.getFile(
          { namespace: props.project.owner_name, project: props.project.name },
          `.gitfox/ci/${file}`,
          currentBranch.value || undefined
        )
        if (content.content) {
          configs.push({ file, content: YAML.parse(content.content) })
        }
      } catch (e) {
        console.warn(`Failed to load ${file}:`, e)
      }
    }
    
    // 简单合并所有配置
    const merged: any = { stages: [], variables: {} }
    for (const { content } of configs) {
      if (content.stages) {
        merged.stages = [...new Set([...merged.stages, ...content.stages])]
      }
      if (content.variables) {
        merged.variables = { ...merged.variables, ...content.variables }
      }
      // 合并 jobs
      for (const [key, value] of Object.entries(content)) {
        if (!['stages', 'variables', 'before_script', 'after_script'].includes(key)) {
          merged[key] = value
        }
      }
    }
    
    mergedConfig.value = YAML.stringify(merged)
  } catch (e) {
    console.error('Failed to load merged config:', e)
    mergedConfig.value = '# 无法加载合并配置'
  }
}

async function selectBranch(branch: string) {
  currentBranch.value = branch
  showBranchDropdown.value = false
  currentFile.value = ''
  await loadEditorInfo()
}

async function selectFile(file: string) {
  if (!props.project?.owner_name || !props.project?.name) return
  showFileDropdown.value = false
  currentFile.value = file
  isNewFile.value = false
  
  try {
    const content = await api.repository.getFile(
      { namespace: props.project.owner_name, project: props.project.name },
      `.gitfox/ci/${file}`,
      currentBranch.value || undefined
    )
    editorContent.value = content.content || ''
    originalContent.value = content.content || ''
    validateYaml()
  } catch (e) {
    console.error('Failed to load file:', e)
    editorContent.value = '# 无法加载文件'
    originalContent.value = ''
  }
}

function createFirstConfig() {
  newFileName.value = 'ci.yml'
  showCreateFileModal.value = true
}

function createNewFile() {
  if (!isValidFileName.value) return
  
  const fileName = newFileName.value.trim()
  showCreateFileModal.value = false
  currentFile.value = fileName
  isNewFile.value = true
  hasConfig.value = true
  editorContent.value = `# ${fileName}\nstages:\n  - build\n  - test\n`
  originalContent.value = ''
  
  nextTick(() => {
    editorRef.value?.focus()
  })
}

function applyTemplate() {
  const template = ciTemplates.find(t => t.id === selectedTemplate.value)
  if (!template) return
  
  showTemplateModal.value = false
  
  if (!currentFile.value) {
    // 创建新文件
    currentFile.value = `${template.id}.yml`
    isNewFile.value = true
    hasConfig.value = true
  }
  
  editorContent.value = template.content
  validateYaml()
  
  nextTick(() => {
    editorRef.value?.focus()
  })
}

function onEditorChange(value: string) {
  editorContent.value = value
  validateYaml()
}

function validateYaml() {
  try {
    const parsed = YAML.parse(editorContent.value)
    parsedConfig.value = parsed
    parseError.value = ''
    
    // 基本验证
    if (!parsed || typeof parsed !== 'object') {
      validationStatus.value = 'invalid'
      validationMessage.value = 'YAML 必须是一个对象'
      return
    }
    
    validationStatus.value = 'valid'
    validationMessage.value = '配置有效'
  } catch (e: any) {
    parsedConfig.value = null
    parseError.value = e.message
    validationStatus.value = 'invalid'
    validationMessage.value = `YAML 语法错误: ${e.message}`
  }
}

function saveFile() {
  if (!hasChanges.value) return
  targetBranch.value = currentBranch.value
  commitMessage.value = ''
  showCommitModal.value = true
}

async function confirmCommit() {
  if (!props.project?.owner_name || !props.project?.name) return
  if (!currentFile.value) return
  
  saving.value = true
  const message = commitMessage.value || defaultCommitMessage.value
  const branch = targetBranch.value || currentBranch.value
  const filePath = `.gitfox/ci/${currentFile.value}`
  
  try {
    if (isNewFile.value) {
      await api.repository.createFile(
        { namespace: props.project.owner_name, project: props.project.name },
        filePath,
        { branch, content: editorContent.value, commit_message: message }
      )
    } else {
      await api.repository.updateFile(
        { namespace: props.project.owner_name, project: props.project.name },
        filePath,
        { branch, content: editorContent.value, commit_message: message }
      )
    }
    
    originalContent.value = editorContent.value
    isNewFile.value = false
    showCommitModal.value = false
    
    // 刷新文件列表
    await loadEditorInfo()
  } catch (e: any) {
    console.error('Failed to save file:', e)
    alert(`保存失败: ${e.response?.data?.message || e.message}`)
  } finally {
    saving.value = false
  }
}

// 点击外部关闭下拉菜单
function handleClickOutside(event: MouseEvent) {
  const target = event.target as HTMLElement
  if (!target.closest('.branch-selector')) {
    showBranchDropdown.value = false
  }
  if (!target.closest('.file-selector')) {
    showFileDropdown.value = false
  }
}

// 生命周期
watch([() => props.project?.owner_name, () => props.project?.name], () => {
  loadBranches()  // loadBranches 内部会调用 loadEditorInfo
}, { immediate: true })

onMounted(() => {
  document.addEventListener('click', handleClickOutside)
})

onUnmounted(() => {
  document.removeEventListener('click', handleClickOutside)
})
</script>

<style lang="scss" scoped>
.ci-editor-page {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: $bg-primary;
}

// 工具栏
.editor-toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 16px;
  border-bottom: 1px solid $border-color;
  background: $bg-secondary;

  .toolbar-left, .toolbar-right {
    display: flex;
    align-items: center;
    gap: 12px;
  }
}

// 下拉选择器共用样式
.branch-selector, .file-selector {
  position: relative;
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
  transition: all $transition-fast;

  &:hover {
    background: $bg-tertiary;
    border-color: $border-color-dark;
  }

  .dropdown-menu {
    position: absolute;
    top: 100%;
    left: 0;
    margin-top: 4px;
    min-width: 240px;
    background: $bg-primary;
    border: 1px solid $border-color;
    border-radius: 8px;
    box-shadow: $shadow-lg;
    z-index: $z-dropdown;
    overflow: hidden;
  }

  .dropdown-search {
    padding: 8px;
    border-bottom: 1px solid $border-color;

    input {
      width: 100%;
      padding: 6px 10px;
      border: 1px solid $border-color;
      border-radius: 4px;
      font-size: 13px;
      outline: none;

      &:focus {
        border-color: $primary-color;
        box-shadow: $shadow-focus;
      }
    }
  }

  .dropdown-items {
    max-height: 300px;
    overflow-y: auto;
  }

  .dropdown-item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    font-size: 13px;
    color: $text-primary;
    cursor: pointer;

    &:hover {
      background: $bg-secondary;
    }

    &.active {
      background: rgba($primary-color, 0.1);
      color: $primary-color;
    }

    &.create-new {
      color: $primary-color;
      border-top: 1px solid $border-color;
    }

    .badge {
      margin-left: auto;
      padding: 2px 6px;
      font-size: 11px;
      background: $bg-tertiary;
      border-radius: 4px;
      color: $text-secondary;
    }
  }

  .dropdown-divider {
    height: 1px;
    background: $border-color;
    margin: 4px 0;
  }
}

// 验证状态
.validation-status {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 10px;
  border-radius: 4px;
  font-size: 12px;

  &.valid {
    color: $color-success;
    background: $color-success-light;
  }

  &.invalid {
    color: $color-danger;
    background: $color-danger-light;
  }
}

// 标签页
.editor-tabs {
  display: flex;
  padding: 0 16px;
  border-bottom: 1px solid $border-color;
  background: $bg-secondary;

  .tab-btn {
    padding: 12px 16px;
    font-size: 13px;
    font-weight: 500;
    color: $text-secondary;
    background: transparent;
    border: none;
    border-bottom: 2px solid transparent;
    cursor: pointer;
    transition: all $transition-fast;

    &:hover {
      color: $text-primary;
    }

    &.active {
      color: $primary-color;
      border-bottom-color: $primary-color;
    }
  }
}

// 内容区域
.editor-content {
  flex: 1;
  overflow: hidden;
}

.tab-content {
  height: 100%;
}

.edit-tab {
  .editor-wrapper {
    height: 100%;
    padding: 0;
  }
}

.visualize-tab {
  padding: 24px;
  overflow: auto;
}

.merged-tab {
  display: flex;
  flex-direction: column;
  height: 100%;

  .merged-config-header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 16px;
    background: $bg-tertiary;
    border-bottom: 1px solid $border-color;
    font-size: 13px;
    color: $text-secondary;
  }

  .merged-editor-wrapper {
    flex: 1;
  }
}

// 加载状态
.loading-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  padding: 80px;
  color: $text-secondary;
  font-size: 14px;

  .loading-spinner {
    width: 32px;
    height: 32px;
    border: 3px solid $border-color;
    border-top-color: $primary-color;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

// 空状态
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 80px 24px;
  text-align: center;

  .empty-icon {
    margin-bottom: 24px;
  }

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
      font-family: $font-mono;
      font-size: 13px;
    }
  }

  .empty-actions {
    display: flex;
    gap: 12px;
  }
}

// 模态框
.modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: $z-modal;
}

.modal-content {
  width: 100%;
  max-width: 480px;
  background: $bg-primary;
  border-radius: 12px;
  box-shadow: $shadow-xl;
  overflow: hidden;

  &.modal-large {
    max-width: 720px;
  }
}

.modal-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 16px 20px;
  border-bottom: 1px solid $border-color;

  h3 {
    font-size: 16px;
    font-weight: 600;
    margin: 0;
  }

  .close-btn {
    width: 28px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: none;
    border-radius: 4px;
    font-size: 20px;
    color: $text-secondary;
    cursor: pointer;

    &:hover {
      background: $bg-secondary;
    }
  }
}

.modal-body {
  padding: 20px;
}

.modal-footer {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
  padding: 16px 20px;
  border-top: 1px solid $border-color;
  background: $bg-secondary;
}

// 表单
.form-group {
  margin-bottom: 16px;

  &:last-child {
    margin-bottom: 0;
  }

  label {
    display: block;
    margin-bottom: 6px;
    font-size: 13px;
    font-weight: 500;
    color: $text-primary;
  }

  input {
    width: 100%;
    padding: 8px 12px;
    border: 1px solid $border-color;
    border-radius: 6px;
    font-size: 14px;
    outline: none;
    transition: all $transition-fast;

    &:focus {
      border-color: $primary-color;
      box-shadow: $shadow-focus;
    }
  }

  .help-text {
    margin-top: 6px;
    font-size: 12px;
    color: $text-secondary;
  }
}

.input-with-prefix {
  display: flex;
  align-items: stretch;

  .input-prefix {
    display: flex;
    align-items: center;
    padding: 0 10px;
    background: $bg-tertiary;
    border: 1px solid $border-color;
    border-right: none;
    border-radius: 6px 0 0 6px;
    font-size: 13px;
    font-family: $font-mono;
    color: $text-secondary;
  }

  input {
    border-radius: 0 6px 6px 0;
  }
}

// 模板网格
.template-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
  gap: 12px;
}

.template-card {
  position: relative;
  padding: 16px;
  border: 2px solid $border-color;
  border-radius: 8px;
  cursor: pointer;
  transition: all $transition-fast;

  &:hover {
    border-color: $border-color-dark;
    background: $bg-secondary;
  }

  &.selected {
    border-color: $primary-color;
    background: rgba($primary-color, 0.05);
  }

  .template-icon {
    width: 40px;
    height: 40px;
    display: flex;
    align-items: center;
    justify-content: center;
    margin-bottom: 12px;
    background: $bg-tertiary;
    border-radius: 8px;
  }

  .template-info {
    h4 {
      font-size: 14px;
      font-weight: 600;
      margin: 0 0 4px;
      color: $text-primary;
    }

    p {
      font-size: 12px;
      color: $text-secondary;
      margin: 0;
      line-height: 1.4;
    }
  }

  .template-check {
    position: absolute;
    top: 12px;
    right: 12px;
    width: 24px;
    height: 24px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: $primary-color;
    border-radius: 50%;
    color: white;
  }
}

// 按钮
.btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 8px 14px;
  font-size: 13px;
  font-weight: 500;
  border-radius: 6px;
  border: 1px solid transparent;
  cursor: pointer;
  transition: all $transition-fast;

  &:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }
}

.btn-primary {
  background: $primary-color;
  color: white;

  &:hover:not(:disabled) {
    background: darken($primary-color, 8%);
  }
}

.btn-secondary {
  background: $bg-primary;
  border-color: $border-color;
  color: $text-primary;

  &:hover:not(:disabled) {
    background: $bg-secondary;
    border-color: $border-color-dark;
  }
}

// 旋转动画
.spinner {
  animation: spin 1s linear infinite;
}
</style>
