export type {
  ChangeProductStatusRequest,
  AssignCategoriesRequest,
  AdjustInventoryRequest,
  Category,
  CompleteUploadRequest,
  CreateProductRequest,
  CreateCategoryRequest,
  CreateVariantRequest,
  CreateStaffRequest,
  CustomerAddress,
  CustomerAddressInput,
  CustomerDetail,
  CustomerSummary,
  DisableStaffRequest,
  ErrorBody,
  ErrorDetail,
  Health,
  GuestCustomerReceipt,
  GuestCustomerRequest,
  InitiateUploadRequest,
  InitiateUploadResponse,
  InventoryMovement,
  InventoryRecord,
  LoginRequest,
  MediaRecord,
  Product,
  ProductMedia,
  StaffProfile,
  StaffRecord,
  Variant,
} from './schema'

import type {
  ChangeProductStatusRequest,
  AssignCategoriesRequest,
  AdjustInventoryRequest,
  Category,
  CompleteUploadRequest,
  CreateProductRequest,
  CreateCategoryRequest,
  CreateVariantRequest,
  CreateStaffRequest,
  CustomerDetail,
  CustomerSummary,
  DisableStaffRequest,
  ErrorBody,
  Health,
  GuestCustomerReceipt,
  GuestCustomerRequest,
  InitiateUploadRequest,
  InitiateUploadResponse,
  InventoryMovement,
  InventoryRecord,
  LoginRequest,
  MediaRecord,
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
    createGuestCustomer: (input: GuestCustomerRequest, idempotencyKey: string) =>
      send<GuestCustomerReceipt>('/api/customers/guest', {
        method: 'POST',
        headers: {
          'content-type': 'application/json',
          'idempotency-key': idempotencyKey,
        },
        body: JSON.stringify(input),
      }),
    listCustomers: (query: { q?: string } = {}) =>
      send<Array<CustomerSummary>>(withQuery('/api/admin/customers', query)),
    customer: (customerId: string) =>
      send<CustomerDetail>(`/api/admin/customers/${customerId}`),
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
    listCategories: () => send<Array<Category>>('/api/admin/categories'),
    createCategory: (input: CreateCategoryRequest) =>
      send<Category>('/api/admin/categories', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify(input),
      }),
    addProductVariant: (productId: string, input: CreateVariantRequest) =>
      send<Product>(`/api/admin/products/${productId}/variants`, {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify(input),
      }),
    assignProductCategories: (
      productId: string,
      input: AssignCategoriesRequest,
    ) =>
      send<Product>(`/api/admin/products/${productId}/categories`, {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify(input),
      }),
    listInventory: () => send<Array<InventoryRecord>>('/api/admin/inventory'),
    inventoryMovements: (variantId: string) =>
      send<Array<InventoryMovement>>(
        `/api/admin/inventory/${variantId}/movements`,
      ),
    adjustInventory: (variantId: string, input: AdjustInventoryRequest) =>
      send<InventoryRecord>(`/api/admin/inventory/${variantId}/adjust`, {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify(input),
      }),
    listPublicCategories: () => send<Array<Category>>('/api/categories'),
    listProducts: (query: { q?: string; category?: string } = {}) =>
      send<Array<Product>>(withQuery('/api/products', query)),
    product: (slug: string) =>
      send<Product>(`/api/products/${encodeURIComponent(slug)}`),
    initiateMediaUpload: (input: InitiateUploadRequest) =>
      send<InitiateUploadResponse>('/api/admin/media/uploads', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify(input),
      }),
    uploadMediaObject: async (
      uploadUrl: string,
      file: Blob,
      contentType: string,
    ) => {
      const response = await request(uploadUrl, {
        method: 'PUT',
        headers: { 'content-type': contentType },
        body: file,
      })
      if (!response.ok) {
        throw new Error('The image could not be uploaded to media storage.')
      }
    },
    completeMediaUpload: (mediaId: string, input: CompleteUploadRequest) =>
      send<MediaRecord>(`/api/admin/media/uploads/${mediaId}/complete`, {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify(input),
      }),
  }
}
