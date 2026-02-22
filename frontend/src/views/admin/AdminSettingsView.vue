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

      <!-- CI/CD Settings Section -->
      <div class="settings-section">
        <div class="section-header" @click="toggleSection('cicd')">
          <div class="section-title">
            <h2>CI/CD 持续集成</h2>
            <p>作业超时、Runner 注册、流水线限制等设置</p>
          </div>
          <svg class="chevron" :class="{ expanded: expandedSections.cicd }" width="16" height="16" viewBox="0 0 16 16">
            <path d="M6 4l4 4-4 4" stroke="currentColor" stroke-width="1.5" fill="none" stroke-linecap="round"/>
          </svg>
        </div>
        <div v-show="expandedSections.cicd" class="section-body">
          <div class="form-group">
            <label for="ci_default_job_timeout">默认作业超时（秒）</label>
            <input id="ci_default_job_timeout" v-model.number="form.ci_default_job_timeout" type="number" min="60" max="86400" />
            <p class="form-hint">作业未指定超时时的默认值（3600 秒 = 1 小时）</p>
          </div>
          <div class="form-group">
            <label for="ci_max_job_timeout">最大作业超时（秒）</label>
            <input id="ci_max_job_timeout" v-model.number="form.ci_max_job_timeout" type="number" min="60" max="2592000" />
            <p class="form-hint">单个作业允许的最长运行时间（86400 秒 = 24 小时）</p>
          </div>
          <div class="form-group">
            <label for="ci_concurrent_jobs_limit">并发作业数限制</label>
            <input id="ci_concurrent_jobs_limit" v-model.number="form.ci_concurrent_jobs_limit" type="number" min="1" max="1000" />
            <p class="form-hint">实例级别的最大并发作业数</p>
          </div>
          <div class="form-group">
            <label for="ci_max_pipeline_size">流水线最大作业数</label>
            <input id="ci_max_pipeline_size" v-model.number="form.ci_max_pipeline_size" type="number" min="1" max="100" />
            <p class="form-hint">单个流水线中允许定义的最大作业数量</p>
          </div>
          <div class="form-group">
            <label for="ci_pipeline_retention_days">流水线保留天数</label>
            <input id="ci_pipeline_retention_days" v-model.number="form.ci_pipeline_retention_days" type="number" min="1" max="365" />
            <p class="form-hint">流水线记录在自动清理前保留的天数</p>
          </div>
          <div class="form-group">
            <label for="ci_artifacts_retention_days">构建产物保留天数</label>
            <input id="ci_artifacts_retention_days" v-model.number="form.ci_artifacts_retention_days" type="number" min="1" max="90" />
            <p class="form-hint">作业产物在自动清理前保留的天数</p>
          </div>
          <div class="form-group">
            <label for="ci_max_log_size_mb">最大日志大小（MB）</label>
            <input id="ci_max_log_size_mb" v-model.number="form.ci_max_log_size_mb" type="number" min="1" max="1024" />
            <p class="form-hint">单个作业日志文件的最大大小</p>
          </div>
          <div class="form-group checkbox-group">
            <label class="checkbox-label">
              <input type="checkbox" v-model="form.ci_runner_registration_enabled" />
              <span>启用 Runner 注册</span>
            </label>
            <p class="form-hint">关闭后将禁止新的 Runner 注册到系统</p>
          </div>
          <div class="form-group checkbox-group">
            <label class="checkbox-label">
              <input type="checkbox" v-model="form.ci_log_streaming_enabled" />
              <span>启用日志实时流</span>
            </label>
            <p class="form-hint">允许在作业运行时实时查看日志输出</p>
          </div>
          
          <div class="form-group">
            <label for="regular_runner_quota_minutes">非 PRO 用户 Runner 限额（分钟/月）</label>
            <input id="regular_runner_quota_minutes" v-model.number="form.regular_runner_quota_minutes" type="number" min="0" max="1000000" />
            <p class="form-hint">0 表示不限制。非 PRO 用户每月可使用的 Runner 分钟数</p>
          </div>
          
          <div class="form-group">
            <label for="pro_runner_quota_minutes">PRO 用户 Runner 限额（分钟/月）</label>
            <input id="pro_runner_quota_minutes" v-model.number="form.pro_runner_quota_minutes" type="number" min="0" max="1000000" />
            <p class="form-hint">0 表示不限制。PRO 用户每月可使用的 Runner 分钟数</p>
          </div>
          
          <div class="section-actions">
            <button class="btn btn-primary" @click="saveSection('cicd')" :disabled="saving">
              {{ saving ? '保存中...' : '保存更改' }}
            </button>
          </div>
        </div>
      </div>

      <!-- WebIDE Section -->
      <div class="settings-section">
        <div class="section-header" @click="toggleSection('webide')">
          <div class="section-title">
            <h2>WebIDE</h2>
            <p>在线集成开发环境，提供类似 VS Code 的代码编辑体验</p>
          </div>
          <svg class="chevron" :class="{ expanded: expandedSections.webide }" width="16" height="16" viewBox="0 0 16 16">
            <path d="M6 4l4 4-4 4" stroke="currentColor" stroke-width="1.5" fill="none" stroke-linecap="round"/>
          </svg>
        </div>
        <div v-show="expandedSections.webide" class="section-body">
          <div class="form-group checkbox-group">
            <label class="checkbox-label">
              <input type="checkbox" v-model="form.webide_enabled" />
              <span>启用 WebIDE</span>
            </label>
            <p class="form-hint">允许用户通过浏览器直接编辑代码文件</p>
          </div>
          
          <div class="section-actions">
            <button class="btn btn-primary" @click="saveSection('webide')" :disabled="saving">
              {{ saving ? '保存中...' : '保存更改' }}
            </button>
          </div>
        </div>
      </div>

      <!-- VS Code Extension Marketplace Section -->
      <div class="settings-section">
        <div class="section-header" @click="toggleSection('vscode')">
          <div class="section-title">
            <h2>VS Code 扩展市场</h2>
            <p>为 WebIDE 配置 VS Code 扩展仓库</p>
          </div>
          <svg class="chevron" :class="{ expanded: expandedSections.vscode }" width="16" height="16" viewBox="0 0 16 16">
            <path d="M6 4l4 4-4 4" stroke="currentColor" stroke-width="1.5" fill="none" stroke-linecap="round"/>
          </svg>
        </div>
        <div v-show="expandedSections.vscode" class="section-body">
          <div class="form-group checkbox-group">
            <label class="checkbox-label">
              <input type="checkbox" v-model="form.vscode_extensions_enabled" />
              <span>启用扩展市场</span>
            </label>
            <p class="form-hint">为所有用户启用 VS Code 扩展市场</p>
          </div>
          
          <div class="subsection" v-if="form.vscode_extensions_enabled">
            <h4>扩展仓库设置</h4>
            <div class="form-group checkbox-group">
              <label class="checkbox-label">
                <input type="checkbox" v-model="form.vscode_extensions_use_open_vsx" />
                <span>使用 Open VSX 扩展仓库</span>
              </label>
              <p class="form-hint">
                <a href="https://open-vsx.org/" target="_blank" rel="noopener">了解更多关于 Open VSX 注册表</a>
              </p>
            </div>
            
            <div class="form-group">
              <label for="vscode_extensions_service_url">服务 URL</label>
              <input 
                id="vscode_extensions_service_url" 
                v-model="form.vscode_extensions_service_url" 
                type="url" 
                placeholder="https://open-vsx.org/vscode/gallery" 
              />
            </div>
            
            <div class="form-group">
              <label for="vscode_extensions_item_url">项目 URL</label>
              <input 
                id="vscode_extensions_item_url" 
                v-model="form.vscode_extensions_item_url" 
                type="url" 
                placeholder="https://open-vsx.org/vscode/item" 
              />
            </div>
            
            <div class="form-group">
              <label for="vscode_extensions_resource_url">资源 URL 模板</label>
              <input 
                id="vscode_extensions_resource_url" 
                v-model="form.vscode_extensions_resource_url" 
                type="text" 
                placeholder="https://open-vsx.org/vscode/unpkg/{publisher}/{name}/{version}/{path}" 
              />
              <p class="form-hint">支持的变量: {publisher}, {name}, {version}, {path}</p>
            </div>
          </div>
          
          <div class="section-actions">
            <button class="btn btn-primary" @click="saveSection('vscode')" :disabled="saving">
              {{ saving ? '保存中...' : '保存更改' }}
            </button>
          </div>
        </div>
      </div>

      <!-- Gitpod Integration Section -->
      <div class="settings-section">
        <div class="section-header" @click="toggleSection('gitpod')">
          <div class="section-title">
            <h2>Gitpod</h2>
            <p>Gitpod 集成后，用户可以从 GitFox 浏览器选项卡启动开发环境</p>
          </div>
          <svg class="chevron" :class="{ expanded: expandedSections.gitpod }" width="16" height="16" viewBox="0 0 16 16">
            <path d="M6 4l4 4-4 4" stroke="currentColor" stroke-width="1.5" fill="none" stroke-linecap="round"/>
          </svg>
        </div>
        <div v-show="expandedSections.gitpod" class="section-body">
          <div class="form-group checkbox-group">
            <label class="checkbox-label">
              <input type="checkbox" v-model="form.gitpod_enabled" />
              <span>启用 Gitpod 集成</span>
            </label>
          </div>
          
          <div class="form-group" v-if="form.gitpod_enabled">
            <label for="gitpod_url">Gitpod 网址</label>
            <input 
              id="gitpod_url" 
              v-model="form.gitpod_url" 
              type="url" 
              placeholder="https://gitpod.io/" 
            />
            <p class="form-hint">
              配置为读取 GitFox 项目的 Gitpod 实例的 URL，例如 https://gitpod.example.com。
              要使用集成，每个用户还必须在其 GitFox 账户上启用 Gitpod。
              <a href="https://www.gitpod.io/docs" target="_blank" rel="noopener">如何启用它？</a>
            </p>
          </div>
          
          <div class="section-actions">
            <button class="btn btn-primary" @click="saveSection('gitpod')" :disabled="saving">
              {{ saving ? '保存中...' : '保存更改' }}
            </button>
          </div>
        </div>
      </div>

      <!-- SMTP Settings Section -->
      <div class="settings-section">
        <div class="section-header" @click="toggleSection('smtp')">
          <div class="section-title">
            <h2>邮件服务 (SMTP)</h2>
            <p>查看 SMTP 配置状态，配置需在 .env 文件中设置</p>
          </div>
          <svg class="chevron" :class="{ expanded: expandedSections.smtp }" width="16" height="16" viewBox="0 0 16 16">
            <path d="M6 4l4 4-4 4" stroke="currentColor" stroke-width="1.5" fill="none" stroke-linecap="round"/>
          </svg>
        </div>
        <div v-show="expandedSections.smtp" class="section-body">
          <div class="smtp-status" :class="smtpConfig.configured ? 'configured' : 'not-configured'">
            <div class="status-indicator">
              <span class="status-dot"></span>
              <span class="status-text">{{ smtpConfig.configured ? 'SMTP 已配置' : 'SMTP 未配置' }}</span>
            </div>
            <p class="status-hint" v-if="!smtpConfig.configured">
              请在 .env 文件中设置 SMTP_HOST, SMTP_PORT, SMTP_USERNAME 等环境变量
            </p>
          </div>
          
          <div class="smtp-config-display" v-if="smtpConfig.configured">
            <h3>当前配置（来自环境变量）</h3>
            <div class="config-grid">
              <div class="config-item">
                <label>服务器</label>
                <span>{{ smtpConfig.host }}:{{ smtpConfig.port }}</span>
              </div>
              <div class="config-item">
                <label>发件人</label>
                <span>{{ smtpConfig.from_name }} &lt;{{ smtpConfig.from_email }}&gt;</span>
              </div>
              <div class="config-item">
                <label>加密方式</label>
                <span>{{ smtpConfig.use_ssl ? 'SSL/TLS' : (smtpConfig.use_tls ? 'STARTTLS' : '无加密') }}</span>
              </div>
            </div>
            
            <!-- Test SMTP -->
            <div class="smtp-test">
              <h3>测试邮件发送</h3>
              <div class="form-row">
                <div class="form-group">
                  <label for="test_email">测试邮箱</label>
                  <input id="test_email" v-model="testEmail" type="email" placeholder="输入接收测试邮件的邮箱" />
                </div>
                <div class="test-actions">
                  <button class="btn btn-secondary" @click="testSmtpConnection" :disabled="smtpTesting">
                    {{ smtpTesting ? '测试中...' : '测试连接' }}
                  </button>
                  <button class="btn btn-secondary" @click="sendTestEmail" :disabled="smtpTesting || !testEmail">
                    {{ smtpTesting ? '发送中...' : '发送测试邮件' }}
                  </button>
                </div>
              </div>
              <div v-if="smtpTestResult" class="smtp-test-result" :class="smtpTestResult.success ? 'success' : 'error'">
                {{ smtpTestResult.message }}
              </div>
            </div>
          </div>
          
          <div class="smtp-env-hint">
            <h3>环境变量配置说明</h3>
            <pre class="env-example">
SMTP_ENABLED=true
SMTP_HOST=smtp.example.com
SMTP_PORT=587
SMTP_USERNAME=your-email@example.com
SMTP_PASSWORD=your-password
SMTP_FROM_EMAIL=noreply@example.com
SMTP_FROM_NAME=GitFox
SMTP_USE_TLS=true
SMTP_USE_SSL=false</pre>
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
  cicd: false,
  webide: false,
  vscode: false,
  gitpod: false,
  smtp: false,
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
  // CI/CD settings
  ci_default_job_timeout: 3600,
  ci_max_job_timeout: 86400,
  ci_concurrent_jobs_limit: 100,
  ci_max_pipeline_size: 50,
  ci_pipeline_retention_days: 90,
  ci_artifacts_retention_days: 30,
  ci_max_log_size_mb: 100,
  ci_runner_registration_enabled: true,
  ci_log_streaming_enabled: true,
  regular_runner_quota_minutes: 2000,
  pro_runner_quota_minutes: 0,
  // WebIDE settings
  webide_enabled: false,
  // VS Code Extension Marketplace settings
  vscode_extensions_enabled: false,
  vscode_extensions_use_open_vsx: true,
  vscode_extensions_service_url: 'https://open-vsx.org/vscode/gallery',
  vscode_extensions_item_url: 'https://open-vsx.org/vscode/item',
  vscode_extensions_resource_url: 'https://open-vsx.org/vscode/unpkg/{publisher}/{name}/{version}/{path}',
  // Gitpod settings
  gitpod_enabled: false,
  gitpod_url: 'https://gitpod.io/',
})

// SMTP config from environment (read-only)
const smtpConfig = reactive({
  configured: false,
  enabled: false,
  host: '',
  port: 587,
  from_email: '',
  from_name: '',
  use_tls: true,
  use_ssl: false,
})

const testEmail = ref('')
const smtpTesting = ref(false)
const smtpTestResult = ref<{ success: boolean; message: string } | null>(null)

function toggleSection(section: keyof typeof expandedSections) {
  expandedSections[section] = !expandedSections[section]
}

const sectionKeys: Record<string, string[]> = {
  general: ['site_name', 'site_description'],
  signup: ['signup_enabled', 'require_email_confirmation', 'user_default_role'],
  project: ['default_project_visibility', 'max_attachment_size_mb'],
  appearance: ['gravatar_enabled', 'home_page_url', 'after_sign_in_path'],
  terms: ['terms_of_service'],
  cicd: [
    'ci_default_job_timeout',
    'ci_max_job_timeout',
    'ci_concurrent_jobs_limit',
    'ci_max_pipeline_size',
    'ci_pipeline_retention_days',
    'ci_artifacts_retention_days',
    'ci_max_log_size_mb',
    'ci_runner_registration_enabled',
    'ci_log_streaming_enabled',
    'regular_runner_quota_minutes',
    'pro_runner_quota_minutes',
  ],
  webide: [
    'webide_enabled',
  ],
  vscode: [
    'vscode_extensions_enabled',
    'vscode_extensions_use_open_vsx',
    'vscode_extensions_service_url',
    'vscode_extensions_item_url',
    'vscode_extensions_resource_url',
  ],
  gitpod: [
    'gitpod_enabled',
    'gitpod_url',
  ],
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

// SMTP functions - load config from environment via API
async function loadSmtpConfig() {
  try {
    const config = await api.admin.getSmtpConfig()
    smtpConfig.configured = config.configured
    smtpConfig.enabled = config.enabled
    smtpConfig.host = config.host
    smtpConfig.port = config.port
    smtpConfig.from_email = config.from_email
    smtpConfig.from_name = config.from_name
    smtpConfig.use_tls = config.use_tls
    smtpConfig.use_ssl = config.use_ssl
  } catch (err) {
    console.error('Failed to load SMTP config:', err)
  }
}

async function testSmtpConnection() {
  smtpTesting.value = true
  smtpTestResult.value = null
  try {
    await api.admin.testSmtpConnection({
      enabled: smtpConfig.enabled,
      host: smtpConfig.host,
      port: smtpConfig.port,
      username: '', // credentials come from env
      password: '',
      from_email: smtpConfig.from_email,
      from_name: smtpConfig.from_name,
      use_tls: smtpConfig.use_tls,
      use_ssl: smtpConfig.use_ssl,
    })
    smtpTestResult.value = { success: true, message: '连接成功！SMTP 服务器配置正确。' }
  } catch (err: any) {
    const msg = err.response?.data?.message || err.response?.data?.error || err.message || '连接失败'
    smtpTestResult.value = { success: false, message: `连接失败: ${msg}` }
  } finally {
    smtpTesting.value = false
  }
}

async function sendTestEmail() {
  if (!testEmail.value) {
    alert('请输入测试邮箱地址')
    return
  }
  smtpTesting.value = true
  smtpTestResult.value = null
  try {
    await api.admin.sendTestEmail({
      enabled: true,
      host: smtpConfig.host,
      port: smtpConfig.port,
      username: '',
      password: '',
      from_email: smtpConfig.from_email,
      from_name: smtpConfig.from_name,
      use_tls: smtpConfig.use_tls,
      use_ssl: smtpConfig.use_ssl,
      test_email: testEmail.value,
    })
    smtpTestResult.value = { success: true, message: `测试邮件已发送到 ${testEmail.value}` }
  } catch (err: any) {
    const msg = err.response?.data?.message || err.response?.data?.error || err.message || '发送失败'
    smtpTestResult.value = { success: false, message: `发送失败: ${msg}` }
  } finally {
    smtpTesting.value = false
  }
}

onMounted(() => {
  loadConfigs()
  loadSmtpConfig()
})
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

// SMTP section styles
.smtp-status {
  padding: $spacing-4;
  border-radius: $border-radius;
  margin-bottom: $spacing-4;
  
  &.configured {
    background: rgba($color-success, 0.1);
    border: 1px solid rgba($color-success, 0.2);
  }
  
  &.not-configured {
    background: rgba($color-warning, 0.1);
    border: 1px solid rgba($color-warning, 0.2);
  }
  
  .status-indicator {
    display: flex;
    align-items: center;
    gap: $spacing-2;
    font-weight: $font-weight-medium;
    
    .status-dot {
      width: 8px;
      height: 8px;
      border-radius: 50%;
    }
  }
  
  &.configured .status-dot {
    background: $color-success;
  }
  
  &.not-configured .status-dot {
    background: $color-warning;
  }
  
  .status-hint {
    margin-top: $spacing-2;
    font-size: $font-size-sm;
    color: $text-secondary;
  }
}

.smtp-config-display {
  h3 {
    font-size: $font-size-base;
    font-weight: $font-weight-medium;
    margin-bottom: $spacing-3;
    color: $text-primary;
  }
}

.config-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
  gap: $spacing-3;
  margin-bottom: $spacing-4;
  
  .config-item {
    label {
      display: block;
      font-size: $font-size-xs;
      color: $text-secondary;
      margin-bottom: $spacing-1;
    }
    
    span {
      font-size: $font-size-sm;
      color: $text-primary;
    }
  }
}

.smtp-env-hint {
  margin-top: $spacing-5;
  padding-top: $spacing-5;
  border-top: 1px solid $border-color-light;
  
  h3 {
    font-size: $font-size-base;
    font-weight: $font-weight-medium;
    margin-bottom: $spacing-3;
    color: $text-primary;
  }
  
  .env-example {
    background: $bg-tertiary;
    border: 1px solid $border-color;
    border-radius: $border-radius;
    padding: $spacing-3;
    font-family: $font-mono;
    font-size: $font-size-sm;
    line-height: 1.6;
    overflow-x: auto;
    color: $text-primary;
  }
}

.smtp-config {
  &.disabled {
    opacity: 0.5;
    pointer-events: none;
  }
}

.form-row {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: $spacing-4;
  
  @media (max-width: 600px) {
    grid-template-columns: 1fr;
  }
}

.form-group-small {
  max-width: 120px;
}

.radio-group {
  display: flex;
  flex-direction: column;
  gap: $spacing-2;
  margin-top: $spacing-2;
}

.radio-label {
  display: inline-flex;
  align-items: center;
  gap: $spacing-2;
  cursor: pointer;
  font-size: $font-size-sm;
  color: $text-primary;

  input[type="radio"] {
    width: 16px;
    height: 16px;
    accent-color: $brand-primary;
    cursor: pointer;
  }

  &:has(input:disabled) {
    opacity: 0.5;
    cursor: not-allowed;
  }
}

.smtp-test {
  margin-top: $spacing-5;
  padding-top: $spacing-5;
  border-top: 1px solid $border-color-light;

  .form-row {
    align-items: flex-end;
  }

  .test-actions {
    display: flex;
    gap: $spacing-2;
    padding-bottom: $spacing-5;
  }
}

.smtp-test-result {
  margin-top: $spacing-3;
  padding: $spacing-3 $spacing-4;
  border-radius: $border-radius;
  font-size: $font-size-sm;
  
  &.success {
    background: rgba($color-success, 0.1);
    color: $color-success;
    border: 1px solid rgba($color-success, 0.2);
  }
  
  &.error {
    background: rgba($color-danger, 0.1);
    color: $color-danger;
    border: 1px solid rgba($color-danger, 0.2);
  }
}

.btn-secondary {
  background: $bg-secondary;
  color: $text-primary;
  border: 1px solid $border-color;
  
  &:hover:not(:disabled) {
    background: $bg-tertiary;
    border-color: $border-color-dark;
  }
}
</style>
