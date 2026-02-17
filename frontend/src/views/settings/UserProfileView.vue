<template>
  <div class="profile-page">
    <!-- 面包屑导航 -->
    <div class="breadcrumb">
      <router-link to="/-/profile">用户设置</router-link>
      <span class="separator">/</span>
      <span>编辑个人资料</span>
    </div>

    <!-- 搜索框 -->
    <div class="search-box">
      <svg class="search-icon" viewBox="0 0 16 16" width="16" height="16" fill="none">
        <path d="M11.5 7a4.5 4.5 0 1 1-9 0 4.5 4.5 0 0 1 9 0ZM10.5 11a5.5 5.5 0 1 1 1-1l3 3a.75.75 0 1 1-1 1l-3-3Z" stroke="currentColor" stroke-width="1.2"/>
      </svg>
      <input type="text" placeholder="搜索页" v-model="searchQuery" />
    </div>

    <!-- 公开头像 -->
    <section class="profile-section">
      <h2>公开头像</h2>
      <p class="section-description">
        可以在这里上传您的头像
      </p>
      
      <div class="avatar-upload">
        <div class="avatar-preview">
          <img v-if="profile.avatar_url" :src="profile.avatar_url" alt="头像" />
          <div v-else class="avatar-placeholder">
            <svg viewBox="0 0 24 24" width="40" height="40" fill="none">
              <circle cx="12" cy="8" r="4" stroke="currentColor" stroke-width="1.5"/>
              <path d="M4 20c0-4 4-6 8-6s8 2 8 6" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
            </svg>
          </div>
        </div>
        <div class="avatar-actions">
          <h3>上传新头像</h3>
          <div class="file-input-wrapper">
            <label class="file-input-btn">
              选择文件...
              <input ref="avatarFileInput" type="file" accept="image/*" @change="handleAvatarChange" />
            </label>
            <span class="file-name">{{ avatarFileName || '未选择文件。' }}</span>
          </div>
          <p class="upload-hint">理想的图像尺寸为 192 x 192 像素。允许的最大文件大小为 200 KiB。</p>
        </div>
      </div>
    </section>

    <!-- 头像裁剪模态框 -->
    <div v-if="showAvatarCropper" class="avatar-cropper-modal">
      <div class="modal-content">
        <div class="modal-header">
          <h2>裁剪头像</h2>
          <button class="close-btn" @click="showAvatarCropper = false">
            <svg viewBox="0 0 16 16" width="16" height="16" fill="none">
              <path d="M4 4l8 8M12 4l-8 8" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
            </svg>
          </button>
        </div>
        <div class="modal-body">
          <div class="cropper-container-wrapper">
            <div 
              class="cropper-container" 
              ref="cropperContainer"
              @mousedown="handleCropperMouseDown"
              @mousemove="handleCropperMouseMove"
              @mouseup="handleCropperMouseUp"
              @mouseleave="handleCropperMouseUp"
            >
              <img 
                v-if="avatarPreview" 
                :src="avatarPreview" 
                alt="预览" 
                ref="cropperImg"
                class="cropper-image"
                @load="onCropperImageLoad"
                draggable="false"
              />
              <!-- 半透明遮罩层 -->
              <div class="crop-mask"></div>
              <!-- 裁剪框 - 使用clip-path显示清晰区域 -->
              <div 
                class="crop-box"
                :style="{ 
                  left: cropBox.x + 'px', 
                  top: cropBox.y + 'px', 
                  width: cropBox.size + 'px', 
                  height: cropBox.size + 'px' 
                }"
              >
                <!-- 四角调整手柄 -->
                <div class="resize-handle nw" data-handle="nw"></div>
                <div class="resize-handle ne" data-handle="ne"></div>
                <div class="resize-handle sw" data-handle="sw"></div>
                <div class="resize-handle se" data-handle="se"></div>
              </div>
              <!-- 用SVG绘制遮罩 -->
              <svg class="crop-overlay-svg" width="100%" height="100%">
                <defs>
                  <mask id="cropMask">
                    <rect width="100%" height="100%" fill="white"/>
                    <rect 
                      :x="cropBox.x" 
                      :y="cropBox.y" 
                      :width="cropBox.size" 
                      :height="cropBox.size" 
                      fill="black"
                    />
                  </mask>
                </defs>
                <rect width="100%" height="100%" fill="rgba(0,0,0,0.5)" mask="url(#cropMask)"/>
              </svg>
            </div>
            <div class="crop-preview">
              <div class="preview-label">预览</div>
              <canvas ref="cropperCanvas" class="preview-canvas"></canvas>
              <div class="crop-size">{{ Math.round(cropBox.size) }} × {{ Math.round(cropBox.size) }}</div>
            </div>
          </div>
        </div>
        <div class="modal-footer">
          <button class="btn btn-secondary" @click="showAvatarCropper = false">取消</button>
          <button class="btn btn-primary" @click="confirmAvatarCrop">确认裁剪</button>
        </div>
      </div>
    </div>

    <!-- 当前状态 -->
    <section class="profile-section">
      <h2>当前状态</h2>
      <p class="section-description">此表情符号和消息会显示在您的个人资料和界面中。</p>
      
      <div class="status-input-wrapper">
        <button class="emoji-btn" @click="showEmojiPicker = !showEmojiPicker">
          <span v-if="profile.status_emoji">{{ profile.status_emoji }}</span>
          <svg v-else viewBox="0 0 16 16" width="16" height="16" fill="none">
            <circle cx="8" cy="8" r="7" stroke="currentColor" stroke-width="1.2"/>
            <circle cx="5.5" cy="6.5" r="1" fill="currentColor"/>
            <circle cx="10.5" cy="6.5" r="1" fill="currentColor"/>
            <path d="M5 10c.5 1 1.5 1.5 3 1.5s2.5-.5 3-1.5" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
          </svg>
        </button>
        <input 
          type="text" 
          v-model="profile.status_message"
          placeholder="您的状态是什么？"
          maxlength="100"
          class="status-input"
        />
        <button v-if="profile.status_message" class="clear-btn" @click="profile.status_message = ''">
          <svg viewBox="0 0 16 16" width="16" height="16" fill="none">
            <path d="M4 4l8 8M12 4l-8 8" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          </svg>
        </button>
      </div>
      <p class="char-count">剩余 {{ 100 - (profile.status_message?.length || 0) }} 个字符。</p>

      <div class="busy-checkbox">
        <label class="checkbox-label">
          <input type="checkbox" v-model="profile.busy" />
          <span class="checkbox-custom"></span>
          <div class="checkbox-text">
            <span class="checkbox-title">设置自己为忙碌中</span>
            <span class="checkbox-desc">显示您正忙或无法响应</span>
          </div>
        </label>
      </div>

      <div class="clear-status">
        <label>清除状态</label>
        <select v-model="profile.clear_status_after" class="form-select">
          <option value="never">从不</option>
          <option value="30m">30 分钟后</option>
          <option value="1h">1 小时后</option>
          <option value="4h">4 小时后</option>
          <option value="today">今天</option>
          <option value="1w">1 周后</option>
        </select>
      </div>
    </section>

    <hr class="divider" />

    <!-- 表单按钮 -->
    <div class="form-actions">
      <button type="button" class="btn btn-primary" @click="saveProfile" :disabled="saving">
        {{ saving ? '保存中...' : '更新个人资料设置' }}
      </button>
      <button type="button" class="btn btn-secondary" @click="resetForm">
        取消
      </button>
    </div>

    <div v-if="message" :class="['alert', messageType === 'success' ? 'alert-success' : 'alert-error']">
      {{ message }}
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted, watch } from 'vue'
import { useAuthStore } from '@/stores/auth'
import apiClient from '@/api'

const authStore = useAuthStore()
const searchQuery = ref('')
const avatarFileName = ref('')
const showEmojiPicker = ref(false)
const showAvatarCropper = ref(false)
const avatarFileInput = ref<HTMLInputElement | null>(null)
const cropperImg = ref<HTMLImageElement | null>(null)
const cropperContainer = ref<HTMLDivElement | null>(null)
const avatarPreview = ref('')
let selectedAvatarFile: File | null = null

// 裁剪框状态 - 可自由拖动和调整大小（正方形）
const cropBox = reactive({
  x: 50,
  y: 50,
  size: 150  // 正方形，只需要一个size
})

// 拖拽/调整状态
let dragMode: 'move' | 'resize' | null = null
let activeHandle: string | null = null
let startMouseX = 0
let startMouseY = 0
let startCropX = 0
let startCropY = 0
let startCropWidth = 0
let startCropHeight = 0

const profile = reactive({
  username: '',
  display_name: '',
  email: '',
  bio: '',
  avatar_url: '',
  status_emoji: '',
  status_message: '',
  busy: false,
  clear_status_after: 'never'
})

const saving = ref(false)
const message = ref('')
const messageType = ref<'success' | 'error'>('success')

const loadProfile = () => {
  const user = authStore.user
  if (user) {
    profile.username = user.username || ''
    profile.display_name = user.display_name || ''
    profile.email = user.email || ''
    profile.bio = ''
    profile.avatar_url = user.avatar_url || ''
    profile.status_emoji = user.status_emoji || ''
    profile.status_message = user.status_message || ''
    profile.busy = user.busy || false
  }
}

const handleAvatarChange = (event: Event) => {
  const target = event.target as HTMLInputElement
  if (target.files && target.files[0]) {
    const file = target.files[0]
    avatarFileName.value = file.name
    selectedAvatarFile = file
    
    // Show preview in cropper modal
    const reader = new FileReader()
    reader.onload = (e) => {
      avatarPreview.value = e.target?.result as string
      // 重置裁剪框到合理初始位置
      cropBox.x = 50
      cropBox.y = 50
      cropBox.size = 150
      showAvatarCropper.value = true
    }
    reader.readAsDataURL(file)
  }
}

const onCropperImageLoad = () => {
  // 根据图像大小初始化裁剪框
  if (cropperImg.value && cropperContainer.value) {
    const img = cropperImg.value
    const container = cropperContainer.value
    const containerWidth = container.clientWidth
    const containerHeight = container.clientHeight
    
    // 计算图像在容器中的显示大小
    const imgRatio = img.naturalWidth / img.naturalHeight
    let displayWidth, displayHeight
    
    if (imgRatio > containerWidth / containerHeight) {
      displayWidth = containerWidth
      displayHeight = containerWidth / imgRatio
    } else {
      displayHeight = containerHeight
      displayWidth = containerHeight * imgRatio
    }
    
    // 初始化裁剪框为图像中心的正方形
    const size = Math.min(displayWidth, displayHeight) * 0.6
    const imgOffsetX = (containerWidth - displayWidth) / 2
    const imgOffsetY = (containerHeight - displayHeight) / 2
    
    cropBox.size = size
    cropBox.x = imgOffsetX + (displayWidth - size) / 2
    cropBox.y = imgOffsetY + (displayHeight - size) / 2
    
    updateCropPreview()
  }
}

// 处理裁剪容器鼠标事件
const handleCropperMouseDown = (e: MouseEvent) => {
  const target = e.target as HTMLElement
  
  // 检查是否点击了调整手柄
  if (target.classList.contains('resize-handle')) {
    dragMode = 'resize'
    activeHandle = target.dataset.handle || null
  } else if (target.closest('.crop-box')) {
    // 点击裁剪框内部进行拖动
    dragMode = 'move'
  } else {
    return
  }
  
  startMouseX = e.clientX
  startMouseY = e.clientY
  startCropX = cropBox.x
  startCropY = cropBox.y
  startCropWidth = cropBox.size
  startCropHeight = cropBox.size
  
  e.preventDefault()
}

const handleCropperMouseMove = (e: MouseEvent) => {
  if (!dragMode) return
  
  const deltaX = e.clientX - startMouseX
  const deltaY = e.clientY - startMouseY
  const container = cropperContainer.value
  if (!container) return
  
  const containerWidth = container.clientWidth
  const containerHeight = container.clientHeight
  const minSize = 20
  
  if (dragMode === 'move') {
    // 拖动裁剪框
    let newX = startCropX + deltaX
    let newY = startCropY + deltaY
    
    // 限制在容器内
    newX = Math.max(0, Math.min(newX, containerWidth - cropBox.size))
    newY = Math.max(0, Math.min(newY, containerHeight - cropBox.size))
    
    cropBox.x = newX
    cropBox.y = newY
  } else if (dragMode === 'resize' && activeHandle) {
    // 调整裁剪框大小（保持正方形）
    let newX = startCropX
    let newY = startCropY
    let newSize = startCropWidth
    
    // 使用较大的delta来调整大小
    const delta = Math.abs(deltaX) > Math.abs(deltaY) ? deltaX : deltaY
    
    // 根据手柄位置调整
    if (activeHandle === 'se') {
      // 右下角：增大
      newSize = Math.max(minSize, startCropWidth + delta)
    } else if (activeHandle === 'nw') {
      // 左上角：调整位置和大小
      newSize = Math.max(minSize, startCropWidth - delta)
      newX = startCropX + startCropWidth - newSize
      newY = startCropY + startCropHeight - newSize
    } else if (activeHandle === 'ne') {
      // 右上角
      const d = Math.abs(deltaX) > Math.abs(deltaY) ? deltaX : -deltaY
      newSize = Math.max(minSize, startCropWidth + d)
      newY = startCropY + startCropHeight - newSize
    } else if (activeHandle === 'sw') {
      // 左下角
      const d = Math.abs(deltaX) > Math.abs(deltaY) ? -deltaX : deltaY
      newSize = Math.max(minSize, startCropWidth + d)
      newX = startCropX + startCropWidth - newSize
    }
    
    // 限制在容器内
    if (newX < 0) {
      newSize += newX
      newX = 0
    }
    if (newY < 0) {
      newSize += newY
      newY = 0
    }
    if (newX + newSize > containerWidth) {
      newSize = containerWidth - newX
    }
    if (newY + newSize > containerHeight) {
      newSize = containerHeight - newY
    }
    
    newSize = Math.max(minSize, newSize)
    
    cropBox.x = newX
    cropBox.y = newY
    cropBox.size = newSize
  }
  
  updateCropPreview()
}

const handleCropperMouseUp = () => {
  dragMode = null
  activeHandle = null
}

watch(() => [cropBox.x, cropBox.y, cropBox.size], () => {
  updateCropPreview()
})

const updateCropPreview = () => {
  const canvas = document.querySelector('.preview-canvas') as HTMLCanvasElement | null
  if (!canvas || !cropperImg.value || !cropperContainer.value) return
  
  const ctx = canvas.getContext('2d')
  if (!ctx) return
  
  // 预览始终是192x192
  canvas.width = 192
  canvas.height = 192
  ctx.fillStyle = '#f0f0f2'
  ctx.fillRect(0, 0, 192, 192)
  
  const img = cropperImg.value
  const container = cropperContainer.value
  const containerWidth = container.clientWidth
  const containerHeight = container.clientHeight
  
  // 计算图像在容器中的显示大小和位置
  const imgRatio = img.naturalWidth / img.naturalHeight
  let displayWidth, displayHeight
  
  if (imgRatio > containerWidth / containerHeight) {
    displayWidth = containerWidth
    displayHeight = containerWidth / imgRatio
  } else {
    displayHeight = containerHeight
    displayWidth = containerHeight * imgRatio
  }
  
  const imgOffsetX = (containerWidth - displayWidth) / 2
  const imgOffsetY = (containerHeight - displayHeight) / 2
  
  // 计算裁剪框对应原始图像的区域
  const scaleX = img.naturalWidth / displayWidth
  const scaleY = img.naturalHeight / displayHeight
  
  // 裁剪框相对于图像的位置
  const cropRelX = cropBox.x - imgOffsetX
  const cropRelY = cropBox.y - imgOffsetY
  
  // 原始图像中的坐标（正方形）
  const srcX = Math.max(0, cropRelX * scaleX)
  const srcY = Math.max(0, cropRelY * scaleY)
  const srcSize = cropBox.size * scaleX  // 正方形，所以scaleX=scaleY
  
  // 确保不超出图像边界
  const clampedSrcX = Math.min(srcX, img.naturalWidth)
  const clampedSrcY = Math.min(srcY, img.naturalHeight)
  const clampedSrcSize = Math.min(srcSize, img.naturalWidth - clampedSrcX, img.naturalHeight - clampedSrcY)
  
  if (clampedSrcSize <= 0) return
  
  try {
    ctx.drawImage(
      img,
      clampedSrcX,
      clampedSrcY,
      clampedSrcSize,
      clampedSrcSize,
      0,
      0,
      192,
      192
    )
  } catch (e) {
    // Image not ready yet
  }
}

const confirmAvatarCrop = async () => {
  if (!selectedAvatarFile || !cropperImg.value || !cropperContainer.value) return
  
  try {
    const img = cropperImg.value
    const container = cropperContainer.value
    const containerWidth = container.clientWidth
    const containerHeight = container.clientHeight
    
    // 计算图像在容器中的显示大小和位置
    const imgRatio = img.naturalWidth / img.naturalHeight
    let displayWidth, displayHeight
    
    if (imgRatio > containerWidth / containerHeight) {
      displayWidth = containerWidth
      displayHeight = containerWidth / imgRatio
    } else {
      displayHeight = containerHeight
      displayWidth = containerHeight * imgRatio
    }
    
    const imgOffsetX = (containerWidth - displayWidth) / 2
    const imgOffsetY = (containerHeight - displayHeight) / 2
    
    // 计算裁剪框对应原始图像的区域
    const scaleX = img.naturalWidth / displayWidth
    
    // 裁剪框相对于图像的位置
    const cropRelX = cropBox.x - imgOffsetX
    const cropRelY = cropBox.y - imgOffsetY
    
    // 原始图像中的坐标（正方形）
    const srcX = Math.max(0, cropRelX * scaleX)
    const srcY = Math.max(0, cropRelY * scaleX)
    const srcSize = Math.round(cropBox.size * scaleX)
    
    // 确保不超出图像边界
    const clampedSrcX = Math.round(Math.min(srcX, img.naturalWidth))
    const clampedSrcY = Math.round(Math.min(srcY, img.naturalHeight))
    const clampedSrcSize = Math.round(Math.min(srcSize, img.naturalWidth - clampedSrcX, img.naturalHeight - clampedSrcY))
    
    // Create canvas and draw cropped image - 输出原始尺寸
    const canvas = document.createElement('canvas')
    const ctx = canvas.getContext('2d')
    if (!ctx) {
      throw new Error('Cannot create canvas context')
    }
    
    // 输出原始裁剪尺寸
    canvas.width = clampedSrcSize
    canvas.height = clampedSrcSize
    
    // Draw the cropped portion
    ctx.drawImage(
      img,
      clampedSrcX,
      clampedSrcY,
      clampedSrcSize,
      clampedSrcSize,
      0,
      0,
      clampedSrcSize,
      clampedSrcSize
    )
    
    // Convert to blob
    const blob = await new Promise<Blob | null>((resolve) => {
      canvas.toBlob(resolve, selectedAvatarFile!.type)
    })
    
    if (!blob) {
      throw new Error('Failed to create image blob')
    }
    
    const ext = selectedAvatarFile!.type.includes('image/svg')
      ? 'svg'
      : selectedAvatarFile!.type.includes('image/png')
      ? 'png'
      : selectedAvatarFile!.type.includes('image/webp')
      ? 'webp'
      : selectedAvatarFile!.type.includes('image/gif')
      ? 'gif'
      : 'jpg'
    
    const croppedFile = new File([blob], `avatar.${ext}`, { type: selectedAvatarFile!.type })
    
    // Upload the cropped avatar
    await uploadAvatar(croppedFile)
    
    // Refresh user data
    await authStore.fetchCurrentUser()
    
    // Update local profile state
    if (authStore.user?.avatar_url) {
      profile.avatar_url = authStore.user.avatar_url
    }
    
    message.value = '头像已更新'
    messageType.value = 'success'
    showAvatarCropper.value = false
    
    // Clear file input
    if (avatarFileInput.value) {
      avatarFileInput.value.value = ''
      avatarFileName.value = ''
    }
    selectedAvatarFile = null
  } catch (error: any) {
    message.value = error.message || '头像上传失败'
    messageType.value = 'error'
  }
}

const uploadAvatar = async (file: File): Promise<string> => {
  const formData = new FormData()
  formData.append('avatar', file)
  
  try {
    const response = await fetch('/api/v1/user/avatar', {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${authStore.token}`
      },
      body: formData
    })
    
    if (!response.ok) {
      const error = await response.json()
      throw new Error(error.message || '上传失败')
    }
    
    const data = await response.json()
    return data.avatar_url
  } catch (error: any) {
    throw error
  }
}

const saveProfile = async () => {
  saving.value = true
  message.value = ''

  try {
    // Update profile with status and other info
    await apiClient.client.put('/user/profile', {
      display_name: profile.display_name,
      status_emoji: profile.status_emoji,
      status_message: profile.status_message,
      busy: profile.busy,
      clear_status_after: profile.clear_status_after
    })

    message.value = '个人资料已更新'
    messageType.value = 'success'
    await authStore.fetchCurrentUser()
  } catch (error: any) {
    message.value = error.response?.data?.message || error.message || '保存失败'
    messageType.value = 'error'
  } finally {
    saving.value = false
  }
}

const resetForm = () => {
  loadProfile()
}

onMounted(() => {
  loadProfile()
})
</script>

<style lang="scss" scoped>
.profile-page {
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

.profile-section {
  margin-bottom: 32px;
  
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
    
    a {
      color: #1f75cb;
      text-decoration: none;
      
      &:hover {
        text-decoration: underline;
      }
    }
  }
}

.avatar-upload {
  display: flex;
  gap: 24px;
  align-items: flex-start;
}

.avatar-preview {
  width: 96px;
  height: 96px;
  border-radius: 50%;
  background: #f0f0f2;
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
  flex-shrink: 0;
  
  img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  
  .avatar-placeholder {
    color: #868686;
  }
}

.avatar-actions {
  h3 {
    font-size: 14px;
    font-weight: 600;
    color: #303030;
    margin: 0 0 12px 0;
  }
  
  .upload-hint {
    font-size: 12px;
    color: #737278;
    margin: 8px 0 0 0;
  }
}

.file-input-wrapper {
  display: flex;
  align-items: center;
  gap: 12px;
  
  .file-input-btn {
    display: inline-flex;
    align-items: center;
    padding: 8px 16px;
    font-size: 14px;
    color: #303030;
    background: #fff;
    border: 1px solid #dcdcde;
    border-radius: 4px;
    cursor: pointer;
    
    &:hover {
      background: #f0f0f2;
    }
    
    input {
      display: none;
    }
  }
  
  .file-name {
    font-size: 14px;
    color: #737278;
  }
}

.status-input-wrapper {
  display: flex;
  align-items: center;
  border: 1px solid #dcdcde;
  border-radius: 4px;
  background: #fff;
  
  &:focus-within {
    border-color: #1f75cb;
    box-shadow: 0 0 0 3px rgba(31, 117, 203, 0.15);
  }
}

.emoji-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 40px;
  height: 40px;
  background: none;
  border: none;
  cursor: pointer;
  color: #737278;
  
  &:hover {
    color: #303030;
  }
}

.status-input {
  flex: 1;
  padding: 10px 0;
  font-size: 14px;
  color: #303030;
  background: none;
  border: none;
  
  &:focus {
    outline: none;
  }
  
  &::placeholder {
    color: #737278;
  }
}

.clear-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 40px;
  height: 40px;
  background: none;
  border: none;
  cursor: pointer;
  color: #737278;
  
  &:hover {
    color: #303030;
  }
}

.char-count {
  font-size: 12px;
  color: #737278;
  margin: 8px 0 16px 0;
}

.busy-checkbox {
  margin-bottom: 16px;
  
  .checkbox-label {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    cursor: pointer;
    
    input[type="checkbox"] {
      display: none;
    }
    
    .checkbox-custom {
      width: 16px;
      height: 16px;
      border: 2px solid #868686;
      border-radius: 3px;
      flex-shrink: 0;
      margin-top: 2px;
      position: relative;
      
      &::after {
        content: '';
        position: absolute;
        top: 1px;
        left: 4px;
        width: 4px;
        height: 8px;
        border: solid #fff;
        border-width: 0 2px 2px 0;
        transform: rotate(45deg);
        opacity: 0;
      }
    }
    
    input[type="checkbox"]:checked + .checkbox-custom {
      background: #1f75cb;
      border-color: #1f75cb;
      
      &::after {
        opacity: 1;
      }
    }
    
    .checkbox-text {
      display: flex;
      flex-direction: column;
      gap: 2px;
    }
    
    .checkbox-title {
      font-size: 14px;
      color: #303030;
    }
    
    .checkbox-desc {
      font-size: 12px;
      color: #737278;
    }
  }
}

.clear-status {
  display: flex;
  flex-direction: column;
  gap: 8px;
  
  label {
    font-size: 14px;
    font-weight: 600;
    color: #303030;
  }
}

.form-select {
  width: 200px;
  padding: 8px 12px;
  font-size: 14px;
  color: #303030;
  background: #fff;
  border: 1px solid #dcdcde;
  border-radius: 4px;
  cursor: pointer;
  
  &:focus {
    outline: none;
    border-color: #1f75cb;
    box-shadow: 0 0 0 3px rgba(31, 117, 203, 0.15);
  }
}

.divider {
  border: none;
  border-top: 1px solid #dcdcde;
  margin: 32px 0;
}

.form-actions {
  display: flex;
  gap: 12px;
}

.btn {
  padding: 10px 16px;
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
  
  &.btn-secondary {
    background: #fff;
    color: #303030;
    border: 1px solid #dcdcde;
    
    &:hover {
      background: #f0f0f2;
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

.avatar-cropper-modal {
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
  
  .modal-content {
    background: #fff;
    border-radius: 8px;
    box-shadow: 0 10px 40px rgba(0, 0, 0, 0.2);
    max-width: 600px;
    width: 90%;
    max-height: 90vh;
    display: flex;
    flex-direction: column;
  }
  
  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 24px;
    border-bottom: 1px solid #dcdcde;
    
    h2 {
      margin: 0;
      font-size: 18px;
      font-weight: 600;
    }
    
    .close-btn {
      background: none;
      border: none;
      cursor: pointer;
      color: #737278;
      padding: 8px;
      display: flex;
      align-items: center;
      
      &:hover {
        color: #303030;
      }
    }
  }
  
  .modal-body {
    flex: 1;
    overflow-y: auto;
    padding: 24px;
    
    .cropper-container-wrapper {
      display: flex;
      gap: 24px;
      align-items: flex-start;
    }
    
    .cropper-container {
      position: relative;
      width: 400px;
      height: 400px;
      background: #f0f0f2;
      border-radius: 4px;
      overflow: hidden;
      flex-shrink: 0;
      
      .cropper-image {
        position: absolute;
        max-width: 100%;
        max-height: 100%;
        top: 50%;
        left: 50%;
        transform: translate(-50%, -50%);
        user-select: none;
        pointer-events: none;
      }
      
      .crop-overlay-svg {
        position: absolute;
        top: 0;
        left: 0;
        width: 100%;
        height: 100%;
        pointer-events: none;
      }
      
      .crop-box {
        position: absolute;
        border: 2px dashed #1f75cb;
        box-sizing: border-box;
        cursor: move;
        pointer-events: auto;
        z-index: 10;
        
        .resize-handle {
          position: absolute;
          width: 12px;
          height: 12px;
          background: #fff;
          border: 2px solid #1f75cb;
          border-radius: 2px;
          pointer-events: auto;
          
          &.nw { top: -6px; left: -6px; cursor: nwse-resize; }
          &.ne { top: -6px; right: -6px; cursor: nesw-resize; }
          &.sw { bottom: -6px; left: -6px; cursor: nesw-resize; }
          &.se { bottom: -6px; right: -6px; cursor: nwse-resize; }
        }
      }
    }
    
    .crop-preview {
      display: flex;
      flex-direction: column;
      align-items: center;
      gap: 12px;
      flex-shrink: 0;
      
      .preview-label {
        font-size: 12px;
        color: #737278;
        font-weight: 500;
      }
      
      .preview-canvas {
        width: 192px;
        height: 192px;
        border: 1px solid #dcdcde;
        border-radius: 4px;
        background: #f9f9fb;
      }
      
      .crop-size {
        font-size: 12px;
        color: #737278;
      }
    }
  }
  
  .modal-footer {
    padding: 16px 24px;
    border-top: 1px solid #dcdcde;
    display: flex;
    gap: 12px;
    justify-content: flex-end;
  }
}
</style>
