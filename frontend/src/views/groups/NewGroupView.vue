<template>
  <div class="new-group-page">
    <div class="page-header">
      <h1>新建群组</h1>
    </div>
    
    <form @submit.prevent="createGroup" class="group-form">
      <div class="form-group">
        <label for="name">群组名称</label>
        <input 
          id="name" 
          v-model="form.name" 
          type="text" 
          required 
          placeholder="我的群组"
        />
      </div>
      
      <div class="form-group">
        <label for="path">群组路径</label>
        <div class="path-input">
          <span class="prefix">{{ baseUrl }}/</span>
          <input 
            id="path" 
            v-model="form.path" 
            type="text" 
            required 
            placeholder="my-group"
          />
        </div>
      </div>
      
      <div class="form-group">
        <label for="description">描述（可选）</label>
        <textarea 
          id="description" 
          v-model="form.description" 
          rows="3"
          placeholder="群组描述..."
        ></textarea>
      </div>
      
      <div class="form-group">
        <label>可见性级别</label>
        <div class="visibility-options">
          <label class="radio-option">
            <input type="radio" v-model="form.visibility" value="private" />
            <div class="option-content">
              <strong>私有</strong>
              <span>仅群组成员可见</span>
            </div>
          </label>
          <label class="radio-option">
            <input type="radio" v-model="form.visibility" value="internal" />
            <div class="option-content">
              <strong>内部</strong>
              <span>登录用户可见</span>
            </div>
          </label>
          <label class="radio-option">
            <input type="radio" v-model="form.visibility" value="public" />
            <div class="option-content">
              <strong>公开</strong>
              <span>所有人可见</span>
            </div>
          </label>
        </div>
      </div>
      
      <div class="form-actions">
        <router-link to="/dashboard/groups" class="btn btn-outline">取消</router-link>
        <button type="submit" class="btn btn-primary" :disabled="submitting">
          {{ submitting ? '创建中...' : '创建群组' }}
        </button>
      </div>
    </form>
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { useRouter } from 'vue-router'

const router = useRouter()

const baseUrl = window.location.origin

const form = ref({
  name: '',
  path: '',
  description: '',
  visibility: 'private'
})

const submitting = ref(false)

watch(() => form.value.name, (name) => {
  form.value.path = name.toLowerCase().replace(/\s+/g, '-').replace(/[^a-z0-9-]/g, '')
})

async function createGroup() {
  submitting.value = true
  // TODO: 实现群组创建 API
  setTimeout(() => {
    submitting.value = false
    router.push('/dashboard/groups')
  }, 1000)
}
</script>

<style lang="scss" scoped>
.new-group-page {
  padding: $spacing-6;
  max-width: 600px;
  margin: 0 auto;
}

.page-header {
  margin-bottom: $spacing-6;
  h1 { font-size: $text-2xl; font-weight: 600; }
}

.group-form {
  .form-group {
    margin-bottom: $spacing-5;
    
    label {
      display: block;
      margin-bottom: $spacing-2;
      font-weight: 500;
      color: $text-primary;
    }
    
    input, textarea {
      width: 100%;
      padding: $spacing-2 $spacing-3;
      border: 1px solid $border-color;
      border-radius: $radius-md;
      background: $bg-primary;
      color: $text-primary;
      
      &:focus {
        outline: none;
        border-color: $color-primary;
      }
    }
    
    .path-input {
      display: flex;
      align-items: center;
      border: 1px solid $border-color;
      border-radius: $radius-md;
      overflow: hidden;
      
      .prefix {
        padding: $spacing-2 $spacing-3;
        background: $bg-tertiary;
        color: $text-muted;
        border-right: 1px solid $border-color;
      }
      
      input {
        border: none;
        border-radius: 0;
      }
    }
  }
  
  .visibility-options {
    display: flex;
    flex-direction: column;
    gap: $spacing-2;
    
    .radio-option {
      display: flex;
      align-items: flex-start;
      gap: $spacing-3;
      padding: $spacing-3;
      border: 1px solid $border-color;
      border-radius: $radius-md;
      cursor: pointer;
      
      &:has(input:checked) {
        border-color: $color-primary;
        background: rgba($color-primary, 0.05);
      }
      
      input { margin-top: 4px; }
      
      .option-content {
        strong { display: block; color: $text-primary; }
        span { font-size: $text-sm; color: $text-secondary; }
      }
    }
  }
  
  .form-actions {
    display: flex;
    justify-content: flex-end;
    gap: $spacing-3;
    margin-top: $spacing-6;
  }
}
</style>
