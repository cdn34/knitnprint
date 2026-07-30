export type {
  ChangeProductStatusRequest,
  CreateProductRequest,
  CreateVariantRequest,
  CreateStaffRequest,
  DisableStaffRequest,
  ErrorBody,
  ErrorDetail,
  Health,
  LoginRequest,
  Product,
  StaffProfile,
  StaffRecord,
  Variant,
} from './schema'

import type {
  ChangeProductStatusRequest,
  CreateProductRequest,
  CreateStaffRequest,
  DisableStaffRequest,
  ErrorBody,
  Health,
  LoginRequest,
  Product,
  StaffProfile,
  StaffRecord,
} from './schema'

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

  function withQuery(
    path: string,
    query: Record<string, string | undefined>,
  ) {
    const search = new URLSearchParams()
    for (const [key, value] of Object.entries(query)) {
      if (value) search.set(key, value)
    }
    const suffix = search.toString()
    return suffix ? `${path}?${suffix}` : path
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
    listStaff: () => send<Array<StaffRecord>>('/api/admin/staff'),
    createStaff: (input: CreateStaffRequest) =>
      send<StaffRecord>('/api/admin/staff', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify(input),
      }),
    disableStaff: (staffId: string, input: DisableStaffRequest) =>
      send<void>(`/api/admin/staff/${staffId}/disable`, {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify(input),
      }),
    listAdminProducts: (query: { q?: string; status?: string } = {}) =>
      send<Array<Product>>(withQuery('/api/admin/products', query)),
    adminProduct: (productId: string) =>
      send<Product>(`/api/admin/products/${productId}`),
    createProduct: (input: CreateProductRequest) =>
      send<Product>('/api/admin/products', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify(input),
      }),
    changeProductStatus: (
      productId: string,
      input: ChangeProductStatusRequest,
    ) =>
      send<Product>(`/api/admin/products/${productId}/status`, {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify(input),
      }),
    listProducts: (query: { q?: string } = {}) =>
      send<Array<Product>>(withQuery('/api/products', query)),
    product: (slug: string) =>
      send<Product>(`/api/products/${encodeURIComponent(slug)}`),
  }
}
