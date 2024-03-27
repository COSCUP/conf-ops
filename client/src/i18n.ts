import { createI18n, useI18n } from 'vue-i18n'
import { InjectionKey, Ref, ShallowRef, computed, inject, provide, shallowRef } from 'vue'
import type { NLocale, NDateLocale } from 'naive-ui'
import { Locale as zhTW, DateLocale as dateZhTW } from './utils/naive/tw'
import messages from '@intlify/unplugin-vue-i18n/messages'

function getDefaultLocale() {
  const languages = navigator.languages
  for (const language of languages) {
    if (language.startsWith('zh')) {
      return 'zh'
    }
    if (language.startsWith('en')) {
      return 'en'
    }
  }
  return 'en'
}

interface LocaleContext {
  locale: Ref<'zh' | 'en'>
  naiveLocale: ShallowRef<NLocale>
  naiveDateLocale: ShallowRef<NDateLocale>
}

const localeSymbol: InjectionKey<LocaleContext> = Symbol('locale')

export function provideLocale() {
  const { locale: _locale } = useI18n()
  const naiveLocale = shallowRef<NLocale>(zhTW)
  const naiveDateLocale = shallowRef<NDateLocale>(dateZhTW)

  const locale = computed({
    get: () => _locale.value as 'zh' | 'en',
    set: (locale) => {
      _locale.value = locale
      setLocale(locale)
    }
  })

  const setLocale = (locale: 'zh' | 'en') => {
    _locale.value = locale
    document.querySelector('html')?.setAttribute('lang', locale === 'zh' ? 'zh-TW' : 'en')
    if (locale === 'zh') {
      naiveLocale.value = zhTW
      naiveDateLocale.value = dateZhTW
    }
    if (locale === 'en') {
      import('./utils/naive/en').then(({ Locale, DateLocale }) => {
        naiveLocale.value = Locale
        naiveDateLocale.value = DateLocale
      })
    }
  }

  setLocale(getDefaultLocale())

  provide(localeSymbol, {
    locale,
    naiveLocale,
    naiveDateLocale
  })

  return { naiveLocale, naiveDateLocale }
}

export function useLocale() {
  const injected = inject(localeSymbol)

  if (!injected) {
    throw new Error('useLocale() is called without provideLocale().')
  }

  return injected
}

export const i18n = createI18n({
  legacy: false,
  locale: getDefaultLocale(),
  fallbackLocale: 'en',
  messages
})
