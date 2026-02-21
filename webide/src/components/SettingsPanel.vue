<template>
  <div class="settings-panel">
    <div class="settings-nav">
      <button 
        v-for="tab in tabs" 
        :key="tab.id"
        class="nav-item"
        :class="{ active: activeTab === tab.id }"
        @click="activeTab = tab.id"
      >
        {{ tab.label }}
      </button>
    </div>
    
    <div class="settings-content">
      <!-- Editor Settings -->
      <div v-if="activeTab === 'editor'" class="settings-section">
        <h3>编辑器</h3>
        
        <div class="setting-item">
          <div class="setting-info">
            <label>字体大小</label>
            <span class="setting-description">编辑器的字体大小 (像素)</span>
          </div>
          <input type="number" v-model="settings.editor.fontSize" class="input input-sm" style="width: 80px" />
        </div>
        
        <div class="setting-item">
          <div class="setting-info">
            <label>字体</label>
            <span class="setting-description">编辑器的字体系列</span>
          </div>
          <input type="text" v-model="settings.editor.fontFamily" class="input input-sm" style="width: 200px" />
        </div>
        
        <div class="setting-item">
          <div class="setting-info">
            <label>Tab 大小</label>
            <span class="setting-description">一个 Tab 等于多少空格</span>
          </div>
          <input type="number" v-model="settings.editor.tabSize" class="input input-sm" style="width: 80px" />
        </div>
        
        <div class="setting-item">
          <div class="setting-info">
            <label>自动换行</label>
            <span class="setting-description">控制编辑器是否自动换行</span>
          </div>
          <label class="toggle">
            <input type="checkbox" v-model="settings.editor.wordWrap" />
            <span class="toggle-slider"></span>
          </label>
        </div>
        
        <div class="setting-item">
          <div class="setting-info">
            <label>迷你地图</label>
            <span class="setting-description">是否显示迷你地图</span>
          </div>
          <label class="toggle">
            <input type="checkbox" v-model="settings.editor.minimap" />
            <span class="toggle-slider"></span>
          </label>
        </div>
      </div>
      
      <!-- Appearance Settings -->
      <div v-if="activeTab === 'appearance'" class="settings-section">
        <h3>外观</h3>
        
        <div class="setting-item">
          <div class="setting-info">
            <label>主题</label>
            <span class="setting-description">选择编辑器颜色主题</span>
          </div>
          <select v-model="settings.appearance.theme" class="input input-sm" style="width: 150px">
            <option value="dark">深色</option>
            <option value="light">浅色</option>
          </select>
        </div>
        
        <div class="setting-item">
          <div class="setting-info">
            <label>图标主题</label>
            <span class="setting-description">文件图标主题</span>
          </div>
          <select v-model="settings.appearance.iconTheme" class="input input-sm" style="width: 150px">
            <option value="default">默认</option>
            <option value="material">Material</option>
          </select>
        </div>
      </div>
      
      <!-- Keybindings Settings -->
      <div v-if="activeTab === 'keybindings'" class="settings-section">
        <h3>快捷键</h3>
        
        <div class="keybinding-list">
          <div class="keybinding-item" v-for="kb in keybindings" :key="kb.command">
            <span class="kb-command">{{ kb.label }}</span>
            <span class="kb-key">{{ kb.keybinding }}</span>
          </div>
        </div>
      </div>
      
      <!-- Extensions Settings -->
      <div v-if="activeTab === 'extensions'" class="settings-section">
        <h3>扩展</h3>
        <p class="settings-description">
          管理已安装的扩展和配置
        </p>
        <!-- Extension settings would go here -->
      </div>
    </div>
    
    <div class="settings-footer">
      <button class="btn btn-secondary" @click="resetSettings">重置为默认</button>
      <button class="btn btn-primary" @click="saveSettings">保存</button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive } from 'vue'

const emit = defineEmits<{
  (e: 'close'): void
}>()

const activeTab = ref('editor')

const tabs = [
  { id: 'editor', label: '编辑器' },
  { id: 'appearance', label: '外观' },
  { id: 'keybindings', label: '快捷键' },
  { id: 'extensions', label: '扩展' }
]

const settings = reactive({
  editor: {
    fontSize: 14,
    fontFamily: "'JetBrains Mono', 'Fira Code', monospace",
    tabSize: 2,
    wordWrap: false,
    minimap: true
  },
  appearance: {
    theme: 'dark',
    iconTheme: 'default'
  }
})

const keybindings = [
  { command: 'save', label: '保存', keybinding: 'Ctrl+S' },
  { command: 'saveAll', label: '保存所有', keybinding: 'Ctrl+Shift+S' },
  { command: 'quickOpen', label: '快速打开', keybinding: 'Ctrl+P' },
  { command: 'commandPalette', label: '命令面板', keybinding: 'Ctrl+Shift+P' },
  { command: 'toggleSidebar', label: '切换侧边栏', keybinding: 'Ctrl+B' },
  { command: 'toggleTerminal', label: '切换终端', keybinding: 'Ctrl+`' },
  { command: 'closeTab', label: '关闭标签页', keybinding: 'Ctrl+W' },
  { command: 'find', label: '查找', keybinding: 'Ctrl+F' },
  { command: 'replace', label: '替换', keybinding: 'Ctrl+H' }
]

function resetSettings() {
  settings.editor = {
    fontSize: 14,
    fontFamily: "'JetBrains Mono', 'Fira Code', monospace",
    tabSize: 2,
    wordWrap: false,
    minimap: true
  }
  settings.appearance = {
    theme: 'dark',
    iconTheme: 'default'
  }
}

function saveSettings() {
  localStorage.setItem('gitfox-ide-settings', JSON.stringify(settings))
  emit('close')
}
</script>

<style lang="scss" scoped>
.settings-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.settings-nav {
  display: flex;
  gap: 4px;
  padding: 8px;
  border-bottom: 1px solid var(--ide-border);
  
  .nav-item {
    padding: 8px 16px;
    background: none;
    border: none;
    color: var(--ide-text-muted);
    cursor: pointer;
    border-radius: 4px;
    font-size: 13px;
    
    &:hover {
      background: var(--ide-surface-hover);
      color: var(--ide-text);
    }
    
    &.active {
      background: var(--ide-surface);
      color: var(--ide-text);
    }
  }
}

.settings-content {
  flex: 1;
  overflow-y: auto;
  padding: 16px;
}

.settings-section {
  h3 {
    font-size: 16px;
    font-weight: 600;
    margin-bottom: 16px;
    color: var(--ide-text);
  }
}

.settings-description {
  color: var(--ide-text-muted);
  font-size: 13px;
  margin-bottom: 16px;
}

.setting-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 0;
  border-bottom: 1px solid var(--ide-border-subtle);
  
  .setting-info {
    label {
      display: block;
      font-size: 13px;
      font-weight: 500;
      color: var(--ide-text);
      margin-bottom: 2px;
    }
    
    .setting-description {
      font-size: 12px;
      color: var(--ide-text-muted);
    }
  }
}

.toggle {
  position: relative;
  display: inline-block;
  width: 40px;
  height: 22px;
  
  input {
    opacity: 0;
    width: 0;
    height: 0;
  }
  
  .toggle-slider {
    position: absolute;
    cursor: pointer;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background-color: var(--ide-surface);
    border-radius: 22px;
    transition: 0.2s;
    
    &::before {
      position: absolute;
      content: '';
      height: 16px;
      width: 16px;
      left: 3px;
      bottom: 3px;
      background-color: var(--ide-text-muted);
      border-radius: 50%;
      transition: 0.2s;
    }
  }
  
  input:checked + .toggle-slider {
    background-color: var(--ide-primary);
    
    &::before {
      transform: translateX(18px);
      background-color: white;
    }
  }
}

.keybinding-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.keybinding-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
  background: var(--ide-surface);
  border-radius: 4px;
  
  .kb-command {
    font-size: 13px;
    color: var(--ide-text);
  }
  
  .kb-key {
    padding: 2px 8px;
    background: var(--ide-bg);
    border-radius: 4px;
    font-size: 12px;
    font-family: monospace;
    color: var(--ide-text-muted);
  }
}

.settings-footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  padding: 12px 16px;
  border-top: 1px solid var(--ide-border);
}
</style>
