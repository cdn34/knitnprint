export type { ErrorBody, ErrorDetail, Health } from './schema'

import type { ErrorBody, Health } from './schema'

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

  async function getHealth(path: '/api/health' | '/api/ready'): Promise<Health> {
    const response = await request(`${baseUrl}${path}`, {
      headers: { accept: 'application/json' },
    })
    const body: unknown = await response.json()

    if (!response.ok) {
      throw new ApiError(response.status, body as ErrorBody)
    }
    return body as Health
  }

  return {
    health: () => getHealth('/api/health'),
    readiness: () => getHealth('/api/ready'),
  }
}

