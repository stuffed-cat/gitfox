/**
 * 群组导航菜单配置
 */

import type { NavSection, NavContext } from '../types'

/**
 * 生成群组菜单配置
 */
export function createGroupMenuConfig(context: NavContext): NavSection[] {
  const basePath = context.group?.path ? `/${context.group.path}` : ''
  
  return [
    {
      id: 'group-main',
      title: '群组',
      items: [
        { 
          id: 'group-overview', 
          label: '群组概览', 
          to: basePath, 
          icon: 'group',
          activeMatch: (path) => path === basePath || path === `${basePath}/`
        },
      ]
    },
    {
      id: 'group-plan',
      title: '计划',
      items: [
        { 
          id: 'group-issues', 
          label: '议题', 
          to: `${basePath}/-/issues`, 
          icon: 'issue',
          activeMatch: /\/-\/issues/
        },
        { 
          id: 'group-mr', 
          label: '合并请求', 
          to: `${basePath}/-/merge_requests`, 
          icon: 'mergeRequest',
          activeMatch: /\/-\/merge_requests/
        },
      ]
    },
    {
      id: 'group-manage',
      title: '管理',
      items: [
        { 
          id: 'group-members', 
          label: '成员', 
          to: `${basePath}/-/members`, 
          icon: 'users',
          activeMatch: /\/-\/members/
        },
      ]
    },
    {
      id: 'group-settings',
      title: '设置',
      items: [
        { 
          id: 'group-general-settings', 
          label: '通用', 
          to: `${basePath}/-/settings`, 
          icon: 'settings',
          activeMatch: /\/-\/settings/
        },
      ]
    },
  ]
}
