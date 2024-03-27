import { useLoadingBar } from 'naive-ui'
import { watch } from 'vue'
import { useRouter } from 'vue-router'

export function usePageTitle (title: () => string) {
  watch(title, (value) => {
    document.title = `${value} - ConfOps`
  }, { immediate: true })
}

export function usePageLoading (loading: () => boolean) {
  const loadingBar = useLoadingBar()

  watch(loading, (value) => {
    if (value) {
      loadingBar.start()
    } else {
      loadingBar.finish()
    }
  }, { immediate: true })
}

export function usePageNotFound (condition: () => boolean) {
  const router = useRouter()

  watch(condition, (value) => {
    if (value) {
      router.replace('/404')
    }
  }, { immediate: true })
}
