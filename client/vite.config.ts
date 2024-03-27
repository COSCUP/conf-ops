import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import Components from 'unplugin-vue-components/vite'
import { NaiveUiResolver } from 'unplugin-vue-components/resolvers'
import VueI18nPlugin from '@intlify/unplugin-vue-i18n/vite'
import UnoCSS from 'unocss/vite'
import { resolve, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [
    UnoCSS(),
    vue(),
    Components({
      resolvers: [NaiveUiResolver()]
    }),
    VueI18nPlugin({
      include: resolve(dirname(fileURLToPath(import.meta.url)), 'src/locales/**')
    })
  ],
  build: {
    outDir: '../public'
  },
  server: {
    proxy: {
      '/api': 'http://127.0.0.1:8000/'
    }
  },
  resolve: {
    alias: {
      '@': resolve(dirname(fileURLToPath(import.meta.url)), 'src')
    }
  }
})
