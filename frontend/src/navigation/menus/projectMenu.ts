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
          id: 'jobs', 
          label: '作业', 
          to: `${basePath}/-/jobs`, 
          icon: 'job',
          activeMatch: /\/-\/jobs/
        },
        { 
          id: 'ci-editor', 
          label: '流水线编辑器', 
          to: `${basePath}/-/ci/editor`, 
          icon: 'edit',
          activeMatch: /\/-\/ci\/editor/
        },
        { 
          id: 'pipeline-schedules', 
          label: '流水线计划', 
          to: `${basePath}/-/pipeline_schedules`, 
          icon: 'clock',
          activeMatch: /\/-\/pipeline_schedules/
        },
        { 
          id: 'artifacts', 
          label: '产物', 
          to: `${basePath}/-/artifacts`, 
          icon: 'download',
          activeMatch: /\/-\/artifacts/
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
        { 
          id: 'runners', 
          label: 'CI/CD Runners', 
          to: `${basePath}/-/runners`, 
          icon: 'pipeline',
          activeMatch: /\/-\/runners/
        },
      ]
    }
  ]
}
