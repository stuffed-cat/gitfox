<template>
  <span 
    v-if="verification" 
    class="gpg-badge"
    :class="badgeClass"
    :title="badgeTitle"
  >
    <!-- Verified: Shield with check -->
    <svg v-if="badgeIconType === 'verified'" class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
      <path stroke-linecap="round" stroke-linejoin="round" d="M9 12.75 11.25 15 15 9.75m-3-7.036A11.959 11.959 0 0 1 3.598 6 11.99 11.99 0 0 0 3 9.749c0 5.592 3.824 10.29 9 11.623 5.176-1.332 9-6.03 9-11.622 0-1.31-.21-2.571-.598-3.751h-.152c-3.196 0-6.1-1.248-8.25-3.285Z" />
    </svg>
    <!-- Unverified: Check circle -->
    <svg v-else-if="badgeIconType === 'unverified'" class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
      <path stroke-linecap="round" stroke-linejoin="round" d="M9 12.75 11.25 15 15 9.75M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0Z" />
    </svg>
    <!-- Invalid: Shield with exclamation -->
    <svg v-else-if="badgeIconType === 'invalid'" class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
      <path stroke-linecap="round" stroke-linejoin="round" d="M12 9v3.75m0-10.036A11.959 11.959 0 0 1 3.598 6 11.99 11.99 0 0 0 3 9.75c0 5.592 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.31-.21-2.57-.598-3.75h-.152c-3.196 0-6.1-1.25-8.25-3.286Zm0 13.036h.008v.008H12v-.008Z" />
    </svg>
    <!-- Unknown: Question mark circle -->
    <svg v-else class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
      <path stroke-linecap="round" stroke-linejoin="round" d="M9.879 7.519c1.171-1.025 3.071-1.025 4.242 0 1.172 1.025 1.172 2.687 0 3.712-.203.179-.43.326-.67.442-.745.361-1.45.999-1.45 1.827v.75M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0Zm-9 5.25h.008v.008H12v-.008Z" />
    </svg>
    <span v-if="showLabel" class="label">{{ badgeLabel }}</span>
  </span>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import type { GpgVerificationInfo } from '@/types'

const props = defineProps<{
  verification?: GpgVerificationInfo
  showLabel?: boolean
}>()

const badgeClass = computed(() => {
  if (!props.verification) return ''
  
  switch (props.verification.status) {
    case 'verified':
      return 'verified'
    case 'unverified':
      return 'unverified'
    case 'bad_email':
    case 'bad_signature':
    case 'expired_key':
    case 'revoked_key':
      return 'invalid'
    case 'unknown_key':
    case 'no_signature':
    default:
      return 'unknown'
  }
})

const badgeIconType = computed(() => {
  if (!props.verification) return 'unknown'
  
  switch (props.verification.status) {
    case 'verified':
      return 'verified'
    case 'unverified':
      return 'unverified'
    case 'bad_email':
    case 'bad_signature':
    case 'expired_key':
    case 'revoked_key':
      return 'invalid'
    case 'unknown_key':
    case 'no_signature':
    default:
      return 'unknown'
  }
})

const badgeLabel = computed(() => {
  if (!props.verification) return '未签名'
  
  switch (props.verification.status) {
    case 'verified':
      return '已验证'
    case 'unverified':
      return '未验证'
    case 'bad_email':
      return '邮箱不匹配'
    case 'bad_signature':
      return '无效签名'
    case 'expired_key':
      return '密钥已过期'
    case 'revoked_key':
      return '密钥已撤销'
    case 'unknown_key':
      return '未知密钥'
    case 'no_signature':
    default:
      return '未签名'
  }
})

const badgeTitle = computed(() => {
  if (!props.verification) return '此提交没有 GPG 签名'
  
  let title = badgeLabel.value
  
  if (props.verification.key_id) {
    title += `\n密钥 ID: ${props.verification.key_id}`
  }
  
  if (props.verification.signer_username) {
    title += `\n签名者: ${props.verification.signer_username}`
  }
  
  if (props.verification.message) {
    title += `\n${props.verification.message}`
  }
  
  return title
})
</script>

<style scoped lang="scss">
.gpg-badge {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 2px 8px;
  border-radius: 4px;
  font-size: 12px;
  font-weight: 500;
  cursor: help;
  
  .icon {
    width: 16px;
    height: 16px;
  }
  
  &.verified {
    background-color: var(--color-success-subtle, #dcfce7);
    color: var(--color-success, #16a34a);
    border: 1px solid var(--color-success-border, #22c55e);
  }
  
  &.unverified {
    background-color: var(--color-warning-subtle, #fef3c7);
    color: var(--color-warning, #d97706);
    border: 1px solid var(--color-warning-border, #f59e0b);
  }
  
  &.invalid {
    background-color: var(--color-danger-subtle, #fee2e2);
    color: var(--color-danger, #dc2626);
    border: 1px solid var(--color-danger-border, #ef4444);
  }
  
  &.unknown {
    background-color: var(--color-muted-subtle, #f3f4f6);
    color: var(--color-muted, #6b7280);
    border: 1px solid var(--color-border, #d1d5db);
  }
}
</style>