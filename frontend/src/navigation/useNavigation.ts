/**
 * 导航系统 Composable
 * 根据当前路由自动返回正确的菜单配置
 */

import { computed, type ComputedRef } from 'vue'
import { useRoute } from 'vue-router'
import { useAuthStore } from '@/stores/auth'
import { useNamespaceStore } from '@/stores/namespace'
import type { NavContext, NavSection, NavItem, ContextHeader } from './types'
import { globalMenuConfig, guestMenuConfig } from './menus/globalMenu'
import { createProjectMenuConfig } from './menus/projectMenu'
import { createGroupMenuConfig } from './menus/groupMenu'
import { userSettingsMenuConfig } from './menus/userSettingsMenu'

export interface UseNavigationReturn {
  /** 当前导航上下文 */
  context: ComputedRef<NavContext>
  /** 当前菜单分区列表 */
  sections: ComputedRef<NavSection[]>
  /** 上下文头部信息（项目、群组等） */
  contextHeader: ComputedRef<ContextHeader | null>
  /** 检查菜单项是否激活 */
  isActive: (item: NavItem) => boolean
}

/**
 * 导航系统 Composable
 */
export function useNavigation(): UseNavigationReturn {
  const route = useRoute()
  const authStore = useAuthStore()
  const namespaceStore = useNamespaceStore()
  
  // 解析当前导航上下文
  const context = computed<NavContext>(() => {
    const path = route.path
    
    // 用户设置页面
    if (path.startsWith('/-/profile')) {
      return {
        type: 'user-settings',
        user: authStore.user ? {
          username: authStore.user.username
        } : undefined
      }
    }
    
    // 群组页面 - 检查路由 meta 或 namespace store
    if (route.meta?.contextType === 'group' || namespaceStore.currentNamespaceType === 'group') {
      const groupPath = (route.params.namespace || route.params.groupPath || namespaceStore.currentNamespace) as string
      return {
        type: 'group',
        group: {
          path: groupPath
        }
      }
    }
    
    // 项目页面 - 检查路由参数
    const { owner, repo } = route.params
    if (owner && repo && typeof owner === 'string' && typeof repo === 'string') {
      return {
        type: 'project',
        project: {
          owner,
          name: repo,
          path: `/${owner}/${repo}`
        }
      }
    }
    
    // 默认全局上下文
    return { type: 'global' }
  })
  
  // 根据上下文生成菜单
  const sections = computed<NavSection[]>(() => {
    // 未登录用户在全局上下文使用访客菜单
    const isAuthenticated = authStore.isAuthenticated
    
    switch (context.value.type) {
      case 'project':
        return createProjectMenuConfig(context.value)
      case 'group':
        return createGroupMenuConfig(context.value)
      case 'user-settings':
        return userSettingsMenuConfig
      default:
        // 未登录用户显示访客菜单
        return isAuthenticated ? globalMenuConfig : guestMenuConfig
    }
  })
  
  // 上下文头部信息
  const contextHeader = computed<ContextHeader | null>(() => {
    const ctx = context.value
    
    if (ctx.type === 'project' && ctx.project) {
      return {
        avatar: ctx.project.name.charAt(0).toUpperCase(),
        title: ctx.project.name,
        subtitle: ctx.project.owner,
        to: ctx.project.path
      }
    }
    
    if (ctx.type === 'group' && ctx.group) {
      const groupName = ctx.group.path.split('/').pop() || ctx.group.path
      return {
        avatar: groupName.charAt(0).toUpperCase(),
        title: groupName,
        subtitle: ctx.group.path,
        to: `/${ctx.group.path}`
      }
    }
    
    if (ctx.type === 'user-settings' && ctx.user) {
      return {
        avatar: ctx.user.username.charAt(0).toUpperCase(),
        title: '用户设置',
        subtitle: ctx.user.username,
        to: '/-/profile'
      }
    }
    
    return null
  })
  
  // 检查菜单项是否激活
  function isActive(item: NavItem): boolean {
    const currentPath = route.path
    const itemTo = typeof item.to === 'function' ? item.to(context.value) : item.to
    
    // 使用自定义匹配规则
    if (item.activeMatch) {
      if (typeof item.activeMatch === 'function') {
        return item.activeMatch(currentPath, context.value)
      }
      return item.activeMatch.test(currentPath)
    }
    
    // 精确匹配
    if (itemTo === currentPath) return true
    
    // 前缀匹配（非首页）
    if (itemTo !== '/' && currentPath.startsWith(itemTo + '/')) return true
    if (itemTo !== '/' && currentPath.startsWith(itemTo)) return true
    
    return false
  }
  
  return {
    context,
    sections,
    contextHeader,
    isActive
  }
}
