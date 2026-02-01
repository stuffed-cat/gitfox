/**
 * 导航菜单项类型定义
 */

/**
 * 菜单项
 */
export interface NavItem {
  /** 唯一标识 */
  id: string
  /** 显示标签 */
  label: string
  /** 路由路径（可以是函数，接收上下文参数） */
  to: string | ((context: NavContext) => string)
  /** 图标名称 */
  icon: string
  /** 徽章数量 */
  badge?: number | string
  /** 徽章类型 */
  badgeType?: 'default' | 'warning' | 'danger' | 'success'
  /** 路由匹配规则（用于高亮判断） */
  activeMatch?: RegExp | ((path: string, context: NavContext) => boolean)
  /** 是否需要特定权限 */
  permissions?: string[]
  /** 子菜单项 */
  children?: NavItem[]
}

/**
 * 菜单分区
 */
export interface NavSection {
  /** 唯一标识 */
  id: string
  /** 分区标题 */
  title: string
  /** 菜单项 */
  items: NavItem[]
}

/**
 * 导航上下文
 */
export interface NavContext {
  /** 上下文类型 */
  type: 'global' | 'project' | 'group' | 'user-settings' | 'admin'
  /** 项目信息 */
  project?: {
    owner: string
    name: string
    path: string
  }
  /** 群组信息 */
  group?: {
    path: string
  }
  /** 用户信息 */
  user?: {
    username: string
  }
}

/**
 * 上下文头部信息
 */
export interface ContextHeader {
  /** 头像文字或图标 */
  avatar: string
  /** 标题 */
  title: string
  /** 副标题 */
  subtitle?: string
  /** 链接 */
  to: string
}
