import { ofetch } from 'ofetch'
import { i18n } from '../i18n'

export interface APIError<F extends Record<string, string> = Record<string, string>> {
  status: string
  status_code: number
  message: string
  fields: F
}

export type APIResponse<T, F extends Record<string, string> = Record<string, string>> = { __api_error: APIError<F> } & Promise<T>

export const instance = ofetch.create({
  baseURL: '/api',
  timeout: 30000,
  retry: 1,
  retryDelay: 1000,
  headers: {
    'Content-Type': 'application/json',
  },
  onRequest ({ options }) {
    (options.headers as Record<string, string>)['X-USER-LOCALE'] = i18n.global.locale.value
  },
  parseResponse: (res) => JSON.parse(res, (key, value) => {
    if (key.endsWith('_at')) {
      return new Date(value * 1000)
    }
    return value
  })
})

export const makeRequest = <T, F extends Record<string, string> = Record<string, string>>(p: Promise<unknown>): APIResponse<T, F> => {
  return p.catch((err) => {
    throw { ...err.data, status_code: err.status }
  }) as APIResponse<T, F>
}

export const makeAnyRequest = <T>(v: T): APIResponse<T, {}> => {
  return Promise.resolve(v) as APIResponse<T, {}>
}

export const makeEmptyRequest = (): APIResponse<null, {}> => {
  return makeAnyRequest(null)
}
