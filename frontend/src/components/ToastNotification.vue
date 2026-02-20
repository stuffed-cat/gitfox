<template>
  <Teleport to="body">
    <TransitionGroup name="toast" tag="div" class="toast-container">
      <div
        v-for="toast in toasts"
        :key="toast.id"
        class="toast"
        :class="toast.type"
        @click="removeToast(toast.id)"
      >
        <div class="toast-icon">
          <svg v-if="toast.type === 'success'" viewBox="0 0 24 24" width="20" height="20">
            <circle cx="12" cy="12" r="10" fill="currentColor" opacity="0.2"/>
            <path d="M8 12l3 3 5-6" stroke="currentColor" stroke-width="2" fill="none" stroke-linecap="round"/>
          </svg>
          <svg v-else-if="toast.type === 'error'" viewBox="0 0 24 24" width="20" height="20">
            <circle cx="12" cy="12" r="10" fill="currentColor" opacity="0.2"/>
            <path d="M8 8l8 8M16 8l-8 8" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
          </svg>
          <svg v-else-if="toast.type === 'warning'" viewBox="0 0 24 24" width="20" height="20">
            <path d="M12 2L2 20h20L12 2z" fill="currentColor" opacity="0.2"/>
            <path d="M12 9v4M12 16v.5" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
          </svg>
          <svg v-else viewBox="0 0 24 24" width="20" height="20">
            <circle cx="12" cy="12" r="10" fill="currentColor" opacity="0.2"/>
            <path d="M12 11v5M12 8v.5" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
          </svg>
        </div>
        <div class="toast-content">
          <div v-if="toast.title" class="toast-title">{{ toast.title }}</div>
          <div class="toast-message">{{ toast.message }}</div>
        </div>
        <button class="toast-close" @click.stop="removeToast(toast.id)">
          <svg viewBox="0 0 16 16" width="16" height="16">
            <path d="M4 4l8 8M12 4l-8 8" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
          </svg>
        </button>
      </div>
    </TransitionGroup>
  </Teleport>
</template>

<script setup lang="ts">
import { ref } from 'vue'

export interface Toast {
  id: number
  type: 'success' | 'error' | 'warning' | 'info'
  title?: string
  message: string
  duration?: number
}

const toasts = ref<Toast[]>([])
let idCounter = 0

function addToast(toast: Omit<Toast, 'id'>) {
  const id = ++idCounter
  const duration = toast.duration ?? 3000
  
  toasts.value.push({
    ...toast,
    id
  })
  
  if (duration > 0) {
    setTimeout(() => {
      removeToast(id)
    }, duration)
  }
}

function removeToast(id: number) {
  const index = toasts.value.findIndex(t => t.id === id)
  if (index !== -1) {
    toasts.value.splice(index, 1)
  }
}

defineExpose({
  success: (message: string, title?: string, duration?: number) => 
    addToast({ type: 'success', message, title, duration }),
  error: (message: string, title?: string, duration?: number) => 
    addToast({ type: 'error', message, title, duration }),
  warning: (message: string, title?: string, duration?: number) => 
    addToast({ type: 'warning', message, title, duration }),
  info: (message: string, title?: string, duration?: number) => 
    addToast({ type: 'info', message, title, duration }),
})
</script>

<style lang="scss" scoped>
.toast-container {
  position: fixed;
  top: 20px;
  right: 20px;
  z-index: 10000;
  display: flex;
  flex-direction: column;
  gap: 12px;
  pointer-events: none;
}

.toast {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  padding: 16px;
  border-radius: 8px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
  background: white;
  min-width: 320px;
  max-width: 480px;
  pointer-events: auto;
  cursor: pointer;
  border-left: 4px solid;
  
  &.success {
    border-left-color: #10b981;
    
    .toast-icon {
      color: #10b981;
    }
  }
  
  &.error {
    border-left-color: #ef4444;
    
    .toast-icon {
      color: #ef4444;
    }
  }
  
  &.warning {
    border-left-color: #f59e0b;
    
    .toast-icon {
      color: #f59e0b;
    }
  }
  
  &.info {
    border-left-color: #3b82f6;
    
    .toast-icon {
      color: #3b82f6;
    }
  }
}

.toast-icon {
  flex-shrink: 0;
  margin-top: 2px;
}

.toast-content {
  flex: 1;
  min-width: 0;
}

.toast-title {
  font-weight: 600;
  font-size: 14px;
  color: #1f2937;
  margin-bottom: 4px;
}

.toast-message {
  font-size: 14px;
  color: #4b5563;
  line-height: 1.5;
  word-wrap: break-word;
}

.toast-close {
  flex-shrink: 0;
  padding: 0;
  border: none;
  background: none;
  cursor: pointer;
  color: #9ca3af;
  transition: color 0.2s;
  
  &:hover {
    color: #4b5563;
  }
}

.toast-enter-active,
.toast-leave-active {
  transition: all 0.3s ease;
}

.toast-enter-from {
  opacity: 0;
  transform: translateX(100%);
}

.toast-leave-to {
  opacity: 0;
  transform: translateX(20px) scale(0.95);
}

.toast-move {
  transition: transform 0.3s ease;
}
</style>
