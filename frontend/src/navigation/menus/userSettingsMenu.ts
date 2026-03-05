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
      { 
        id: 'gpg-keys', 
        label: 'GPG 密钥', 
        to: '/-/profile/gpg-keys', 
        icon: 'key' 
      },
      { 
        id: 'access-tokens', 
        label: '访问令牌', 
        to: '/-/profile/tokens', 
        icon: 'token',
        activeMatch: /^\/-\/profile\/tokens/
      },
      { 
        id: 'two-factor', 
        label: '双因素认证', 
        to: '/-/profile/two-factor', 
        icon: 'shield',
        activeMatch: /^\/-\/profile\/two-factor/
      },
      { 
        id: 'oauth-accounts', 
        label: '已关联账号', 
        to: '/-/profile/accounts', 
        icon: 'oauth',
        activeMatch: /^\/-\/profile\/accounts/
      },
    ]
  },
  {
    id: 'applications',
    title: '应用',
    items: [
      { 
        id: 'oauth-applications', 
        label: 'OAuth 应用', 
        to: '/-/profile/applications', 
        icon: 'oauth',
        activeMatch: /^\/-\/profile\/applications/
      },
      { 
        id: 'runners', 
        label: '私有 Runners', 
        to: '/-/profile/runners', 
        icon: 'pipeline',
        activeMatch: /^\/-\/profile\/runners/
      },
    ]
  }
]
