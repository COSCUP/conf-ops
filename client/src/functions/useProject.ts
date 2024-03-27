import { api } from '@/api'
import { Feature, Project, User } from '@/api/modules/project'
import { useAPI } from '@/functions/useAPI'
import { useLocale } from '@/i18n'
import { InjectionKey, ComputedRef, ShallowRef, computed, provide, watch, inject } from 'vue'


interface ProjectContext {
  user: ShallowRef<User | null>
  project: ShallowRef<Project | null>
  features: ShallowRef<Feature[]>
  loading: ComputedRef<boolean>,
  reloadUser: () => Promise<void>
  reloadProject: () => Promise<void>
  reloadFeatures: () => Promise<void>
}

const projectSymbol: InjectionKey<ProjectContext> = Symbol('project')

export function provideProject () {
  const { locale } = useLocale()

  const { data: user, loading: userLoading, execute: reloadUser } = useAPI(() => api.project.getMeInfo())

  const { data: project, loading: projectLoading, execute: reloadProject } = useAPI(() => api.project.getProjectInfo())

  const { data: features, loading: featuresLoading, execute: reloadFeatures } = useAPI(() => api.project.getFeatures(), { default: [] })

  const initWatcher = watch(user, (data) => {
    if (data) {
      locale.value = data.locale === 'zh' ? 'zh' : 'en'
      initWatcher()
    }
  }, { immediate: true })

  const loading = computed(() => userLoading.value || projectLoading.value || featuresLoading.value)

  const context = {
    user,
    project,
    features,
    loading,
    reloadUser,
    reloadProject,
    reloadFeatures,
  }

  provide(projectSymbol, context)

  return context
}

export function useProject () {
  const context = inject(projectSymbol)

  if (!context) {
    throw new Error('useProject() is called without provideProject().')
  }

  return context
}
