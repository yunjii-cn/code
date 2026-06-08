import { createApp } from 'vue'
import { createPinia } from 'pinia'
import router from './router'
import App from './App.vue'
// 2026-06-08 TASK-1.9 引入：设计系统 token + reset
import '@shared/styles/tokens.css'
import '@shared/styles/reset.css'
import './styles/main.css'
import 'vant/lib/index.css'

const app = createApp(App)
app.use(createPinia())
app.use(router)
app.mount('#app')
