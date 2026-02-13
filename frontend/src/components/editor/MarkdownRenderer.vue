<template>
  <div class="md-content gl-markdown" v-html="renderedHtml"></div>
</template>

<script setup lang="ts">
import { computed } from 'vue'

interface Props {
  content: string
}

const props = defineProps<Props>()

// 简单的 markdown 解析
function parseMarkdown(text: string): string {
  if (!text) return ''
  
  // 先处理代码块，避免内部被解析
  const codeBlocks: string[] = []
  let html = text.replace(/```(\w*)\n([\s\S]*?)```/g, (_, lang, code) => {
    codeBlocks.push(`<pre><code class="language-${lang}">${escapeHtml(code)}</code></pre>`)
    return `__CODE_BLOCK_${codeBlocks.length - 1}__`
  })
  
  // 按行处理
  const lines = html.split('\n')
  const result: string[] = []
  let inList = false
  
  for (const line of lines) {
    const trimmed = line.trim()
    
    // 标题
    if (trimmed.startsWith('### ')) {
      if (inList) { result.push('</ul>'); inList = false }
      result.push(`<h3>${trimmed.slice(4)}</h3>`)
    } else if (trimmed.startsWith('## ')) {
      if (inList) { result.push('</ul>'); inList = false }
      result.push(`<h2>${trimmed.slice(3)}</h2>`)
    } else if (trimmed.startsWith('# ')) {
      if (inList) { result.push('</ul>'); inList = false }
      result.push(`<h1>${trimmed.slice(2)}</h1>`)
    }
    // 列表
    else if (trimmed.startsWith('- ')) {
      if (!inList) { result.push('<ul>'); inList = true }
      result.push(`<li>${trimmed.slice(2)}</li>`)
    }
    // 空行
    else if (trimmed === '') {
      if (inList) { result.push('</ul>'); inList = false }
      result.push('')
    }
    // 普通段落
    else {
      if (inList) { result.push('</ul>'); inList = false }
      result.push(`<p>${trimmed}</p>`)
    }
  }
  if (inList) result.push('</ul>')
  
  html = result.join('\n')
  
  // 行内格式
  html = html
    .replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>')
    .replace(/\*(.+?)\*/g, '<em>$1</em>')
    .replace(/`([^`]+)`/g, '<code>$1</code>')
    .replace(/\[([^\]]+)\]\(([^)]+)\)/g, '<a href="$2">$1</a>')
  
  // 恢复代码块
  codeBlocks.forEach((block, i) => {
    html = html.replace(`__CODE_BLOCK_${i}__`, block)
  })
  
  return html
}

function escapeHtml(text: string): string {
  return text.replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#039;' }[c] || c))
}

const renderedHtml = computed(() => parseMarkdown(props.content))
</script>

<style lang="scss">
@import '@/styles/variables.scss';

// GitLab 风格的 Markdown 渲染样式
.gl-markdown {
  font-size: 14px;
  line-height: 1.6;
  color: $gray-900;
  word-wrap: break-word;

  // 标题
  h1, h2, h3, h4, h5, h6 {
    margin-top: 24px;
    margin-bottom: 16px;
    font-weight: 600;
    line-height: 1.25;
    color: $gray-900;

    &:first-child {
      margin-top: 0;
    }
  }

  h1 {
    font-size: 1.75em;
    padding-bottom: 0.3em;
    border-bottom: 1px solid $gray-200;
  }

  h2 {
    font-size: 1.5em;
    padding-bottom: 0.3em;
    border-bottom: 1px solid $gray-200;
  }

  h3 { font-size: 1.25em; }
  h4 { font-size: 1em; }
  h5 { font-size: 0.875em; }
  h6 { font-size: 0.85em; color: $gray-600; }

  // 段落
  p {
    margin-top: 0;
    margin-bottom: 16px;
  }

  // 链接
  a {
    color: $brand-primary;
    text-decoration: none;

    &:hover {
      text-decoration: underline;
    }
  }

  // 强调
  strong { font-weight: 600; }
  em { font-style: italic; }

  // 列表
  ul, ol {
    margin-top: 0;
    margin-bottom: 16px;
    padding-left: 2em;
  }

  li {
    margin-top: 4px;

    > p {
      margin-bottom: 8px;
    }

    + li {
      margin-top: 4px;
    }
  }

  // 嵌套列表
  ul ul, ul ol, ol ul, ol ol {
    margin-top: 0;
    margin-bottom: 0;
  }

  // 任务列表 (GFM)
  .task-list-item {
    list-style-type: none;
    margin-left: -1.5em;

    input[type="checkbox"] {
      margin-right: 0.5em;
    }
  }

  // 引用块
  blockquote {
    margin: 0 0 16px;
    padding: 8px 16px;
    color: $gray-600;
    border-left: 4px solid $gray-300;
    background-color: $gray-50;

    > :first-child { margin-top: 0; }
    > :last-child { margin-bottom: 0; }
  }

  // 内联代码
  code:not(pre code) {
    padding: 0.2em 0.4em;
    margin: 0;
    font-size: 85%;
    font-family: $font-mono;
    background-color: rgba($gray-500, 0.15);
    border-radius: 4px;
    color: $gray-900;
  }

  // 代码块
  pre {
    margin-top: 0;
    margin-bottom: 16px;
    padding: 16px;
    overflow: auto;
    font-size: 85%;
    line-height: 1.45;
    background-color: $gray-900;
    border-radius: 4px;

    code {
      display: block;
      padding: 0;
      margin: 0;
      overflow: visible;
      line-height: inherit;
      word-wrap: normal;
      background-color: transparent;
      border: 0;
      font-family: $font-mono;
      font-size: inherit;
      color: inherit;
    }
  }

  // 表格
  table {
    display: block;
    width: max-content;
    max-width: 100%;
    overflow: auto;
    margin-top: 0;
    margin-bottom: 16px;
    border-spacing: 0;
    border-collapse: collapse;

    th, td {
      padding: 8px 12px;
      border: 1px solid $gray-300;
    }

    th {
      font-weight: 600;
      background-color: $gray-100;
    }

    tr {
      background-color: #fff;
      border-top: 1px solid $gray-300;

      &:nth-child(2n) {
        background-color: $gray-50;
      }
    }
  }

  // 图片
  img {
    max-width: 100%;
    height: auto;
    box-sizing: content-box;
    background-color: #fff;
    border-style: none;
  }

  // 分隔线
  hr {
    height: 4px;
    padding: 0;
    margin: 24px 0;
    background-color: $gray-200;
    border: 0;
  }

  // 删除线
  del {
    text-decoration: line-through;
  }

  // highlight.js 覆盖
  .hljs {
    background: transparent !important;
    padding: 0 !important;
  }
}
</style>
