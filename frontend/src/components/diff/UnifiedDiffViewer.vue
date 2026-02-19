<template>
  <div class="unified-diff-viewer">
    <div 
      v-for="(line, index) in parsedLines" 
      :key="index" 
      :class="['diff-line', line.type]"
    >
      <span class="line-number old">{{ line.oldNum }}</span>
      <span class="line-number new">{{ line.newNum }}</span>
      <span class="line-content" v-html="line.html"></span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import hljs from 'highlight.js'

interface Props {
  diff: string
  language?: string
}

const props = withDefaults(defineProps<Props>(), {
  language: 'plaintext',
})

interface DiffLine {
  type: 'add' | 'delete' | 'context' | 'header'
  oldNum: string
  newNum: string
  content: string
  html: string
}

const parsedLines = computed(() => {
  const lines = props.diff.split('\n')
  const result: DiffLine[] = []
  let oldLineNum = 0
  let newLineNum = 0
  
  for (const line of lines) {
    if (!line) continue
    
    const firstChar = line[0]
    const content = line.slice(1)
    
    let type: DiffLine['type'] = 'context'
    let oldNum = ''
    let newNum = ''
    
    if (firstChar === '+') {
      type = 'add'
      newLineNum++
      newNum = String(newLineNum)
    } else if (firstChar === '-') {
      type = 'delete'
      oldLineNum++
      oldNum = String(oldLineNum)
    } else if (firstChar === ' ') {
      type = 'context'
      oldLineNum++
      newLineNum++
      oldNum = String(oldLineNum)
      newNum = String(newLineNum)
    } else if (firstChar === 'F' || firstChar === 'H') {
      type = 'header'
      // 文件头或 hunk 头，解析行号
      if (line.includes('@@')) {
        const match = line.match(/@@ -(\d+)/)
        if (match) {
          oldLineNum = parseInt(match[1]) - 1
          newLineNum = oldLineNum
        }
      }
    } else {
      continue
    }
    
    // 语法高亮
    let html = content
    if (type !== 'header' && props.language !== 'plaintext') {
      try {
        html = hljs.highlight(content, { language: props.language, ignoreIllegals: true }).value
      } catch {
        html = escapeHtml(content)
      }
    } else {
      html = escapeHtml(content)
    }
    
    result.push({ type, oldNum, newNum, content, html })
  }
  
  return result
})

function escapeHtml(text: string): string {
  const div = document.createElement('div')
  div.textContent = text
  return div.innerHTML
}
</script>

<style lang="scss" scoped>
.unified-diff-viewer {
  font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', 'Consolas', monospace;
  font-size: 12px;
  line-height: 20px;
  background: #0d1117;
  border: 1px solid #30363d;
  border-radius: 6px;
  overflow-x: auto;
}

.diff-line {
  display: flex;
  min-height: 20px;
  
  &:hover {
    background: rgba(56, 139, 253, 0.1);
  }
  
  &.add {
    background: rgba(46, 160, 67, 0.15);
    
    .line-content {
      color: #aff5b4;
    }
    
    &::before {
      content: '+';
      color: #3fb950;
      font-weight: bold;
      padding: 0 8px;
    }
  }
  
  &.delete {
    background: rgba(248, 81, 73, 0.15);
    
    .line-content {
      color: #ffdcd7;
    }
    
    &::before {
      content: '-';
      color: #f85149;
      font-weight: bold;
      padding: 0 8px;
    }
  }
  
  &.context {
    &::before {
      content: ' ';
      padding: 0 8px;
    }
  }
  
  &.header {
    background: rgba(56, 139, 253, 0.15);
    color: #79c0ff;
    font-weight: 600;
    
    &::before {
      content: '';
    }
    
    .line-number {
      display: none;
    }
  }
}

.line-number {
  flex-shrink: 0;
  width: 50px;
  padding: 0 8px;
  text-align: right;
  color: #6e7681;
  user-select: none;
  
  &.old {
    border-right: 1px solid #30363d;
  }
  
  &.new {
    border-right: 1px solid #30363d;
    margin-right: 8px;
  }
}

.line-content {
  flex: 1;
  white-space: pre;
  padding-right: 16px;
  color: #c9d1d9;
  
  :deep(code) {
    background: none;
    padding: 0;
  }
}
</style>
