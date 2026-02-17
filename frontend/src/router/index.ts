import { createRouter, createWebHistory } from 'vue-router'
import { useAuthStore } from '@/stores/auth'

const routes = [
  // Auth routes
  {
    path: '/login',
    name: 'Login',
    component: () => import('@/views/auth/LoginView.vue'),
    meta: { guest: true }
  },
  {
    path: '/register',
    name: 'Register',
    component: () => import('@/views/auth/RegisterView.vue'),
    meta: { guest: true }
  },
  {
    path: '/oauth/callback',
    name: 'OAuthCallback',
    component: () => import('@/views/auth/OAuthCallbackView.vue'),
    meta: { guest: true }
  },
  {
    path: '/oauth/authorize',
    name: 'OAuthAuthorize',
    component: () => import('@/views/auth/OAuthAuthorizeView.vue'),
    meta: { requiresAuth: false }  // Component handles auth internally
  },
  {
    path: '/confirm-email',
    name: 'ConfirmEmail',
    component: () => import('@/views/auth/ConfirmEmailView.vue'),
    meta: { guest: true }
  },
  {
    path: '/forgot-password',
    name: 'ForgotPassword',
    component: () => import('@/views/auth/ForgotPasswordView.vue'),
    meta: { guest: true }
  },
  {
    path: '/reset-password',
    name: 'ResetPassword',
    component: () => import('@/views/auth/ResetPasswordView.vue'),
    meta: { guest: true }
  },
  
  // Dashboard routes (must be before /:owner/:repo)
  {
    path: '/',
    name: 'Home',
    component: () => import('@/views/DashboardView.vue'),
    meta: { requiresAuth: true }
  },
  {
    path: '/dashboard/projects',
    name: 'MyProjects',
    component: () => import('@/views/dashboard/MyProjectsView.vue'),
    meta: { requiresAuth: true }
  },
  {
    path: '/dashboard/groups',
    name: 'MyGroups',
    component: () => import('@/views/dashboard/MyGroupsView.vue'),
    meta: { requiresAuth: true }
  },
  {
    path: '/dashboard/issues',
    name: 'MyIssues',
    component: () => import('@/views/dashboard/MyIssuesView.vue'),
    meta: { requiresAuth: true }
  },
  {
    path: '/dashboard/merge-requests',
    name: 'MyMergeRequests',
    component: () => import('@/views/dashboard/MyMergeRequestsView.vue'),
    meta: { requiresAuth: true }
  },
  {
    path: '/dashboard/todos',
    name: 'MyTodos',
    component: () => import('@/views/dashboard/MyTodosView.vue'),
    meta: { requiresAuth: true }
  },
  {
    path: '/dashboard/activity',
    name: 'MyActivity',
    component: () => import('@/views/dashboard/MyActivityView.vue'),
    meta: { requiresAuth: true }
  },
  
  // Explore routes
  {
    path: '/explore',
    redirect: '/explore/projects'
  },
  {
    path: '/explore/projects',
    name: 'ExploreProjects',
    component: () => import('@/views/explore/ExploreProjectsView.vue'),
    meta: { requiresAuth: false }
  },
  {
    path: '/explore/groups',
    name: 'ExploreGroups',
    component: () => import('@/views/explore/ExploreGroupsView.vue'),
    meta: { requiresAuth: false }
  },
  
  // Create routes
  {
    path: '/projects/new',
    name: 'NewProject',
    component: () => import('@/views/projects/NewProjectView.vue'),
    meta: { requiresAuth: true }
  },
  {
    path: '/groups/new',
    name: 'NewGroup',
    component: () => import('@/views/groups/NewGroupView.vue'),
    meta: { requiresAuth: true }
  },
  
  // User settings routes
  {
    path: '/-/profile',
    name: 'UserProfile',
    component: () => import('@/views/settings/UserProfileView.vue'),
    meta: { requiresAuth: true }
  },
  {
    path: '/-/profile/keys',
    name: 'SshKeys',
    component: () => import('@/views/settings/SshKeysView.vue'),
    meta: { requiresAuth: true }
  },
  {
    path: '/-/profile/two-factor',
    name: 'TwoFactor',
    component: () => import('@/views/settings/TwoFactorView.vue'),
    meta: { requiresAuth: true }
  },
  {
    path: '/-/profile/preferences',
    name: 'UserPreferences',
    component: () => import('@/views/settings/UserPreferencesView.vue'),
    meta: { requiresAuth: true }
  },
  {
    path: '/-/profile/tokens',
    name: 'PersonalAccessTokens',
    component: () => import('@/views/settings/PersonalAccessTokensView.vue'),
    meta: { requiresAuth: true }
  },
  {
    path: '/-/profile/accounts',
    name: 'LinkedAccounts',
    component: () => import('@/views/settings/LinkedAccountsView.vue'),
    meta: { requiresAuth: true }
  },
  {
    path: '/-/profile/applications',
    name: 'OAuthApplications',
    component: () => import('@/views/settings/OAuthApplicationsView.vue'),
    meta: { requiresAuth: true }
  },
  
  // Admin routes (require admin role)
  {
    path: '/admin',
    name: 'AdminDashboard',
    component: () => import('@/views/admin/AdminDashboardView.vue'),
    meta: { requiresAuth: true, requiresAdmin: true }
  },
  {
    path: '/admin/users',
    name: 'AdminUsers',
    component: () => import('@/views/admin/AdminUsersView.vue'),
    meta: { requiresAuth: true, requiresAdmin: true }
  },
  {
    path: '/admin/projects',
    name: 'AdminProjects',
    component: () => import('@/views/admin/AdminProjectsView.vue'),
    meta: { requiresAuth: true, requiresAdmin: true }
  },
  {
    path: '/admin/groups',
    name: 'AdminGroups',
    component: () => import('@/views/admin/AdminGroupsView.vue'),
    meta: { requiresAuth: true, requiresAdmin: true }
  },
  {
    path: '/admin/settings',
    name: 'AdminSettings',
    component: () => import('@/views/admin/AdminSettingsView.vue'),
    meta: { requiresAuth: true, requiresAdmin: true }
  },
  {
    path: '/admin/oauth-providers',
    name: 'AdminOAuthProviders',
    component: () => import('@/views/admin/AdminOAuthProvidersView.vue'),
    meta: { requiresAuth: true, requiresAdmin: true }
  },
  
  // User/Group profile (single segment path - handles both users and groups)
  {
    path: '/:namespace',
    name: 'Namespace',
    component: () => import('@/views/namespace/NamespaceView.vue'),
    meta: { requiresAuth: false }
  },

  // Group sub-pages (single level group with sub-routes)
  {
    path: '/:namespace/-/members',
    name: 'GroupMembers',
    component: () => import('@/views/groups/GroupMembersView.vue'),
    meta: { requiresAuth: true, contextType: 'group' }
  },
  {
    path: '/:namespace/-/settings',
    name: 'GroupSettings',
    component: () => import('@/views/groups/GroupSettingsView.vue'),
    meta: { requiresAuth: true, contextType: 'group' }
  },
  
  // Project routes (must be LAST - catches /:owner/:repo)
  {
    path: '/:owner/:repo',
    name: 'Project',
    component: () => import('@/views/projects/ProjectView.vue'),
    meta: { requiresAuth: false },
    children: [
      {
        path: '',
        name: 'ProjectOverview',
        component: () => import('@/views/projects/ProjectOverview.vue')
      },
      {
        path: '-/tree/:ref?/:path(.*)?',
        name: 'ProjectFiles',
        component: () => import('@/views/repository/FileBrowserView.vue')
      },
      {
        path: '-/blob/:ref/:path(.*)',
        name: 'ProjectBlob',
        component: () => import('@/views/repository/FileBrowserView.vue')
      },
      {
        path: '-/commits/:ref?',
        name: 'ProjectCommits',
        component: () => import('@/views/repository/CommitListView.vue')
      },
      {
        path: '-/commit/:sha',
        name: 'CommitDetail',
        component: () => import('@/views/repository/CommitDetailView.vue')
      },
      {
        path: '-/branches',
        name: 'ProjectBranches',
        component: () => import('@/views/repository/BranchListView.vue')
      },
      {
        path: '-/tags',
        name: 'ProjectTags',
        component: () => import('@/views/repository/TagListView.vue')
      },
      {
        path: '-/issues',
        name: 'Issues',
        component: () => import('@/views/issues/IssueListView.vue')
      },
      {
        path: '-/issues/new',
        name: 'NewIssue',
        component: () => import('@/views/issues/NewIssueView.vue')
      },
      {
        path: '-/issues/:iid',
        name: 'IssueDetail',
        component: () => import('@/views/issues/IssueDetailView.vue')
      },
      {
        path: '-/merge_requests',
        name: 'MergeRequests',
        component: () => import('@/views/merge-requests/MergeRequestListView.vue')
      },
      {
        path: '-/merge_requests/new',
        name: 'NewMergeRequest',
        component: () => import('@/views/merge-requests/NewMergeRequestView.vue')
      },
      {
        path: '-/merge_requests/:iid',
        name: 'MergeRequestDetail',
        component: () => import('@/views/merge-requests/MergeRequestDetailView.vue')
      },
      {
        path: '-/pipelines',
        name: 'Pipelines',
        component: () => import('@/views/pipelines/PipelineListView.vue')
      },
      {
        path: '-/pipelines/:pipelineId',
        name: 'PipelineDetail',
        component: () => import('@/views/pipelines/PipelineDetailView.vue')
      },
      {
        path: '-/settings',
        name: 'ProjectSettings',
        component: () => import('@/views/projects/ProjectSettingsView.vue')
      },
      {
        path: '-/forks/new',
        name: 'ForkProject',
        component: () => import('@/views/projects/ForkProjectView.vue'),
        meta: { requiresAuth: true }
      }
    ]
  }
]

const router = createRouter({
  history: createWebHistory(),
  routes
})

// 安全的 redirect 路径验证（防止 Open Redirect 攻击）
function isSafeRedirect(path: string): boolean {
  if (!path || typeof path !== 'string') return false
  
  // 只允许内部路径（以 / 开头，不包含协议）
  if (!path.startsWith('/')) return false
  if (path.includes('://')) return false
  if (path.startsWith('//')) return false // 防止 protocol-relative URL
  
  return true
}

// OAuth redirect URL 验证（允许外部 URL，用于 OAuth 回调）
// 注意：OAuth redirect_uri 的验证在后端完成（检查是否匹配注册的应用配置）
// 这里只是传递给 OAuth 授权端点，不做验证
function isOAuthFlow(to: any): boolean {
  // OAuth2 授权流程会通过 query 参数传递 redirect_uri
  return to.query && typeof to.query.redirect_uri === 'string'
}

router.beforeEach((to, _from, next) => {
  const authStore = useAuthStore()
  
  if (to.meta.requiresAuth && !authStore.isAuthenticated) {
    // 需要登录但未登录，首页特殊处理：跳转到探索页
    if (to.path === '/') {
      next({ name: 'ExploreProjects' })
    } else {
      // 安全地保存 redirect 路径到 sessionStorage（不通过 URL 传递，防止 XSS）
      // 注意：OAuth 流程除外，OAuth 使用 redirect_uri query 参数（后端验证）
      if (!isOAuthFlow(to) && isSafeRedirect(to.fullPath)) {
        sessionStorage.setItem('login_redirect', to.fullPath)
      }
      next({ name: 'Login' })
    }
  } else if (to.meta.requiresAdmin && !authStore.isAdmin) {
    // 需要管理员权限但非管理员
    next({ name: 'Home' })
  } else if (to.meta.guest && authStore.isAuthenticated) {
    next({ name: 'Home' })
  } else {
    next()
  }
})

export default router
