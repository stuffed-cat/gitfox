/**
 * 导航图标集合
 * 所有图标使用 16x16 的 SVG path
 */

export const navIcons: Record<string, string> = {
  // 通用
  home: 'M8 1L1 6v8a1 1 0 001 1h4v-5h4v5h4a1 1 0 001-1V6L8 1z',
  search: 'M7 7m-5 0a5 5 0 1010 0a5 5 0 10-10 0M11 11l3 3',
  settings: 'M8 8m-2 0a2 2 0 104 0a2 2 0 10-4 0M8 1v2M8 13v2M1 8h2M13 8h2M3 3l1.5 1.5M11.5 11.5l1.5 1.5M3 13l1.5-1.5M11.5 4.5l1.5-1.5',
  dashboard: 'M2 2h5v6H2zM9 2h5v4H9zM9 8h5v6H9zM2 10h5v4H2z',
  
  // 项目相关
  project: 'M2 3a1 1 0 011-1h10a1 1 0 011 1v10a1 1 0 01-1 1H3a1 1 0 01-1-1V3zM5 6h6M5 9h4',
  folder: 'M2 4a1 1 0 011-1h3l2 2h5a1 1 0 011 1v7a1 1 0 01-1 1H3a1 1 0 01-1-1V4z',
  code: 'M5 4L1 8l4 4M11 4l4 4-4 4M9 2l-2 12',
  file: 'M4 2a1 1 0 00-1 1v10a1 1 0 001 1h8a1 1 0 001-1V6l-4-4H4zM9 2v4h4',
  
  // Git 相关
  commit: 'M8 8m-3 0a3 3 0 106 0a3 3 0 10-6 0M1 8h4M11 8h4',
  branch: 'M4 2v8a2 2 0 002 2h2M12 2v12M4 6h4a2 2 0 012 2v4',
  tag: 'M1 3l6-1 8 8-5 5-8-8zM5 5m-1 0a1 1 0 102 0a1 1 0 10-2 0',
  mergeRequest: 'M4 4m-2 0a2 2 0 104 0a2 2 0 10-4 0M12 12m-2 0a2 2 0 104 0a2 2 0 10-4 0M4 6v4a2 2 0 002 2h4M12 4v4',
  
  // 计划相关
  issue: 'M8 8m-6 0a6 6 0 1012 0a6 6 0 10-12 0M8 5v3M8 10v.5',
  issueOpen: 'M8 8m-6 0a6 6 0 1012 0a6 6 0 10-12 0',
  issueClosed: 'M8 8m-6 0a6 6 0 1012 0a6 6 0 10-12 0M5 8l2 2 4-4',
  milestone: 'M3 2h10v3l-3 3 3 3v3H3v-3l3-3-3-3V2z',
  
  // CI/CD
  pipeline: 'M2 4h4v4H2zM10 4h4v4h-4zM6 6h4M2 10h4v4H2zM10 10h4v4h-4zM6 12h4',
  job: 'M3 3h10v10H3zM6 6h4v4H6z',
  
  // Pipeline/Job 状态图标
  statusPending: 'M8 8m-6 0a6 6 0 1012 0a6 6 0 10-12 0',
  statusRunning: 'M8 8m-6 0a6 6 0 1012 0a6 6 0 10-12 0M6 5v6l5-3z',
  statusSuccess: 'M8 8m-6 0a6 6 0 1012 0a6 6 0 10-12 0M5 8l2 2 4-4',
  statusFailed: 'M8 8m-6 0a6 6 0 1012 0a6 6 0 10-12 0M6 6l4 4M10 6l-4 4',
  statusCanceled: 'M8 8m-6 0a6 6 0 1012 0a6 6 0 10-12 0M5 8h6',
  statusSkipped: 'M8 8m-6 0a6 6 0 1012 0a6 6 0 10-12 0M6 5l3 3-3 3M9 5l3 3-3 3',
  statusBlocked: 'M8 8m-6 0a6 6 0 1012 0a6 6 0 10-12 0M8 5v2M8 9v2',
  statusWarning: 'M8 2L1 14h14L8 2zM8 6v4M8 11v1',
  play: 'M4 3v10l9-5z',
  
  // 用户相关
  user: 'M8 5m-3 0a3 3 0 106 0a3 3 0 10-6 0M2 14a6 6 0 0112 0',
  users: 'M5 5m-2 0a2 2 0 104 0a2 2 0 10-4 0M11 5m-2 0a2 2 0 104 0a2 2 0 10-4 0M1 12a4 4 0 018 0M7 12a4 4 0 018 0',
  group: 'M1 3h6v6H1zM9 3h6v6H9zM5 9h6v6H5z',
  
  // 活动
  activity: 'M1 8h3l2-5 2 10 2-5h5',
  todo: 'M3 4h10M3 8h10M3 12h6',
  notification: 'M8 2a4 4 0 014 4v3l2 2H2l2-2V6a4 4 0 014-4zM6 13a2 2 0 104 0',
  
  // 安全
  key: 'M14 2l-1 1m-4 4a3 3 0 11-4.243 4.243A3 3 0 019 7zm0 0L11 5m0 0l2 2 2-2-2-2m-2 2l1-1',
  shield: 'M8 1L2 4v4c0 4.5 2.5 7.5 6 9 3.5-1.5 6-4.5 6-9V4L8 1z',
  lock: 'M4 7V5a4 4 0 018 0v2M3 7h10v7H3V7z',
  
  // 其他
  clock: 'M8 8m-6 0a6 6 0 1012 0a6 6 0 10-12 0M8 4v4l2 2',
  link: 'M6 8h4M7 5H4a3 3 0 000 6h3M9 5h3a3 3 0 010 6H9',
  externalLink: 'M11 3h2v2M13 3L7 9M5 5H3v8h8v-2',
  plus: 'M8 3v10M3 8h10',
  edit: 'M11 2l3 3-8 8H3v-3l8-8zM9 4l3 3',
  trash: 'M3 4h10M6 4V2h4v2M5 4v9h6V4',
  download: 'M8 2v8m-3-3l3 3 3-3M3 12h10',
  upload: 'M8 10V2m-3 3l3-3 3 3M3 12h10',
  check: 'M3 8l4 4 6-8',
  x: 'M4 4l8 8M12 4l-8 8',
  chevronDown: 'M4 6l4 4 4-4',
  chevronRight: 'M6 4l4 4-4 4',
  
  // 偏好设置
  preferences: 'M1 4h4m2 0h8M1 8h8m2 0h4M1 12h2m2 0h10M5 3v2M11 7v2M3 11v2',
  palette: 'M8 2a6 6 0 100 12 2 2 0 01-2-2c0-.5.2-1 .5-1.5S7 9.7 7 9a1 1 0 00-1-1H4a2 2 0 01-2-2 6 6 0 016-4zm0 3m-.5 0a.5.5 0 101 0 .5.5 0 10-1 0m2 0m-.5 0a.5.5 0 101 0 .5.5 0 10-1 0m2 2m-.5 0a.5.5 0 101 0 .5.5 0 10-1 0',
  language: 'M2 2h12v12H2zM2 6h12M6 2v4M5 9l2 3 2-3m-5 0h6',
  
  // OAuth 和访问令牌
  oauth: 'M8 1a7 7 0 100 14A7 7 0 008 1zM8 4a1 1 0 110 2 1 1 0 010-2zm-2.5 6.5a2.5 2.5 0 015 0M4 8h1m6 0h1',
  token: 'M12 1L8 5 4 1M8 5v6m-4 2h8a2 2 0 012 2v0H2v0a2 2 0 012-2z',
}
