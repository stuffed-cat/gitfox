import { createRouter, createWebHistory } from 'vue-router'
import type { RouteRecordRaw } from 'vue-router'

const routes: RouteRecordRaw[] = [
  {
    path: '/',
    name: 'Home',
    component: () => import('@/views/HomeView.vue'),
    meta: {
      title: 'GitFox WebIDE - 选择项目'
    }
  },
  {
    path: '/:owner/:repo',
    name: 'IDE',
    component: () => import('@/views/IDEView.vue'),
    props: (route) => ({ owner: route.params.owner as string, repo: route.params.repo as string, gitRef: undefined, path: undefined }),
    meta: {
      title: 'GitFox WebIDE'
    }
  },
  {
    path: '/:owner/:repo/-/ide/:ref?/:path(.*)?',
    name: 'IDEWithPath',
    component: () => import('@/views/IDEView.vue'),
    props: (route) => ({ owner: route.params.owner as string, repo: route.params.repo as string, gitRef: route.params.ref as string, path: route.params.path as string }),
    meta: {
      title: 'GitFox WebIDE'
    }
  },
  {
    path: '/:owner/:repo/:ref/:path(.*)',
    name: 'IDEWithRefAndPath',
    component: () => import('@/views/IDEView.vue'),
    props: (route) => ({ owner: route.params.owner as string, repo: route.params.repo as string, gitRef: route.params.ref as string, path: route.params.path as string }),
    meta: {
      title: 'GitFox WebIDE'
    }
  }
]

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes
})

// Update document title
router.beforeEach((to, _from, next) => {
  document.title = (to.meta.title as string) || 'GitFox WebIDE'
  next()
})

export default router
