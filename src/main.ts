import { createApp } from "vue";

// 入口组件
import App from "./App.vue";

// 路由
import router from "./router";

// 全局状态管理
import { createPinia } from 'pinia'

// Vue框架下面的一个生态：UI库
import ElementPlus from 'element-plus'
import 'element-plus/dist/index.css'

const pinia = createPinia()
const app = createApp(App)

app.use(router)
app.use(ElementPlus)
app.use(pinia)
app.mount('#app')
