/**
 * 用户设置导航菜单配置
 */

import type { NavSection } from '../types'

export const userSettingsMenuConfig: NavSection[] = [
  {
    id: 'user-settings',
    title: '用户设置',
    items: [
      { 
        id: 'profile', 
        label: '个人资料', 
        to: '/-/profile', 
        icon: 'user',
        activeMatch: (path) => path === '/-/profile'
      },
      { 
        id: 'preferences', 
        label: '偏好设置', 
        to: '/-/profile/preferences', 
        icon: 'preferences' 
      },
    ]
  },
  {
    id: 'access',
    title: '访问',
    items: [
      { 
        id: 'ssh-keys', 
        label: 'SSH 密钥', 
        to: '/-/profile/keys', 
        icon: 'key' 
      },
    ]
  }
]
