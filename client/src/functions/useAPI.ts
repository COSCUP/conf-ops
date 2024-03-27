import { FormInst, useDialog } from 'naive-ui'
import { computed, shallowRef } from 'vue'
import { APIError, APIResponse } from '../api/base'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'

interface UseAPIConfig<D, F extends Record<string, string>> {
  validate: () => boolean
  immediate: boolean
  default: D
  failure: (error: APIError<F>) => void
}

export const getAPIErrorMessage = <F extends Record<string, string>>(error: APIError<F>) => {
  let message = error.message
  for (const key in error.fields) {
    message += `\n${key}: ${error.fields[key]}`
  }
  return message
}

export const useAPI = <R, F extends Record<string, string>, C extends Partial<UseAPIConfig<R, F>>>(api: () => APIResponse<R, F>, config?: C) => {
  type DefaultData = C extends { default: any } ? R : null

  const { t } = useI18n()
  const router = useRouter()
  const dialog = useDialog()
  const loading = shallowRef(false)
  const disabled = computed(() => !(config?.validate?.() ?? true))
  const data = shallowRef<R | DefaultData>(config?.default ?? null as DefaultData)

  const execute = async () => {
    if (disabled.value) return
    if (loading.value) return
    loading.value = true
    try {
      data.value = await api()
    } catch (e) {
      const error = e as APIError<F>
      if (error.status_code === 401) {
        router.push('/')
        dialog.destroyAll()
        dialog.warning({
          title: t('unauthorized.title'),
          content: t('unauthorized.content'),
          positiveText: t('ok')
        })
        return
      }
      if (config?.failure) {
        config.failure(error)
      } else {
        console.error(error)
        dialog.error({
          title: t('error'),
          content: getAPIErrorMessage(error),
          class: 'whitespace-pre-line',
          positiveText: t('ok')
        })
      }
      throw error
    } finally {
      loading.value = false
    }
  }

  if ((config?.immediate ?? true)) {
    execute()
  }

  return {
    disabled,
    loading,
    data,
    execute
  }
}

export const useActionAPI = <D, R, F extends Record<string, string>, C extends Omit<Partial<UseAPIConfig<R, F>>, 'immediate'>>(
  api: (data: D) => APIResponse<R, F>,
  success?: (data: R) => void,
  config?: C
) => {
  let reqData: D
  const { disabled, loading, data, execute } = useAPI(() => api(reqData!), { ...config, immediate: false })

  const new_execute = async (newData: D) => {
    reqData = newData
    await execute()
    success?.(data.value as R)
    return data.value as R
  }

  return {
    disabled,
    loading,
    execute: new_execute
  }
}

export const useFormAPI = <D, R, F extends Record<string, string>, C extends Partial<UseAPIConfig<R, F>>>(
  api: (data: D) => APIResponse<R, F>,
  getForm: () => FormInst | null,
  success?: (data: R) => void,
  config?: C
) => {
  const { disabled, loading, execute } = useActionAPI(api, success, config)
  return {
    disabled,
    loading,
    execute: async (newData: D) => {
      const form = getForm()
      if (!form) {
        throw new Error('form is required')
      }
      await form.validate?.()
      return await execute(newData)
    }
  }
}
