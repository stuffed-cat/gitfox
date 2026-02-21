<template>
  <div class="home-view">
    <div class="home-content">
      <div class="logo">
        <svg viewBox="0 0 64 64" fill="none">
          <path d="M32 4L4 20v24l28 16 28-16V20L32 4z" fill="currentColor" opacity="0.1"/>
          <path d="M32 4L4 20v24l28 16 28-16V20L32 4z" stroke="currentColor" stroke-width="2"/>
          <path d="M32 24v20M22 29v10M42 29v10" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
        </svg>
        <h1>GitFox WebIDE</h1>
      </div>
      
      <p class="description">
        在浏览器中编辑代码，支持 VSCode 扩展和主题
      </p>
      
      <div class="quick-open">
        <div class="input-wrapper">
          <svg viewBox="0 0 24 24" fill="none">
            <circle cx="11" cy="11" r="7" stroke="currentColor" stroke-width="2"/>
            <path d="M21 21l-4.35-4.35" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
          </svg>
          <input 
            type="text" 
            v-model="projectPath"
            placeholder="输入项目路径 (例如: owner/repo)"
            @keydown.enter="openProject"
          />
          <button class="btn btn-primary" @click="openProject" :disabled="!projectPath">
            打开
          </button>
        </div>
      </div>
      
      <div v-if="recentProjects.length > 0" class="recent-projects">
        <h3>最近项目</h3>
        <div class="project-list">
          <div 
            v-for="project in recentProjects" 
            :key="project"
            class="project-item"
            @click="goToProject(project)"
          >
            <svg viewBox="0 0 24 24" fill="none">
              <path d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" fill="currentColor" opacity="0.2"/>
              <path d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" stroke="currentColor" stroke-width="1.5"/>
            </svg>
            <span>{{ project }}</span>
          </div>
        </div>
      </div>
      
      <div class="shortcuts">
        <h3>快捷键</h3>
        <div class="shortcut-grid">
          <div class="shortcut-item">
            <kbd>Ctrl</kbd> + <kbd>S</kbd>
            <span>保存文件</span>
          </div>
          <div class="shortcut-item">
            <kbd>Ctrl</kbd> + <kbd>P</kbd>
            <span>快速打开</span>
          </div>
          <div class="shortcut-item">
            <kbd>Ctrl</kbd> + <kbd>Shift</kbd> + <kbd>P</kbd>
            <span>命令面板</span>
          </div>
          <div class="shortcut-item">
            <kbd>Ctrl</kbd> + <kbd>B</kbd>
            <span>切换侧边栏</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'

const router = useRouter()
const projectPath = ref('')
const recentProjects = ref<string[]>([])

onMounted(() => {
  // Load recent projects from localStorage
  const stored = localStorage.getItem('gitfox-recent-projects')
  if (stored) {
    recentProjects.value = JSON.parse(stored)
  }
})

function openProject() {
  if (!projectPath.value) return
  
  const path = projectPath.value.trim()
  goToProject(path)
}

function goToProject(path: string) {
  // Add to recent projects
  const recent = recentProjects.value.filter(p => p !== path)
  recent.unshift(path)
  recentProjects.value = recent.slice(0, 10)
  localStorage.setItem('gitfox-recent-projects', JSON.stringify(recentProjects.value))
  
  // Navigate to IDE
  router.push(`/${path}`)
}
</script>

<style lang="scss" scoped>
.home-view {
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--ide-bg);
}

.home-content {
  max-width: 600px;
  padding: 40px;
  text-align: center;
}

.logo {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 16px;
  margin-bottom: 16px;
  
  svg {
    width: 80px;
    height: 80px;
    color: var(--ide-primary);
  }
  
  h1 {
    font-size: 28px;
    font-weight: 600;
    color: var(--ide-text);
  }
}

.description {
  color: var(--ide-text-muted);
  margin-bottom: 32px;
}

.quick-open {
  margin-bottom: 40px;
  
  .input-wrapper {
    display: flex;
    align-items: center;
    gap: 8px;
    background: var(--ide-surface);
    border: 1px solid var(--ide-border);
    border-radius: 8px;
    padding: 8px 12px;
    
    svg {
      width: 20px;
      height: 20px;
      color: var(--ide-text-muted);
      flex-shrink: 0;
    }
    
    input {
      flex: 1;
      background: none;
      border: none;
      color: var(--ide-text);
      font-size: 14px;
      outline: none;
      
      &::placeholder {
        color: var(--ide-text-subtle);
      }
    }
  }
}

.recent-projects {
  text-align: left;
  margin-bottom: 40px;
  
  h3 {
    font-size: 14px;
    font-weight: 600;
    color: var(--ide-text-muted);
    margin-bottom: 12px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }
  
  .project-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  
  .project-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    background: var(--ide-surface);
    border-radius: 6px;
    cursor: pointer;
    transition: background var(--ide-transition-fast);
    
    &:hover {
      background: var(--ide-surface-hover);
    }
    
    svg {
      width: 18px;
      height: 18px;
      color: var(--ide-primary);
    }
    
    span {
      font-size: 14px;
      color: var(--ide-text);
    }
  }
}

.shortcuts {
  text-align: left;
  
  h3 {
    font-size: 14px;
    font-weight: 600;
    color: var(--ide-text-muted);
    margin-bottom: 12px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }
  
  .shortcut-grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 12px;
  }
  
  .shortcut-item {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    color: var(--ide-text-muted);
    
    kbd {
      display: inline-block;
      padding: 2px 6px;
      background: var(--ide-surface);
      border: 1px solid var(--ide-border);
      border-radius: 4px;
      font-family: inherit;
      font-size: 11px;
      color: var(--ide-text);
    }
    
    span {
      margin-left: auto;
      color: var(--ide-text-subtle);
    }
  }
}
</style>
