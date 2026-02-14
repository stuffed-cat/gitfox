/**
 * 管理员面板导航菜单配置
 */

import type { NavSection } from '../types'

export const adminMenuConfig: NavSection[] = [
  {
    id: 'admin-overview',
    title: '管理区域',
    items: [
      { 
        id: 'admin-dashboard', 
        label: '仪表盘', 
        to: '/admin', 
        icon: 'dashboard',
        activeMatch: (path) => path === '/admin'
      },
    ]
  },
  {
    id: 'admin-manage',
    title: '管理',
    items: [
      { 
        id: 'admin-users', 
        label: '用户管理', 
        to: '/admin/users', 
        icon: 'users',
        activeMatch: /^\/admin\/users/
      },
      { 
        id: 'admin-projects', 
        label: '项目管理', 
        to: '/admin/projects', 
        icon: 'project',
        activeMatch: /^\/admin\/projects/
      },
      { 
        id: 'admin-groups', 
        label: '群组管理', 
        to: '/admin/groups', 
        icon: 'group',
        activeMatch: /^\/admin\/groups/
      },
    ]
  },
  {
    id: 'admin-settings',
    title: '设置',
    items: [
      { 
        id: 'admin-settings-general', 
        label: '常规设置', 
        to: '/admin/settings', 
        icon: 'settings',
        activeMatch: /^\/admin\/settings/
      },
    ]
  }
]
