/**
 * 全局导航菜单配置
 */

import type { NavSection } from '../types'

/** 访客菜单（未登录用户） */
export const guestMenuConfig: NavSection[] = [
  {
    id: 'explore',
    title: '探索',
    items: [
      { id: 'explore-projects', label: '项目', to: '/explore/projects', icon: 'search' },
      { id: 'explore-groups', label: '群组', to: '/explore/groups', icon: 'users' },
    ]
  }
]

/** 已登录用户的全局菜单 */
export const globalMenuConfig: NavSection[] = [
  {
    id: 'your-work',
    title: '你的工作',
    items: [
      { id: 'home', label: '首页', to: '/', icon: 'home' },
      { id: 'projects', label: '项目', to: '/dashboard/projects', icon: 'project' },
      { id: 'groups', label: '群组', to: '/dashboard/groups', icon: 'group' },
      { 
        id: 'issues', 
        label: '议题', 
        to: '/dashboard/issues', 
        icon: 'issue',
        activeMatch: /^\/dashboard\/issues/
      },
      { 
        id: 'merge-requests', 
        label: '合并请求', 
        to: '/dashboard/merge-requests', 
        icon: 'mergeRequest',
        activeMatch: /^\/dashboard\/merge-requests/
      },
      { 
        id: 'todos', 
        label: '待办事项', 
        to: '/dashboard/todos', 
        icon: 'todo'
      },
      { id: 'activity', label: '动态', to: '/dashboard/activity', icon: 'activity' },
    ]
  },
  {
    id: 'explore',
    title: '探索',
    items: [
      { id: 'explore-projects', label: '项目', to: '/explore/projects', icon: 'search' },
      { id: 'explore-groups', label: '群组', to: '/explore/groups', icon: 'users' },
    ]
  }
]

/** 管理员在全局菜单中额外显示的管理入口 */
export const adminEntrySection: NavSection = {
  id: 'admin',
  title: '管理',
  items: [
    { 
      id: 'admin-panel', 
      label: '管理面板', 
      to: '/admin', 
      icon: 'shield',
      activeMatch: /^\/admin/
    },
  ]
}
