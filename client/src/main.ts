import { createApp } from 'vue'
import App from './App.vue'
import { i18n } from './i18n'
import { router } from './router'

// UnoCSS
import 'virtual:uno.css'
// General Font
import 'vfonts/Lato.css'
// Monospace Font
import 'vfonts/FiraCode.css'

createApp(App)
  .use(router)
  .use(i18n)
  .mount('#app')
