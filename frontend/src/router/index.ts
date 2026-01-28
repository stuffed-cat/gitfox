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
    path: '/-/profile/keys',
    name: 'SshKeys',
    component: () => import('@/views/settings/SshKeysView.vue'),
    meta: { requiresAuth: true }
  },
  
  // User/Group profile (single segment path)
  {
    path: '/:namespace',
    name: 'Namespace',
    component: () => import('@/views/namespace/NamespaceView.vue'),
    meta: { requiresAuth: false }
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
      }
    ]
  }
]

const router = createRouter({
  history: createWebHistory(),
  routes
})

router.beforeEach((to, _from, next) => {
  const authStore = useAuthStore()
  
  if (to.meta.requiresAuth && !authStore.isAuthenticated) {
    next({ name: 'Login' })
  } else if (to.meta.guest && authStore.isAuthenticated) {
    next({ name: 'Home' })
  } else {
    next()
  }
})

export default router
