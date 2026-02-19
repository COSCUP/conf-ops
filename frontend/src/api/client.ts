import createClient from 'openapi-fetch'

// import type { paths } from './schema'

// The API client will use generated types from OpenAPI spec once available.
// For now, create an untyped client pointing to the backend.
const client = createClient({
  baseUrl: import.meta.env.VITE_API_BASE_URL ?? 'http://localhost:8080',
})

export default client
