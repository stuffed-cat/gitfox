import { createRouter, createWebHistory } from 'vue-router'
import { useAuthStore } from '@/stores/auth'

const routes = [
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
    path: '/',
    name: 'Dashboard',
    component: () => import('@/views/DashboardView.vue'),
    meta: { requiresAuth: true }
  },
  {
    path: '/projects',
    name: 'Projects',
    component: () => import('@/views/projects/ProjectListView.vue'),
    meta: { requiresAuth: true }
  },
  {
    path: '/projects/new',
    name: 'NewProject',
    component: () => import('@/views/projects/NewProjectView.vue'),
    meta: { requiresAuth: true }
  },
  {
    path: '/projects/:slug',
    name: 'Project',
    component: () => import('@/views/projects/ProjectView.vue'),
    meta: { requiresAuth: true },
    children: [
      {
        path: '',
        name: 'ProjectOverview',
        component: () => import('@/views/projects/ProjectOverview.vue')
      },
      {
        path: 'files',
        name: 'ProjectFiles',
        component: () => import('@/views/repository/FileBrowserView.vue')
      },
      {
        path: 'files/:path(.*)',
        name: 'ProjectFilePath',
        component: () => import('@/views/repository/FileBrowserView.vue')
      },
      {
        path: 'commits',
        name: 'ProjectCommits',
        component: () => import('@/views/repository/CommitListView.vue')
      },
      {
        path: 'commits/:sha',
        name: 'CommitDetail',
        component: () => import('@/views/repository/CommitDetailView.vue')
      },
      {
        path: 'branches',
        name: 'ProjectBranches',
        component: () => import('@/views/repository/BranchListView.vue')
      },
      {
        path: 'tags',
        name: 'ProjectTags',
        component: () => import('@/views/repository/TagListView.vue')
      },
      {
        path: 'merge-requests',
        name: 'MergeRequests',
        component: () => import('@/views/merge-requests/MergeRequestListView.vue')
      },
      {
        path: 'merge-requests/new',
        name: 'NewMergeRequest',
        component: () => import('@/views/merge-requests/NewMergeRequestView.vue')
      },
      {
        path: 'merge-requests/:iid',
        name: 'MergeRequestDetail',
        component: () => import('@/views/merge-requests/MergeRequestDetailView.vue')
      },
      {
        path: 'pipelines',
        name: 'Pipelines',
        component: () => import('@/views/pipelines/PipelineListView.vue')
      },
      {
        path: 'pipelines/:pipelineId',
        name: 'PipelineDetail',
        component: () => import('@/views/pipelines/PipelineDetailView.vue')
      },
      {
        path: 'settings',
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
    next({ name: 'Dashboard' })
  } else {
    next()
  }
})

export default router
