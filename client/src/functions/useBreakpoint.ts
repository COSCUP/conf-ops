import { computed, onBeforeUnmount, onMounted, ref } from 'vue'

export function useBreakpoint () {
  const mediaQuery = window.matchMedia(`(min-width: 640px)`)
  const isMatch = ref(true)

  const handleBreakPointChange = (e: MediaQueryListEvent | MediaQueryList) => {
    if (e.matches) {
      isMatch.value = true
    } else {
      isMatch.value = false
    }
  }
  onMounted(() => {
    handleBreakPointChange(mediaQuery)

    if (mediaQuery.addEventListener) {
      mediaQuery.addEventListener('change', handleBreakPointChange)
    } else {
      mediaQuery.addListener(handleBreakPointChange)
    }
  })

  onBeforeUnmount(() => {
    if (mediaQuery.removeEventListener) {
      mediaQuery.removeEventListener('change', handleBreakPointChange)
    } else {
      mediaQuery.removeListener(handleBreakPointChange)
    }
  })

  return {
    isMobile: computed(() => !isMatch.value),
    isDesktop: computed(() => isMatch.value)
  }
}
