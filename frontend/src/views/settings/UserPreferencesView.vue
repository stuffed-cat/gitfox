<template>
  <div class="preferences-page">
    <!-- 面包屑导航 -->
    <div class="breadcrumb">
      <router-link to="/-/profile">用户设置</router-link>
      <span class="separator">/</span>
      <span>偏好设置</span>
    </div>

    <!-- 搜索框 -->
    <div class="search-box">
      <svg class="search-icon" viewBox="0 0 16 16" width="16" height="16" fill="none">
        <path d="M11.5 7a4.5 4.5 0 1 1-9 0 4.5 4.5 0 0 1 9 0ZM10.5 11a5.5 5.5 0 1 1 1-1l3 3a.75.75 0 1 1-1 1l-3-3Z" stroke="currentColor" stroke-width="1.2"/>
      </svg>
      <input type="text" placeholder="搜索页" v-model="searchQuery" />
    </div>

    <!-- Mode 部分 -->
    <section class="preference-section">
      <h2>Mode</h2>
      <p class="section-description">Choose a color mode.</p>
      
      <div class="radio-group">
        <label class="radio-option">
          <input type="radio" v-model="preferences.mode" value="light" />
          <span class="radio-custom"></span>
          <span class="radio-label">亮色</span>
        </label>
        <label class="radio-option">
          <input type="radio" v-model="preferences.mode" value="dark" />
          <span class="radio-custom"></span>
          <span class="radio-label">Dark</span>
        </label>
        <label class="radio-option">
          <input type="radio" v-model="preferences.mode" value="auto" />
          <span class="radio-custom"></span>
          <span class="radio-label">Auto</span>
        </label>
      </div>
    </section>

    <!-- Theme 部分 -->
    <section class="preference-section">
      <h2>Theme</h2>
      <p class="section-description">Select the accent color for the application interface.</p>
      
      <div class="theme-colors">
        <label class="color-option" :class="{ active: preferences.theme === 'default' }">
          <input type="radio" v-model="preferences.theme" value="default" />
          <div class="color-preview default"></div>
          <span>Default</span>
        </label>
        <label class="color-option" :class="{ active: preferences.theme === 'indigo' }">
          <input type="radio" v-model="preferences.theme" value="indigo" />
          <div class="color-preview indigo"></div>
          <span>靛蓝</span>
        </label>
        <label class="color-option" :class="{ active: preferences.theme === 'blue' }">
          <input type="radio" v-model="preferences.theme" value="blue" />
          <div class="color-preview blue"></div>
          <span>蓝色</span>
        </label>
        <label class="color-option" :class="{ active: preferences.theme === 'green' }">
          <input type="radio" v-model="preferences.theme" value="green" />
          <div class="color-preview green"></div>
          <span>绿色</span>
        </label>
        <label class="color-option" :class="{ active: preferences.theme === 'red' }">
          <input type="radio" v-model="preferences.theme" value="red" />
          <div class="color-preview red"></div>
          <span>红色</span>
        </label>
        <label class="color-option" :class="{ active: preferences.theme === 'gray' }">
          <input type="radio" v-model="preferences.theme" value="gray" />
          <div class="color-preview gray"></div>
          <span>灰色</span>
        </label>
      </div>
    </section>

    <!-- Syntax highlighting 部分 -->
    <section class="preference-section">
      <h2>Syntax highlighting</h2>
      <p class="section-description">Choose a highlight color scheme for viewing and editing code.</p>
      
      <div class="syntax-section">
        <h3>Light color scheme</h3>
        <div class="syntax-options">
          <label class="syntax-option" :class="{ active: preferences.lightSyntax === 'white' }">
            <input type="radio" v-model="preferences.lightSyntax" value="white" />
            <div class="syntax-preview white">
              <div class="code-line"><span class="keyword">class</span> DoctypesCont</div>
              <div class="code-line">  <span class="method">def</span> index</div>
              <div class="code-line">    <span class="variable">@doctypes</span> = Dc</div>
              <div class="code-line">  <span class="keyword">end</span></div>
            </div>
          </label>
          <label class="syntax-option" :class="{ active: preferences.lightSyntax === 'solarized-light' }">
            <input type="radio" v-model="preferences.lightSyntax" value="solarized-light" />
            <div class="syntax-preview solarized-light">
              <div class="code-line"><span class="keyword">class</span> DoctypesCont</div>
              <div class="code-line">  <span class="method">def</span> index</div>
              <div class="code-line">    <span class="variable">@doctypes</span> = Dc</div>
              <div class="code-line">  <span class="keyword">end</span></div>
            </div>
          </label>
          <label class="syntax-option" :class="{ active: preferences.lightSyntax === 'monokai' }">
            <input type="radio" v-model="preferences.lightSyntax" value="monokai" />
            <div class="syntax-preview monokai">
              <div class="code-line"><span class="keyword">class</span> DoctypesCont</div>
              <div class="code-line">  <span class="method">def</span> index</div>
              <div class="code-line">    <span class="variable">@doctypes</span> = Dc</div>
              <div class="code-line">  <span class="keyword">end</span></div>
            </div>
          </label>
          <label class="syntax-option" :class="{ active: preferences.lightSyntax === 'solarized-dark' }">
            <input type="radio" v-model="preferences.lightSyntax" value="solarized-dark" />
            <div class="syntax-preview solarized-dark">
              <div class="code-line"><span class="keyword">class</span> DoctypesCont</div>
              <div class="code-line">  <span class="method">def</span> index</div>
              <div class="code-line">    <span class="variable">@doctypes</span> = Dc</div>
              <div class="code-line">  <span class="keyword">end</span></div>
            </div>
          </label>
          <label class="syntax-option" :class="{ active: preferences.lightSyntax === 'dracula' }">
            <input type="radio" v-model="preferences.lightSyntax" value="dracula" />
            <div class="syntax-preview dracula">
              <div class="code-line"><span class="keyword">class</span> DoctypesCont</div>
              <div class="code-line">  <span class="method">def</span> index</div>
              <div class="code-line">    <span class="variable">@doctypes</span> = Dc</div>
              <div class="code-line">  <span class="keyword">end</span></div>
            </div>
          </label>
          <label class="syntax-option" :class="{ active: preferences.lightSyntax === 'none' }">
            <input type="radio" v-model="preferences.lightSyntax" value="none" />
            <div class="syntax-preview none">
              <div class="code-line">class DoctypesCont</div>
              <div class="code-line">  def index</div>
              <div class="code-line">    @doctypes = Dc</div>
              <div class="code-line">  end</div>
            </div>
          </label>
        </div>
      </div>
    </section>

    <!-- 保存按钮 -->
    <div class="form-actions">
      <button type="button" class="btn btn-primary" @click="savePreferences" :disabled="saving">
        {{ saving ? '保存中...' : '保存更改' }}
      </button>
    </div>

    <div v-if="message" :class="['alert', messageType === 'success' ? 'alert-success' : 'alert-error']">
      {{ message }}
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive } from 'vue'

const searchQuery = ref('')

const preferences = reactive({
  mode: 'light',
  theme: 'default',
  lightSyntax: 'white',
  darkSyntax: 'monokai'
})

const saving = ref(false)
const message = ref('')
const messageType = ref<'success' | 'error'>('success')

const savePreferences = async () => {
  saving.value = true
  message.value = ''

  try {
    localStorage.setItem('user-preferences', JSON.stringify(preferences))
    message.value = '偏好设置已保存'
    messageType.value = 'success'
  } catch (error: any) {
    message.value = error.message || '保存失败'
    messageType.value = 'error'
  } finally {
    saving.value = false
  }
}
</script>

<style lang="scss" scoped>
.preferences-page {
  padding: 24px 40px;
  max-width: 1000px;
  background: #fff;
  min-height: 100vh;
}

.breadcrumb {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
  margin-bottom: 24px;
  color: #737278;
  
  a {
    color: #1f75cb;
    text-decoration: none;
    
    &:hover {
      text-decoration: underline;
    }
  }
  
  .separator {
    color: #737278;
  }
  
  span:last-child {
    color: #303030;
  }
}

.search-box {
  position: relative;
  margin-bottom: 32px;
  
  .search-icon {
    position: absolute;
    left: 12px;
    top: 50%;
    transform: translateY(-50%);
    color: #737278;
  }
  
  input {
    width: 100%;
    padding: 10px 12px 10px 40px;
    font-size: 14px;
    color: #303030;
    background: #fff;
    border: 1px solid #dcdcde;
    border-radius: 4px;
    
    &:focus {
      outline: none;
      border-color: #1f75cb;
      box-shadow: 0 0 0 3px rgba(31, 117, 203, 0.15);
    }
    
    &::placeholder {
      color: #737278;
    }
  }
}

.preference-section {
  margin-bottom: 40px;
  
  h2 {
    font-size: 20px;
    font-weight: 600;
    color: #303030;
    margin: 0 0 8px 0;
  }
  
  .section-description {
    font-size: 14px;
    color: #737278;
    margin: 0 0 20px 0;
  }
  
  h3 {
    font-size: 14px;
    font-weight: 600;
    color: #303030;
    margin: 0 0 16px 0;
  }
}

.radio-group {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.radio-option {
  display: flex;
  align-items: center;
  gap: 10px;
  cursor: pointer;
  font-size: 14px;
  color: #303030;
  
  input[type="radio"] {
    display: none;
  }
  
  .radio-custom {
    width: 16px;
    height: 16px;
    border: 2px solid #868686;
    border-radius: 50%;
    position: relative;
    transition: all 0.2s;
    
    &::after {
      content: '';
      position: absolute;
      top: 50%;
      left: 50%;
      transform: translate(-50%, -50%);
      width: 8px;
      height: 8px;
      background: #1f75cb;
      border-radius: 50%;
      opacity: 0;
      transition: opacity 0.2s;
    }
  }
  
  input[type="radio"]:checked + .radio-custom {
    border-color: #1f75cb;
    
    &::after {
      opacity: 1;
    }
  }
}

.theme-colors {
  display: flex;
  gap: 16px;
  flex-wrap: wrap;
}

.color-option {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
  cursor: pointer;
  
  input[type="radio"] {
    display: none;
  }
  
  .color-preview {
    width: 120px;
    height: 80px;
    border-radius: 8px;
    border: 2px solid #dcdcde;
    transition: border-color 0.2s;
    
    &.default { background: linear-gradient(135deg, #fff 50%, #f0f0f2 50%); }
    &.indigo { background: #6366f1; }
    &.blue { background: #1f75cb; }
    &.green { background: #108548; }
    &.red { background: #dd2b0e; }
    &.gray { background: #e5e5e5; }
  }
  
  &.active .color-preview {
    border-color: #1f75cb;
    border-width: 3px;
  }
  
  span {
    font-size: 13px;
    color: #303030;
    display: flex;
    align-items: center;
    gap: 6px;
    
    &::before {
      content: '';
      display: inline-block;
      width: 14px;
      height: 14px;
      border: 2px solid #868686;
      border-radius: 50%;
    }
  }
  
  &.active span::before {
    border-color: #1f75cb;
    background: #1f75cb;
    box-shadow: inset 0 0 0 2px #fff;
  }
}

.syntax-options {
  display: flex;
  gap: 12px;
  flex-wrap: wrap;
}

.syntax-option {
  cursor: pointer;
  
  input[type="radio"] {
    display: none;
  }
  
  .syntax-preview {
    width: 140px;
    padding: 8px;
    border-radius: 6px;
    border: 2px solid #dcdcde;
    font-family: ui-monospace, SFMono-Regular, 'SF Mono', Menlo, monospace;
    font-size: 10px;
    line-height: 1.4;
    transition: border-color 0.2s;
    
    &.white {
      background: #fff;
      color: #303030;
      .keyword { color: #d73a49; }
      .method { color: #6f42c1; }
      .variable { color: #005cc5; }
    }
    
    &.solarized-light {
      background: #fdf6e3;
      color: #657b83;
      .keyword { color: #859900; }
      .method { color: #268bd2; }
      .variable { color: #b58900; }
    }
    
    &.monokai {
      background: #272822;
      color: #f8f8f2;
      .keyword { color: #f92672; }
      .method { color: #a6e22e; }
      .variable { color: #66d9ef; }
    }
    
    &.solarized-dark {
      background: #002b36;
      color: #839496;
      .keyword { color: #859900; }
      .method { color: #268bd2; }
      .variable { color: #b58900; }
    }
    
    &.dracula {
      background: #282a36;
      color: #f8f8f2;
      .keyword { color: #ff79c6; }
      .method { color: #50fa7b; }
      .variable { color: #8be9fd; }
    }
    
    &.none {
      background: #f6f8fa;
      color: #303030;
    }
  }
  
  &.active .syntax-preview {
    border-color: #1f75cb;
  }
  
  .code-line {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
}

.form-actions {
  margin-top: 32px;
  padding-top: 24px;
  border-top: 1px solid #dcdcde;
}

.btn {
  padding: 10px 20px;
  font-size: 14px;
  font-weight: 500;
  border-radius: 4px;
  cursor: pointer;
  border: none;
  
  &.btn-primary {
    background: #1f75cb;
    color: white;
    
    &:hover:not(:disabled) {
      background: #1068bf;
    }
    
    &:disabled {
      opacity: 0.6;
      cursor: not-allowed;
    }
  }
}

.alert {
  margin-top: 16px;
  padding: 12px 16px;
  border-radius: 4px;
  font-size: 14px;
  
  &.alert-success {
    background: #ecf4ee;
    border: 1px solid #108548;
    color: #108548;
  }
  
  &.alert-error {
    background: #fcf1ef;
    border: 1px solid #dd2b0e;
    color: #dd2b0e;
  }
}
</style>
