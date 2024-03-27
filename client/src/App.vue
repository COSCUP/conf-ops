<template>
  <NConfigProvider
    :locale="naiveLocale"
    :date-locale="naiveDateLocale"
    :theme="darkTheme"
  >
    <NDialogProvider>
      <NMessageProvider>
        <NLayout class="h-screen h-dvh">
          <NLoadingBarProvider>
            <RouterView />
          </NLoadingBarProvider>
          <NLayoutFooter class="text-center pa-1">
            <n-text
              type="secondary"
            >
              ConfOps ©2024
            </n-text>
          </NLayoutFooter>
        </NLayout>
      </NMessageProvider>
    </NDialogProvider>
  </NConfigProvider>
</template>

<script setup lang="ts">
import { darkTheme } from 'naive-ui'
import { provideLocale } from './i18n'
import { RouterView } from 'vue-router';

const { naiveLocale, naiveDateLocale } = provideLocale()
</script>
