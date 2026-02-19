<template>
  <div class="project-runners-view">
    <div class="page-header">
      <div class="header-content">
        <h1>CI/CD Runners</h1>
        <p class="description">管理此项目可用的 Runners</p>
      </div>
      <button class="btn btn-primary" @click="showCreateModal = true">
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
          <path d="M8 3V13M3 8H13" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
        </svg>
        新建项目 Runner
      </button>
    </div>

    <!-- Stats Overview -->
    <div class="stats-grid">
      <div class="stat-card">
        <div class="stat-icon total">
          <svg width="24" height="24" viewBox="0 0 24 24" fill="none">
            <rect x="4" y="4" width="16" height="16" rx="2" stroke="currentColor" stroke-width="2"/>
            <path d="M9 12L11 14L15 10" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
        </div>
        <div class="stat-content">
          <div class="stat-label">可用 Runners</div>
          <div class="stat-value">{{ totalAvailableRunners }}</div>
        </div>
      </div>
      
      <div class="stat-card">
        <div class="stat-icon project">
          <svg width="24" height="24" viewBox="0 0 24 24" fill="none">
            <path d="M3 3H21V21H3V3Z" stroke="currentColor" stroke-width="2"/>
            <path d="M9 9H15M9 15H15" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
          </svg>
        </div>
        <div class="stat-content">
          <div class="stat-label">项目专用</div>
          <div class="stat-value">{{ projectRunners.length }}</div>
        </div>
      </div>

      <div class="stat-card">
        <div class="stat-icon online">
          <svg width="24" height="24" viewBox="0 0 24 24" fill="none">
            <circle cx="12" cy="12" r="8" stroke="currentColor" stroke-width="2"/>
            <path d="M12 8V12L14.5 14.5" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
          </svg>
        </div>
        <div class="stat-content">
          <div class="stat-label">在线</div>
          <div class="stat-value">{{ onlineRunners }}</div>
        </div>
      </div>

      <div class="stat-card">
        <div class="stat-icon running">
          <svg width="24" height="24" viewBox="0 0 24 24" fill="none">
            <circle cx="12" cy="12" r="10" stroke="currentColor" stroke-width="2"/>
            <path d="M10 8L16 12L10 16V8Z" fill="currentColor"/>
          </svg>
        </div>
        <div class="stat-content">
          <div class="stat-label">运行中</div>
          <div class="stat-value">{{ runningRunners }}</div>
        </div>
      </div>
    </div>

    <!-- Info Banner -->
    <div class="info-banner">
      <svg class="banner-icon" width="20" height="20" viewBox="0 0 20 20" fill="none">
        <circle cx="10" cy="10" r="9" stroke="currentColor" stroke-width="2"/>
        <path d="M10 6V10M10 14H10.01" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
      </svg>
      <div class="banner-content">
        <strong>Runner 优先级</strong>
        <p>任务执行时会按以下顺序选择 Runner：项目专用 → 群组 → 私有 → 系统共享</p>
      </div>
    </div>

    <!-- Loading State -->
    <div v-if="loading" class="loading-state">
      <div class="spinner"></div>
      <p>加载 Runners...</p>
    </div>

    <!-- Error State -->
    <div v-else-if="error" class="error-banner">
      <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
        <circle cx="10" cy="10" r="9" stroke="currentColor" stroke-width="2"/>
        <path d="M10 6V10M10 14H10.01" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
      </svg>
      <span>{{ error }}</span>
    </div>

    <template v-else>
      <!-- Project-specific Runners -->
      <div class="runners-section">
        <div class="section-header">
          <div class="section-title">
            <h2>项目专用 Runners</h2>
            <span class="count-badge">{{ projectRunners.length }}</span>
          </div>
          <div v-if="projectRunners.length > 0" class="filters">
            <input
              v-model="projectSearchQuery"
              type="text"
              class="search-input"
              placeholder="搜索..."
            />
            <select v-model="projectStatusFilter" class="filter-select">
              <option value="">所有状态</option>
              <option value="idle">空闲</option>
              <option value="running">运行中</option>
              <option value="offline">离线</option>
            </select>
          </div>
        </div>

        <div v-if="filteredProjectRunners.length === 0 && projectRunners.length === 0" class="empty-state">
          <svg width="64" height="64" viewBox="0 0 64 64" fill="none">
            <rect x="8" y="8" width="48" height="48" rx="4" stroke="currentColor" stroke-width="2"/>
            <path d="M24 28L32 36L40 28" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
          <h3>暂无项目专用 Runner</h3>
          <p>创建专用 Runner 以确保此项目的任务在指定机器上执行</p>
          <button class="btn btn-primary" @click="showCreateModal = true">新建 Runner</button>
        </div>

        <div v-else-if="filteredProjectRunners.length > 0" class="runners-table-container">
          <table class="runners-table">
            <thead>
              <tr>
                <th>名称</th>
                <th>状态</th>
                <th>标签</th>
                <th>版本</th>
                <th>最后联系</th>
                <th>配置</th>
                <th>操作</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="runner in filteredProjectRunners" :key="runner.id" :class="{ inactive: !runner.is_active }">
                <td>
                  <div class="runner-name">
                    <span class="status-dot" :class="runner.status"></span>
                    <div>
                      <div class="name">{{ runner.name }}</div>
                      <div v-if="runner.description" class="description">{{ runner.description }}</div>
                    </div>
                  </div>
                </td>
                <td>
                  <span class="status-badge" :class="runner.status">
                    {{ getStatusText(runner.status) }}
                  </span>
                </td>
                <td>
                  <div class="tags">
                    <span v-for="tag in runner.tags" :key="tag" class="tag">{{ tag }}</span>
                    <span v-if="runner.run_untagged" class="tag untagged">untagged</span>
                    <span v-if="runner.tags.length === 0 && !runner.run_untagged" class="no-tags">无标签</span>
                  </div>
                </td>
                <td>
                  <div v-if="runner.version" class="version-info">
                    <div class="version">{{ runner.version }}</div>
                    <div v-if="runner.platform" class="platform">{{ runner.platform }} / {{ runner.architecture }}</div>
                  </div>
                  <span v-else class="text-muted">-</span>
                </td>
                <td>
                  <span v-if="runner.last_contact_at" class="last-contact">
                    {{ formatTime(runner.last_contact_at) }}
                  </span>
                  <span v-else class="text-muted">从未</span>
                </td>
                <td>
                  <div class="config-badges">
                    <span v-if="runner.locked" class="badge locked" title="已锁定">🔒</span>
                    <span v-if="runner.maximum_timeout" class="badge timeout" :title="`超时: ${runner.maximum_timeout}s`">
                      ⏱️ {{ runner.maximum_timeout }}s
                    </span>
                  </div>
                </td>
                <td>
                  <div class="actions">
                    <button class="btn-icon" @click="editRunner(runner)" title="编辑">
                      <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
                        <path d="M11.333 2.00004C11.5081 1.82494 11.716 1.68605 11.9447 1.59129C12.1735 1.49653 12.4187 1.44775 12.6663 1.44775C12.914 1.44775 13.1592 1.49653 13.3879 1.59129C13.6167 1.68605 13.8246 1.82494 13.9997 2.00004C14.1748 2.17513 14.3137 2.383 14.4084 2.61178C14.5032 2.84055 14.552 3.08575 14.552 3.33337C14.552 3.58099 14.5032 3.82619 14.4084 4.05497C14.3137 4.28374 14.1748 4.49161 13.9997 4.66671L5.33301 13.3334L1.33301 14.6667L2.66634 10.6667L11.333 2.00004Z" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
                      </svg>
                    </button>
                    <button class="btn-icon danger" @click="confirmDelete(runner)" title="删除">
                      <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
                        <path d="M2 4H3.33333H14" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
                        <path d="M5.33301 4.00004V2.66671C5.33301 2.31309 5.47348 1.97395 5.72353 1.7239C5.97358 1.47385 6.31272 1.33337 6.66634 1.33337H9.33301C9.68663 1.33337 10.0258 1.47385 10.2758 1.7239C10.5259 1.97395 10.6663 2.31309 10.6663 2.66671V4.00004M12.6663 4.00004V13.3334C12.6663 13.687 12.5259 14.0261 12.2758 14.2762C12.0258 14.5262 11.6866 14.6667 11.333 14.6667H4.66634C4.31272 14.6667 3.97358 14.5262 3.72353 14.2762C3.47348 14.0261 3.33301 13.687 3.33301 13.3334V4.00004H12.6663Z" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
                      </svg>
                    </button>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
        </div>

        <div v-else-if="projectSearchQuery || projectStatusFilter" class="no-results">
          <p>未找到匹配的 Runners</p>
        </div>
      </div>

      <!-- Available Runners from other scopes -->
      <div v-if="hasAvailableRunners" class="runners-section available-runners">
        <div class="section-header">
          <h2>可用 Runners（继承）</h2>
        </div>

        <!-- Group Runners -->
        <div v-if="namespaceRunners.length > 0" class="runner-group">
          <h3>
            <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
              <path d="M5.33333 14.6667V7.33333H10.6667V14.6667" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
              <path d="M1.33333 6L8 1.33333L14.6667 6V13.3333C14.6667 13.687 14.5262 14.0261 14.2761 14.2761C14.0261 14.5262 13.687 14.6667 13.3333 14.6667H2.66667C2.31304 14.6667 1.97391 14.5262 1.72386 14.2761C1.47381 14.0261 1.33333 13.687 1.33333 13.3333V6Z" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
            群组 Runners
            <span class="scope-badge group">{{ namespaceRunners.length }}</span>
          </h3>
          <div class="runners-grid">
            <div v-for="runner in namespaceRunners" :key="runner.id" class="runner-card" :class="{ inactive: !runner.is_active }">
              <div class="card-header">
                <span class="status-dot" :class="runner.status"></span>
                <div class="runner-info">
                  <div class="runner-name">{{ runner.name }}</div>
                  <span class="status-badge small" :class="runner.status">{{ getStatusText(runner.status) }}</span>
                </div>
              </div>
              <div v-if="runner.description" class="card-description">{{ runner.description }}</div>
              <div class="card-meta">
                <div class="tags">
                  <span v-for="tag in runner.tags.slice(0, 3)" :key="tag" class="tag small">{{ tag }}</span>
                  <span v-if="runner.tags.length > 3" class="tag small">+{{ runner.tags.length - 3 }}</span>
                  <span v-if="runner.run_untagged" class="tag small untagged">untagged</span>
                </div>
                <div v-if="runner.version" class="version">{{ runner.version }}</div>
              </div>
            </div>
          </div>
        </div>

        <!-- User Private Runners (only shown if user is namespace owner) -->
        <div v-if="userRunners.length > 0" class="runner-group">
          <h3>
            <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
              <path d="M13.3333 14V12.6667C13.3333 11.9594 13.0524 11.2811 12.5523 10.781C12.0522 10.281 11.3739 10 10.6667 10H5.33333C4.62609 10 3.94781 10.281 3.44772 10.781C2.94762 11.2811 2.66667 11.9594 2.66667 12.6667V14" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
              <circle cx="8" cy="4.66667" r="2.66667" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
            私有 Runners
            <span class="scope-badge user">{{ userRunners.length }}</span>
          </h3>
          <div class="runners-grid">
            <div v-for="runner in userRunners" :key="runner.id" class="runner-card" :class="{ inactive: !runner.is_active }">
              <div class="card-header">
                <span class="status-dot" :class="runner.status"></span>
                <div class="runner-info">
                  <div class="runner-name">{{ runner.name }}</div>
                  <span class="status-badge small" :class="runner.status">{{ getStatusText(runner.status) }}</span>
                </div>
              </div>
              <div v-if="runner.description" class="card-description">{{ runner.description }}</div>
              <div class="card-meta">
                <div class="tags">
                  <span v-for="tag in runner.tags.slice(0, 3)" :key="tag" class="tag small">{{ tag }}</span>
                  <span v-if="runner.tags.length > 3" class="tag small">+{{ runner.tags.length - 3 }}</span>
                  <span v-if="runner.run_untagged" class="tag small untagged">untagged</span>
                </div>
                <div v-if="runner.version" class="version">{{ runner.version }}</div>
              </div>
            </div>
          </div>
        </div>

        <!-- System Shared Runners -->
        <div v-if="sharedRunners.length > 0" class="runner-group">
          <h3>
            <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
              <circle cx="8" cy="8" r="6" stroke="currentColor" stroke-width="1.5"/>
              <path d="M8 2V8L11 11" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
            </svg>
            系统共享 Runners
            <span class="scope-badge system">{{ sharedRunners.length }}</span>
          </h3>
          <div class="runners-grid">
            <div v-for="runner in sharedRunners" :key="runner.id" class="runner-card" :class="{ inactive: !runner.is_active }">
              <div class="card-header">
                <span class="status-dot" :class="runner.status"></span>
                <div class="runner-info">
                  <div class="runner-name">{{ runner.name }}</div>
                  <span class="status-badge small" :class="runner.status">{{ getStatusText(runner.status) }}</span>
                </div>
              </div>
              <div v-if="runner.description" class="card-description">{{ runner.description }}</div>
              <div class="card-meta">
                <div class="tags">
                  <span v-for="tag in runner.tags.slice(0, 3)" :key="tag" class="tag small">{{ tag }}</span>
                  <span v-if="runner.tags.length > 3" class="tag small">+{{ runner.tags.length - 3 }}</span>
                  <span v-if="runner.run_untagged" class="tag small untagged">untagged</span>
                </div>
                <div v-if="runner.version" class="version">{{ runner.version }}</div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </template>

    <!-- Create Runner Modal -->
    <div v-if="showCreateModal" class="modal-overlay" @click.self="showCreateModal = false">
      <div class="modal">
        <div class="modal-header">
          <h2>新建项目 Runner</h2>
          <button class="btn-close" @click="showCreateModal = false">×</button>
        </div>
        <div class="modal-body">
          <div v-if="createError" class="error-banner">
            <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
              <circle cx="10" cy="10" r="9" stroke="currentColor" stroke-width="2"/>
              <path d="M10 6V10M10 14H10.01" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
            </svg>
            <span>{{ createError }}</span>
          </div>

          <div class="form-group">
            <label for="runner-name">名称 *</label>
            <input
              id="runner-name"
              v-model="newRunner.name"
              type="text"
              class="form-input"
              placeholder="例如: project-runner-1"
              required
            />
          </div>

          <div class="form-group">
            <label for="runner-description">描述</label>
            <textarea
              id="runner-description"
              v-model="newRunner.description"
              class="form-input"
              rows="3"
              placeholder="Runner 的用途说明..."
            ></textarea>
          </div>

          <div class="form-group">
            <label>标签</label>
            <div class="tags-input">
              <span v-for="(tag, index) in newRunner.tags" :key="index" class="tag">
                {{ tag }}
                <button type="button" @click="removeTag(index)" class="tag-remove">×</button>
              </span>
              <input
                v-model="tagInput"
                type="text"
                placeholder="输入标签后按回车"
                @keydown.enter.prevent="addTag"
              />
            </div>
            <p class="form-help">用于匹配特定的 CI/CD 任务</p>
          </div>

          <div class="form-group">
            <label class="checkbox-label">
              <input v-model="newRunner.run_untagged" type="checkbox" />
              <span>运行无标签的任务</span>
            </label>
          </div>

          <div class="form-group">
            <label class="checkbox-label">
              <input v-model="newRunner.locked" type="checkbox" />
              <span>锁定到当前项目</span>
            </label>
            <p class="form-help">锁定后此 Runner 不能被其他项目使用</p>
          </div>

          <div class="form-group">
            <label for="runner-timeout">最大超时时间（秒）</label>
            <input
              id="runner-timeout"
              v-model.number="newRunner.maximum_timeout"
              type="number"
              class="form-input"
              min="0"
              placeholder="0 表示无限制"
            />
          </div>
        </div>
        <div class="modal-footer">
          <button class="btn btn-secondary" @click="showCreateModal = false">取消</button>
          <button class="btn btn-primary" @click="createRunner" :disabled="creating || !newRunner.name">
            {{ creating ? '创建中...' : '创建 Runner' }}
          </button>
        </div>
      </div>
    </div>

    <!-- Token Display Modal -->
    <div v-if="showTokenModal" class="modal-overlay" @click.self="showTokenModal = false">
      <div class="modal modal-token">
        <div class="modal-header">
          <h2>Runner 已创建</h2>
          <button class="btn-close" @click="showTokenModal = false">×</button>
        </div>
        <div class="modal-body">
          <div class="success-banner">
            <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
              <circle cx="10" cy="10" r="9" stroke="currentColor" stroke-width="2"/>
              <path d="M6 10L9 13L14 7" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
            <span>Runner 创建成功！</span>
          </div>

          <div class="warning-box">
            <strong>⚠️ 重要提示</strong>
            <p>请立即保存此令牌，它只会显示一次。</p>
          </div>

          <div class="token-display">
            <code>{{ createdToken }}</code>
            <button class="btn-copy" @click="copyToken" :class="{ copied: tokenCopied }">
              <svg v-if="!tokenCopied" width="16" height="16" viewBox="0 0 16 16" fill="none">
                <rect x="5" y="5" width="9" height="9" rx="1" stroke="currentColor" stroke-width="1.5"/>
                <path d="M3 11V3C3 2.44772 3.44772 2 4 2H11" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
              </svg>
              <svg v-else width="16" height="16" viewBox="0 0 16 16" fill="none">
                <path d="M3 8L6 11L13 4" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
              </svg>
              {{ tokenCopied ? '已复制' : '复制' }}
            </button>
          </div>

          <div class="setup-instructions">
            <h3>设置说明</h3>
            <ol>
              <li>在目标机器上安装 GitFox Runner</li>
              <li>使用以下命令注册 Runner：
                <pre><code>gitfox-runner register --url {{ serverUrl }} --token {{ createdToken }}</code></pre>
              </li>
              <li>启动 Runner 服务</li>
            </ol>
          </div>
        </div>
        <div class="modal-footer">
          <button class="btn btn-primary" @click="showTokenModal = false">我已保存令牌</button>
        </div>
      </div>
    </div>

    <!-- Edit Runner Modal -->
    <div v-if="showEditModal" class="modal-overlay" @click.self="showEditModal = false">
      <div class="modal">
        <div class="modal-header">
          <h2>编辑 Runner</h2>
          <button class="btn-close" @click="showEditModal = false">×</button>
        </div>
        <div class="modal-body">
          <div v-if="editError" class="error-banner">
            <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
              <circle cx="10" cy="10" r="9" stroke="currentColor" stroke-width="2"/>
              <path d="M10 6V10M10 14H10.01" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
            </svg>
            <span>{{ editError }}</span>
          </div>

          <div class="form-group">
            <label for="edit-runner-name">名称 *</label>
            <input
              id="edit-runner-name"
              v-model="editingRunner.name"
              type="text"
              class="form-input"
              required
            />
          </div>

          <div class="form-group">
            <label for="edit-runner-description">描述</label>
            <textarea
              id="edit-runner-description"
              v-model="editingRunner.description"
              class="form-input"
              rows="3"
            ></textarea>
          </div>

          <div class="form-group">
            <label>标签</label>
            <div class="tags-input">
              <span v-for="(tag, index) in editingRunner.tags" :key="index" class="tag">
                {{ tag }}
                <button type="button" @click="removeEditTag(index)" class="tag-remove">×</button>
              </span>
              <input
                v-model="editTagInput"
                type="text"
                placeholder="输入标签后按回车"
                @keydown.enter.prevent="addEditTag"
              />
            </div>
          </div>

          <div class="form-group">
            <label class="checkbox-label">
              <input v-model="editingRunner.run_untagged" type="checkbox" />
              <span>运行无标签的任务</span>
            </label>
          </div>

          <div class="form-group">
            <label class="checkbox-label">
              <input v-model="editingRunner.locked" type="checkbox" />
              <span>锁定到当前项目</span>
            </label>
          </div>

          <div class="form-group">
            <label for="edit-runner-timeout">最大超时时间（秒）</label>
            <input
              id="edit-runner-timeout"
              v-model.number="editingRunner.maximum_timeout"
              type="number"
              class="form-input"
              min="0"
            />
          </div>

          <div class="form-group">
            <label class="checkbox-label">
              <input v-model="editingRunner.is_active" type="checkbox" />
              <span>激活状态</span>
            </label>
          </div>
        </div>
        <div class="modal-footer">
          <button class="btn btn-secondary" @click="showEditModal = false">取消</button>
          <button class="btn btn-primary" @click="updateRunner" :disabled="updating || !editingRunner.name">
            {{ updating ? '保存中...' : '保存更改' }}
          </button>
        </div>
      </div>
    </div>

    <!-- Delete Confirmation Modal -->
    <div v-if="showDeleteModal" class="modal-overlay" @click.self="showDeleteModal = false">
      <div class="modal modal-small">
        <div class="modal-header">
          <h2>删除 Runner</h2>
          <button class="btn-close" @click="showDeleteModal = false">×</button>
        </div>
        <div class="modal-body">
          <div v-if="deleteError" class="error-banner">
            <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
              <circle cx="10" cy="10" r="9" stroke="currentColor" stroke-width="2"/>
              <path d="M10 6V10M10 14H10.01" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
            </svg>
            <span>{{ deleteError }}</span>
          </div>

          <div class="warning-box">
            <strong>⚠️ 确认删除</strong>
            <p>确定要删除 Runner <strong>{{ runnerToDelete?.name }}</strong> 吗？</p>
            <p>此操作无法撤销。</p>
          </div>
        </div>
        <div class="modal-footer">
          <button class="btn btn-secondary" @click="showDeleteModal = false">取消</button>
          <button class="btn btn-danger" @click="deleteRunner" :disabled="deleting">
            {{ deleting ? '删除中...' : '确认删除' }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import api from '@/api'
import type { Runner, CreateRunnerRequest, UpdateRunnerRequest, Project } from '@/types'

const route = useRoute()
const props = defineProps<{ project?: Project }>()

const projectPath = computed(() => ({
  namespace: (props.project?.owner_name || route.params.owner || route.params.namespace) as string,
  project: (props.project?.name || route.params.repo || route.params.project) as string
}))

// State
const projectRunners = ref<Runner[]>([])
const namespaceRunners = ref<Runner[]>([])
const userRunners = ref<Runner[]>([])
const sharedRunners = ref<Runner[]>([])
const loading = ref(false)
const error = ref('')

// Filters
const projectSearchQuery = ref('')
const projectStatusFilter = ref('')

// Modals
const showCreateModal = ref(false)
const showTokenModal = ref(false)
const showEditModal = ref(false)
const showDeleteModal = ref(false)

// Create
const newRunner = ref<CreateRunnerRequest>({
  name: '',
  description: '',
  tags: [],
  run_untagged: false,
  locked: false,
  maximum_timeout: 0
})
const tagInput = ref('')
const creating = ref(false)
const createError = ref('')
const createdToken = ref('')
const tokenCopied = ref(false)
const serverUrl = computed(() => window.location.origin)

// Edit
const editingRunner = ref<UpdateRunnerRequest & { id?: string }>({
  name: '',
  description: '',
  tags: [],
  run_untagged: false,
  locked: false,
  maximum_timeout: 0,
  is_active: true
})
const editTagInput = ref('')
const updating = ref(false)
const editError = ref('')

// Delete
const runnerToDelete = ref<Runner | null>(null)
const deleting = ref(false)
const deleteError = ref('')

// Computed
const totalAvailableRunners = computed(() => 
  projectRunners.value.length + 
  namespaceRunners.value.length + 
  userRunners.value.length + 
  sharedRunners.value.length
)

const allRunners = computed(() => [
  ...projectRunners.value,
  ...namespaceRunners.value,
  ...userRunners.value,
  ...sharedRunners.value
])

const onlineRunners = computed(() => 
  allRunners.value.filter(r => r.status !== 'offline').length
)

const runningRunners = computed(() => 
  allRunners.value.filter(r => r.status === 'running').length
)

const hasAvailableRunners = computed(() => 
  namespaceRunners.value.length > 0 || 
  userRunners.value.length > 0 || 
  sharedRunners.value.length > 0
)

const filteredProjectRunners = computed(() => {
  let filtered = projectRunners.value

  if (projectSearchQuery.value) {
    const query = projectSearchQuery.value.toLowerCase()
    filtered = filtered.filter(r => 
      r.name.toLowerCase().includes(query) ||
      r.description?.toLowerCase().includes(query) ||
      r.tags.some(t => t.toLowerCase().includes(query))
    )
  }

  if (projectStatusFilter.value) {
    filtered = filtered.filter(r => r.status === projectStatusFilter.value)
  }

  return filtered
})

// Methods
async function loadRunners() {
  loading.value = true
  error.value = ''
  try {
    // Load all available runners for this project
    const allRunnersData = await api.runners.projectList(projectPath.value)
    
    // Separate runners by scope
    projectRunners.value = allRunnersData.filter(r => r.scope === 'project')
    namespaceRunners.value = allRunnersData.filter(r => r.scope === 'namespace')
    userRunners.value = allRunnersData.filter(r => r.scope === 'user')
    sharedRunners.value = allRunnersData.filter(r => r.scope === 'system')
  } catch (e: any) {
    error.value = e.response?.data?.error || '加载 Runners 失败'
  } finally {
    loading.value = false
  }
}

function getStatusText(status: string): string {
  const statusMap: Record<string, string> = {
    idle: '空闲',
    running: '运行中',
    offline: '离线'
  }
  return statusMap[status] || status
}

function formatTime(timestamp: string): string {
  const date = new Date(timestamp)
  const now = new Date()
  const diff = now.getTime() - date.getTime()
  const seconds = Math.floor(diff / 1000)
  const minutes = Math.floor(seconds / 60)
  const hours = Math.floor(minutes / 60)
  const days = Math.floor(hours / 24)

  if (days > 0) return `${days} 天前`
  if (hours > 0) return `${hours} 小时前`
  if (minutes > 0) return `${minutes} 分钟前`
  return '刚刚'
}

function addTag() {
  const tag = tagInput.value.trim()
  if (tag && !newRunner.value.tags?.includes(tag)) {
    if (!newRunner.value.tags) {
      newRunner.value.tags = []
    }
    newRunner.value.tags.push(tag)
    tagInput.value = ''
  }
}

function removeTag(index: number) {
  if (newRunner.value.tags) {
    newRunner.value.tags.splice(index, 1)
  }
}

function addEditTag() {
  const tag = editTagInput.value.trim()
  if (tag && !editingRunner.value.tags?.includes(tag)) {
    if (!editingRunner.value.tags) {
      editingRunner.value.tags = []
    }
    editingRunner.value.tags.push(tag)
    editTagInput.value = ''
  }
}

function removeEditTag(index: number) {
  if (editingRunner.value.tags) {
    editingRunner.value.tags.splice(index, 1)
  }
}

async function createRunner() {
  creating.value = true
  createError.value = ''
  try {
    const response = await api.runners.projectCreate(projectPath.value, {
      name: newRunner.value.name,
      description: newRunner.value.description || undefined,
      tags: newRunner.value.tags,
      run_untagged: newRunner.value.run_untagged,
      locked: newRunner.value.locked,
      maximum_timeout: newRunner.value.maximum_timeout || undefined
    })
    
    createdToken.value = response.token
    showCreateModal.value = false
    showTokenModal.value = true
    
    // Reset form
    newRunner.value = {
      name: '',
      description: '',
      tags: [],
      run_untagged: false,
      locked: false,
      maximum_timeout: 0
    }
    
    // Reload runners
    await loadRunners()
  } catch (e: any) {
    createError.value = e.response?.data?.error || '创建 Runner 失败'
  } finally {
    creating.value = false
  }
}

async function copyToken() {
  try {
    await navigator.clipboard.writeText(createdToken.value)
    tokenCopied.value = true
    setTimeout(() => {
      tokenCopied.value = false
    }, 2000)
  } catch (e) {
    console.error('Failed to copy token:', e)
  }
}

function editRunner(runner: Runner) {
  editingRunner.value = {
    id: runner.id,
    name: runner.name,
    description: runner.description || '',
    tags: [...runner.tags],
    run_untagged: runner.run_untagged,
    locked: runner.locked,
    maximum_timeout: runner.maximum_timeout || 0,
    is_active: runner.is_active
  }
  showEditModal.value = true
}

async function updateRunner() {
  if (!editingRunner.value.id) return
  
  updating.value = true
  editError.value = ''
  try {
    const updateData: UpdateRunnerRequest = {
      name: editingRunner.value.name,
      description: editingRunner.value.description || undefined,
      tags: editingRunner.value.tags || [],
      run_untagged: editingRunner.value.run_untagged,
      locked: editingRunner.value.locked,
      maximum_timeout: editingRunner.value.maximum_timeout || undefined,
      is_active: editingRunner.value.is_active
    }
    
    await api.runners.projectUpdate(projectPath.value, editingRunner.value.id, updateData)
    showEditModal.value = false
    await loadRunners()
  } catch (e: any) {
    editError.value = e.response?.data?.error || '更新 Runner 失败'
  } finally {
    updating.value = false
  }
}

function confirmDelete(runner: Runner) {
  runnerToDelete.value = runner
  showDeleteModal.value = true
}

async function deleteRunner() {
  if (!runnerToDelete.value) return
  
  deleting.value = true
  deleteError.value = ''
  try {
    await api.runners.projectDelete(projectPath.value, runnerToDelete.value.id)
    showDeleteModal.value = false
    runnerToDelete.value = null
    await loadRunners()
  } catch (e: any) {
    deleteError.value = e.response?.data?.error || '删除 Runner 失败'
  } finally {
    deleting.value = false
  }
}

onMounted(() => {
  loadRunners()
})
</script>

<style scoped lang="scss">
@import '@/styles/variables.scss';

// 重用 GroupRunnersView 的样式，但添加一些特定样式

.project-runners-view {
  padding: 24px;
  max-width: 1400px;
  margin: 0 auto;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 24px;

  .header-content {
    h1 {
      font-size: 28px;
      font-weight: 600;
      margin: 0 0 8px 0;
      color: $text-primary;
    }

    .description {
      color: $text-secondary;
      margin: 0;
    }
  }
}

.stats-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
  gap: 16px;
  margin-bottom: 24px;
}

.stat-card {
  background: $bg-secondary;
  border: 1px solid $border-color;
  border-radius: 8px;
  padding: 20px;
  display: flex;
  align-items: center;
  gap: 16px;

  .stat-icon {
    width: 48px;
    height: 48px;
    border-radius: 8px;
    display: flex;
    align-items: center;
    justify-content: center;

    &.total {
      background: rgba(59, 130, 246, 0.1);
      color: #3b82f6;
    }

    &.project {
      background: rgba(168, 85, 247, 0.1);
      color: #a855f7;
    }

    &.online {
      background: rgba(16, 185, 129, 0.1);
      color: #10b981;
    }

    &.running {
      background: rgba(251, 146, 60, 0.1);
      color: #fb923c;
    }
  }

  .stat-content {
    flex: 1;

    .stat-label {
      font-size: 13px;
      color: $text-secondary;
      margin-bottom: 4px;
    }

    .stat-value {
      font-size: 28px;
      font-weight: 600;
      color: $text-primary;
    }
  }
}

.info-banner {
  background: rgba(59, 130, 246, 0.1);
  border: 1px solid rgba(59, 130, 246, 0.3);
  border-radius: 8px;
  padding: 16px;
  display: flex;
  gap: 12px;
  margin-bottom: 24px;

  .banner-icon {
    color: #3b82f6;
    flex-shrink: 0;
  }

  .banner-content {
    flex: 1;

    strong {
      display: block;
      margin-bottom: 4px;
      color: #3b82f6;
    }

    p {
      margin: 0;
      color: $text-secondary;
      font-size: 14px;
    }
  }
}

.runners-section {
  margin-bottom: 32px;

  &.available-runners {
    background: rgba(139, 92, 246, 0.03);
    border: 1px solid rgba(139, 92, 246, 0.1);
    border-radius: 12px;
    padding: 24px;
  }

  h2 {
    font-size: 20px;
    font-weight: 600;
    margin: 0 0 16px 0;
    color: $text-primary;
  }

  .section-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 16px;

    .section-title {
      display: flex;
      align-items: center;
      gap: 12px;

      h2 {
        margin: 0;
      }

      .count-badge {
        padding: 4px 12px;
        background: rgba(59, 130, 246, 0.1);
        border: 1px solid rgba(59, 130, 246, 0.3);
        color: #3b82f6;
        border-radius: 12px;
        font-size: 13px;
        font-weight: 600;
      }
    }

    .filters {
      display: flex;
      gap: 12px;

      .search-input {
        width: 240px;
      }

      .filter-select {
        min-width: 120px;
      }
    }
  }
}

.runner-group {
  margin-bottom: 32px;

  &:last-child {
    margin-bottom: 0;
  }

  h3 {
    font-size: 16px;
    font-weight: 600;
    margin: 0 0 16px 0;
    color: $text-primary;
    display: flex;
    align-items: center;
    gap: 8px;

    svg {
      color: $text-secondary;
    }

    .scope-badge {
      padding: 2px 8px;
      border-radius: 10px;
      font-size: 12px;
      font-weight: 600;

      &.system {
        background: rgba(139, 92, 246, 0.1);
        color: #8b5cf6;
      }

      &.group {
        background: rgba(34, 197, 94, 0.1);
        color: #22c55e;
      }

      &.user {
        background: rgba(59, 130, 246, 0.1);
        color: #3b82f6;
      }
    }
  }
}

.runners-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
  gap: 16px;
}

.runner-card {
  background: $bg-secondary;
  border: 1px solid $border-color;
  border-radius: 8px;
  padding: 16px;
  transition: all 0.2s;

  &:hover {
    border-color: darken($border-color, 10%);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.05);
  }

  &.inactive {
    opacity: 0.6;
  }

  .card-header {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 12px;

    .status-dot {
      width: 10px;
      height: 10px;
      border-radius: 50%;
      flex-shrink: 0;

      &.idle {
        background: #10b981;
      }

      &.running {
        background: #fb923c;
        animation: pulse 2s infinite;
      }

      &.offline {
        background: #6b7280;
      }
    }

    .runner-info {
      flex: 1;
      min-width: 0;

      .runner-name {
        font-weight: 600;
        color: $text-primary;
        margin-bottom: 4px;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
      }

      .status-badge {
        display: inline-block;
        padding: 3px 10px;
        border-radius: 10px;
        font-size: 11px;
        font-weight: 500;

        &.small {
          font-size: 11px;
          padding: 2px 8px;
        }

        &.idle {
          background: rgba(16, 185, 129, 0.1);
          color: #10b981;
        }

        &.running {
          background: rgba(251, 146, 60, 0.1);
          color: #fb923c;
        }

        &.offline {
          background: rgba(107, 114, 128, 0.1);
          color: #6b7280;
        }
      }
    }
  }

  .card-description {
    color: $text-secondary;
    font-size: 13px;
    margin-bottom: 12px;
    line-height: 1.5;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .card-meta {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;

    .tags {
      display: flex;
      flex-wrap: wrap;
      gap: 4px;
      flex: 1;
      min-width: 0;

      .tag {
        padding: 3px 8px;
        background: $bg-tertiary;
        border: 1px solid $border-color;
        border-radius: 4px;
        font-size: 11px;
        color: $text-secondary;
        white-space: nowrap;

        &.small {
          font-size: 11px;
          padding: 2px 6px;
        }

        &.untagged {
          background: rgba(251, 146, 60, 0.1);
          border-color: rgba(251, 146, 60, 0.3);
          color: #fb923c;
        }
      }
    }

    .version {
      font-size: 12px;
      color: $text-tertiary;
      white-space: nowrap;
    }
  }
}

.runners-table-container {
  background: $bg-secondary;
  border: 1px solid $border-color;
  border-radius: 8px;
  overflow: hidden;
}

.runners-table {
  width: 100%;
  border-collapse: collapse;

  thead {
    background: $bg-tertiary;
    border-bottom: 1px solid $border-color;

    th {
      text-align: left;
      padding: 12px 16px;
      font-size: 13px;
      font-weight: 600;
      color: $text-secondary;
      text-transform: uppercase;
      letter-spacing: 0.5px;
    }
  }

  tbody {
    tr {
      border-bottom: 1px solid $border-color;
      transition: background-color 0.2s;

      &:hover {
        background: $bg-secondary;
      }

      &.inactive {
        opacity: 0.6;
      }

      &:last-child {
        border-bottom: none;
      }
    }

    td {
      padding: 12px 16px;
      font-size: 14px;
    }
  }
}

.runner-name {
  display: flex;
  align-items: center;
  gap: 12px;

  .status-dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    flex-shrink: 0;

    &.idle {
      background: #10b981;
    }

    &.running {
      background: #fb923c;
      animation: pulse 2s infinite;
    }

    &.offline {
      background: #6b7280;
    }
  }

  .name {
    font-weight: 500;
    color: $text-primary;
  }

  .description {
    font-size: 13px;
    color: $text-secondary;
    margin-top: 2px;
  }
}

.status-badge {
  display: inline-block;
  padding: 4px 12px;
  border-radius: 12px;
  font-size: 12px;
  font-weight: 500;

  &.idle {
    background: rgba(16, 185, 129, 0.1);
    color: #10b981;
  }

  &.running {
    background: rgba(251, 146, 60, 0.1);
    color: #fb923c;
  }

  &.offline {
    background: rgba(107, 114, 128, 0.1);
    color: #6b7280;
  }
}

.tags {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;

  .tag {
    padding: 4px 8px;
    background: $bg-tertiary;
    border: 1px solid $border-color;
    border-radius: 4px;
    font-size: 12px;
    color: $text-secondary;

    &.untagged {
      background: rgba(251, 146, 60, 0.1);
      border-color: rgba(251, 146, 60, 0.3);
      color: #fb923c;
    }
  }

  .no-tags {
    color: $text-tertiary;
    font-size: 13px;
  }
}

.version-info {
  .version {
    font-weight: 500;
    color: $text-primary;
  }

  .platform {
    font-size: 12px;
    color: $text-secondary;
    margin-top: 2px;
  }
}

.last-contact {
  color: $text-secondary;
  font-size: 13px;
}

.config-badges {
  display: flex;
  gap: 8px;

  .badge {
    padding: 4px 8px;
    background: $bg-tertiary;
    border: 1px solid $border-color;
    border-radius: 4px;
    font-size: 12px;

    &.locked {
      background: rgba(239, 68, 68, 0.1);
      border-color: rgba(239, 68, 68, 0.3);
    }

    &.timeout {
      background: rgba(59, 130, 246, 0.1);
      border-color: rgba(59, 130, 246, 0.3);
    }
  }
}

.actions {
  display: flex;
  gap: 8px;

  .btn-icon {
    padding: 6px;
    background: transparent;
    border: 1px solid $border-color;
    border-radius: 4px;
    cursor: pointer;
    color: $text-secondary;
    transition: all 0.2s;

    &:hover {
      background: $bg-secondary;
      color: $text-primary;
    }

    &.danger:hover {
      background: rgba(239, 68, 68, 0.1);
      border-color: #ef4444;
      color: #ef4444;
    }
  }
}

.empty-state,
.no-results {
  text-align: center;
  padding: 64px 24px;
  background: $bg-secondary;
  border: 1px solid $border-color;
  border-radius: 8px;

  svg {
    color: $text-tertiary;
    margin-bottom: 16px;
  }

  h3 {
    font-size: 18px;
    font-weight: 600;
    margin: 0 0 8px 0;
    color: $text-primary;
  }

  p {
    color: $text-secondary;
  }
}

.no-results p {
  margin: 0;
}

.loading-state {
  text-align: center;
  padding: 64px 24px;

  p {
    margin-top: 16px;
    color: $text-secondary;
  }
}

.error-banner,
.success-banner,
.warning-box {
  padding: 12px 16px;
  border-radius: 6px;
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 16px;

  svg {
    flex-shrink: 0;
  }
}

.error-banner {
  background: rgba(239, 68, 68, 0.1);
  border: 1px solid rgba(239, 68, 68, 0.3);
  color: #ef4444;
}

.success-banner {
  background: rgba(16, 185, 129, 0.1);
  border: 1px solid rgba(16, 185, 129, 0.3);
  color: #10b981;
}

.warning-box {
  background: rgba(251, 146, 60, 0.1);
  border: 1px solid rgba(251, 146, 60, 0.3);
  padding: 16px;
  display: block;

  strong {
    display: block;
    margin-bottom: 8px;
    color: #fb923c;
  }

  p {
    margin: 4px 0;
    color: $text-secondary;
    font-size: 14px;
  }
}

// Modal styles (same as GroupRunnersView)
.modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
  padding: 24px;
}

.modal {
  background: $bg-primary;
  border: 1px solid $border-color;
  border-radius: 12px;
  max-width: 600px;
  width: 100%;
  max-height: 90vh;
  display: flex;
  flex-direction: column;
  box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.1);

  &.modal-small {
    max-width: 480px;
  }

  &.modal-token {
    max-width: 700px;
  }
}

.modal-header {
  padding: 24px;
  border-bottom: 1px solid $border-color;
  display: flex;
  justify-content: space-between;
  align-items: center;

  h2 {
    font-size: 20px;
    font-weight: 600;
    margin: 0;
    color: $text-primary;
  }

  .btn-close {
    background: transparent;
    border: none;
    font-size: 28px;
    color: $text-secondary;
    cursor: pointer;
    padding: 0;
    width: 32px;
    height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;

    &:hover {
      background: $bg-secondary;
      color: $text-primary;
    }
  }
}

.modal-body {
  padding: 24px;
  overflow-y: auto;
}

.modal-footer {
  padding: 16px 24px;
  border-top: 1px solid $border-color;
  display: flex;
  justify-content: flex-end;
  gap: 12px;
}

.form-group {
  margin-bottom: 20px;

  label {
    display: block;
    margin-bottom: 8px;
    font-weight: 500;
    color: $text-primary;
    font-size: 14px;
  }

  .form-input {
    width: 100%;
    padding: 10px 12px;
    border: 1px solid $border-color;
    border-radius: 6px;
    font-size: 14px;
    background: $bg-secondary;
    color: $text-primary;
    transition: border-color 0.2s;

    &:focus {
      outline: none;
      border-color: $primary-color;
    }
  }

  textarea.form-input {
    resize: vertical;
    min-height: 80px;
  }

  .form-help {
    margin: 6px 0 0 0;
    font-size: 13px;
    color: $text-secondary;
  }
}

.checkbox-label {
  display: flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
  font-weight: 400;

  input[type="checkbox"] {
    width: 18px;
    height: 18px;
    cursor: pointer;
  }
}

.tags-input {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  padding: 8px;
  border: 1px solid $border-color;
  border-radius: 6px;
  background: $bg-secondary;
  min-height: 44px;

  &:focus-within {
    border-color: $primary-color;
  }

  .tag {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 8px;
    background: $bg-tertiary;
    border: 1px solid $border-color;
    border-radius: 4px;
    font-size: 13px;
    color: $text-primary;

    .tag-remove {
      background: transparent;
      border: none;
      color: $text-secondary;
      cursor: pointer;
      padding: 0;
      font-size: 16px;
      line-height: 1;

      &:hover {
        color: #ef4444;
      }
    }
  }

  input {
    flex: 1;
    min-width: 120px;
    border: none;
    background: transparent;
    outline: none;
    font-size: 14px;
    color: $text-primary;
  }
}

.token-display {
  position: relative;
  background: $bg-tertiary;
  border: 1px solid $border-color;
  border-radius: 6px;
  padding: 16px;
  margin-bottom: 24px;

  code {
    display: block;
    font-family: 'Monaco', 'Menlo', 'Courier New', monospace;
    font-size: 13px;
    word-break: break-all;
    color: $text-primary;
    padding-right: 80px;
  }

  .btn-copy {
    position: absolute;
    top: 12px;
    right: 12px;
    padding: 6px 12px;
    background: $bg-secondary;
    border: 1px solid $border-color;
    border-radius: 4px;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 13px;
    color: $text-primary;
    transition: all 0.2s;

    &:hover {
      background: $bg-secondary;
    }

    &.copied {
      background: rgba(16, 185, 129, 0.1);
      border-color: #10b981;
      color: #10b981;
    }
  }
}

.setup-instructions {
  h3 {
    font-size: 16px;
    font-weight: 600;
    margin: 0 0 12px 0;
    color: $text-primary;
  }

  ol {
    margin: 0;
    padding-left: 24px;

    li {
      margin-bottom: 12px;
      color: $text-secondary;
      line-height: 1.6;

      pre {
        margin-top: 8px;
        padding: 12px;
        background: $bg-tertiary;
        border: 1px solid $border-color;
        border-radius: 4px;
        overflow-x: auto;

        code {
          font-family: 'Monaco', 'Menlo', 'Courier New', monospace;
          font-size: 12px;
          color: $text-primary;
        }
      }
    }
  }
}

.btn {
  padding: 10px 20px;
  border-radius: 6px;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s;
  border: none;
  display: inline-flex;
  align-items: center;
  gap: 8px;

  &:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  &.btn-primary {
    background: $primary-color;
    color: white;

    &:hover:not(:disabled) {
      background: darken($primary-color, 5%);
    }
  }

  &.btn-secondary {
    background: $bg-tertiary;
    border: 1px solid $border-color;
    color: $text-primary;

    &:hover:not(:disabled) {
      background: $bg-secondary;
    }
  }

  &.btn-danger {
    background: #ef4444;
    color: white;

    &:hover:not(:disabled) {
      background: darken(#ef4444, 5%);
    }
  }
}

.spinner {
  width: 40px;
  height: 40px;
  border: 3px solid $border-color;
  border-top-color: $primary-color;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

@keyframes pulse {
  0%, 100% {
    opacity: 1;
  }
  50% {
    opacity: 0.5;
  }
}

.text-muted {
  color: $text-tertiary;
}

.search-input,
.filter-select {
  padding: 8px 12px;
  border: 1px solid $border-color;
  border-radius: 6px;
  font-size: 14px;
  background: $bg-secondary;
  color: $text-primary;

  &:focus {
    outline: none;
    border-color: $primary-color;
  }
}
</style>
