/**
 * 项目导航菜单配置
 */

import type { NavSection, NavContext } from '../types'

/**
 * 生成项目菜单配置
 */
export function createProjectMenuConfig(context: NavContext): NavSection[] {
  const basePath = context.project?.path || ''
  
  return [
    {
      id: 'project-main',
      title: '项目',
      items: [
        { 
          id: 'project-overview', 
          label: '项目概览', 
          to: basePath, 
          icon: 'project',
          activeMatch: (path) => path === basePath || path === `${basePath}/`
        },
      ]
    },
    {
      id: 'code',
      title: '代码',
      items: [
        { 
          id: 'files', 
          label: '文件', 
          to: `${basePath}/-/tree`, 
          icon: 'code',
          activeMatch: /\/-\/(tree|blob)/
        },
        { 
          id: 'commits', 
          label: '提交', 
          to: `${basePath}/-/commits`, 
          icon: 'commit',
          activeMatch: /\/-\/commit/
        },
        { 
          id: 'branches', 
          label: '分支', 
          to: `${basePath}/-/branches`, 
          icon: 'branch' 
        },
        { 
          id: 'tags', 
          label: '标签', 
          to: `${basePath}/-/tags`, 
          icon: 'tag' 
        },
      ]
    },
    {
      id: 'plan',
      title: '计划',
      items: [
        { 
          id: 'project-issues', 
          label: '议题', 
          to: `${basePath}/-/issues`, 
          icon: 'issue',
          activeMatch: /\/-\/issues/
        },
        { 
          id: 'project-mr', 
          label: '合并请求', 
          to: `${basePath}/-/merge_requests`, 
          icon: 'mergeRequest',
          activeMatch: /\/-\/merge_requests/
        },
      ]
    },
    {
      id: 'build',
      title: '构建',
      items: [
        { 
          id: 'pipelines', 
          label: '流水线', 
          to: `${basePath}/-/pipelines`, 
          icon: 'pipeline',
          activeMatch: /\/-\/pipelines/
        },
        { 
          id: 'runners', 
          label: 'Runners', 
          to: `${basePath}/-/runners`, 
          icon: 'pipeline',
          activeMatch: /\/-\/runners/
        },
      ]
    },
    {
      id: 'project-settings',
      title: '设置',
      items: [
        { 
          id: 'settings', 
          label: '通用', 
          to: `${basePath}/-/settings`, 
          icon: 'settings' 
        },
      ]
    }
  ]
}
