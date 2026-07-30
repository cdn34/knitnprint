export type {
  ErrorBody,
  ErrorDetail,
  Health,
  LoginRequest,
  StaffProfile,
} from './schema'

import type { ErrorBody, Health, LoginRequest, StaffProfile } from './schema'

export class ApiError extends Error {
  constructor(
    public readonly status: number,
    public readonly body: ErrorBody,
  ) {
    super(body.error.message)
    this.name = 'ApiError'
  }
}

export interface ApiClientOptions {
  baseUrl?: string
  fetch?: typeof globalThis.fetch
}

export function createApiClient(options: ApiClientOptions = {}) {
  const baseUrl = options.baseUrl?.replace(/\/$/, '') ?? ''
  const request = options.fetch ?? globalThis.fetch

  async function send<T>(path: string, init: RequestInit = {}): Promise<T> {
    const response = await request(`${baseUrl}${path}`, {
      credentials: 'include',
      ...init,
      headers: {
        accept: 'application/json',
        ...init.headers,
      },
    })
    const body: unknown =
      response.status === 204 ? undefined : await response.json()

    if (!response.ok) {
      throw new ApiError(response.status, body as ErrorBody)
    }
    return body as T
  }

  return {
    health: () => send<Health>('/api/health'),
    readiness: () => send<Health>('/api/ready'),
    login: (input: LoginRequest) =>
      send<StaffProfile>('/api/admin/auth/login', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify(input),
      }),
    logout: () =>
      send<void>('/api/admin/auth/logout', {
        method: 'POST',
      }),
    profile: () => send<StaffProfile>('/api/admin/auth/me'),
  }
}
