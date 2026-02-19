import createClient, { type Middleware } from 'openapi-fetch'

// import type { paths } from './schema'

let getAccessToken: (() => string | null) | undefined
let onRefreshNeeded: (() => Promise<boolean>) | undefined

export function setupAuthInterceptor(options: {
  getToken: () => string | null
  refresh: () => Promise<boolean>
}) {
  getAccessToken = options.getToken
  onRefreshNeeded = options.refresh
}

const authMiddleware: Middleware = {
  onRequest({ request }) {
    const token = getAccessToken?.()
    if (token) {
      request.headers.set('Authorization', `Bearer ${token}`)
    }
    return request
  },
  async onResponse({ response, request }) {
    if (response.status === 401 && onRefreshNeeded) {
      const refreshed = await onRefreshNeeded()
      if (refreshed) {
        const token = getAccessToken?.()
        if (token) {
          const retryRequest = new Request(request.url, {
            method: request.method,
            headers: new Headers(request.headers),
            body: request.body,
          })
          retryRequest.headers.set('Authorization', `Bearer ${token}`)
          return fetch(retryRequest)
        }
      }
    }
    return response
  },
}

const client = createClient({
  baseUrl: import.meta.env.VITE_API_BASE_URL ?? 'http://localhost:8080',
})

client.use(authMiddleware)

export default client
