import { api } from '@/api'
import { Feature, Project, User } from '@/api/modules/project'
import { Role } from '@/api/modules/role'
import { useAPI } from '@/functions/useAPI'
import { useLocale } from '@/i18n'
import { InjectionKey, ComputedRef, ShallowRef, computed, provide, watch, inject } from 'vue'
import { useRoute } from 'vue-router'


interface ProjectContext {
  user: ShallowRef<User | null>
  roles: ShallowRef<Role[]>
  project: ShallowRef<Project | null>
  features: ShallowRef<Feature[]>
  loading: ComputedRef<boolean>,
  reloadUser: () => Promise<void>
  reloadRoles: () => Promise<void>
  reloadProject: () => Promise<void>
  reloadFeatures: () => Promise<void>
}

const projectSymbol: InjectionKey<ProjectContext> = Symbol('project')

export function provideProject () {
  const { locale } = useLocale()
  const route = useRoute()

  const { data: user, loading: userLoading, execute: reloadUser } = useAPI(api.project.getMeInfo)

  const { data: project, loading: projectLoading, execute: reloadProject } = useAPI(api.project.getProjectInfo)

  const { data: features, loading: featuresLoading, execute: reloadFeatures } = useAPI(api.project.getFeatures, { default: [] })

  const { data: roles, loading: rolesLoading, execute: reloadRoles } = useAPI(api.role.getMyRoles, { default: [] })

  watch(user, (data) => {
    if (data) {
      locale.value = data.locale === 'zh' ? 'zh' : 'en'
    }
  }, { immediate: true })

  watch(route, () => {
    reloadFeatures()
  })

  const loading = computed(() => userLoading.value || projectLoading.value || featuresLoading.value || rolesLoading.value)

  const context = {
    user,
    roles,
    project,
    features,
    loading,
    reloadUser,
    reloadRoles,
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
