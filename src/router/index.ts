import { createMemoryHistory, createRouter } from 'vue-router'

import Home from '../pages/Home/index.vue'
import Chat from '../pages/Chat/index.vue'

const routes = [
  { path: '/', component: Home },
  { path: '/Chat', component: Chat },
]

const router = createRouter({
  history: createMemoryHistory(),
  routes,
})

export default router