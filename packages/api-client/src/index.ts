export type {
  AddCartItemRequest,
  AdminProductFeedback,
  AppliedDiscount,
  ApplyDiscountRequest,
  AccountTokenRequest,
  ChangeProductStatusRequest,
  ChangeDiscountStatusRequest,
  AssignCategoriesRequest,
  Cart,
  CartAddress,
  CartDelivery,
  CartIssue,
  CartItem,
  CommercialSettings,
  CancelOrderRequest,
  AdjustInventoryRequest,
  Category,
  CompleteUploadRequest,
  CreateAccountAddressRequest,
  CreateFulfillmentLineRequest,
  CreateFulfillmentRequest,
  CreateDiscountRequest,
  CreateRefundLineRequest,
  CreateRefundRequest,
  CreateOrderRequest,
  CreateProductRequest,
  CreateProductFeedbackRequest,
  CreateCategoryRequest,
  CreateVariantRequest,
  CreateStaffRequest,
  CustomerAddress,
  CustomerAddressInput,
  CustomerAccountProfile,
  CustomerDetail,
  CustomerLoginRequest,
  CustomerRegisterRequest,
  CustomerSummary,
  Discount,
  DisableStaffRequest,
  ErrorBody,
  ErrorDetail,
  Health,
  GuestCustomerReceipt,
  GuestCustomerRequest,
  ForgotPasswordRequest,
  Fulfillment,
  FulfillmentLine,
  InitiateUploadRequest,
  InitiateUploadResponse,
  InventoryMovement,
  InventoryRecord,
  LoginRequest,
  MediaRecord,
  NotificationStatus,
  OperationalDashboard,
  ManualPaymentRequest,
  Order,
  OrderDiscount,
  OrderAddress,
  OrderCustomer,
  OrderEvent,
  OrderLine,
  OrderPayment,
  OrderOperations,
  OrderSummary,
  PaymentAttempt,
  PaymentCheckout,
  PaymentOptions,
  PaymentStatusEvent,
  PersonalizationConfig,
  Product,
  ProductFeedback,
  ProductFeedbackSummary,
  ProductMedia,
  ReorderCategoriesRequest,
  ReplyToProductFeedbackRequest,
  Refund,
  RefundLine,
  ResetPasswordRequest,
  SelectShippingMethodRequest,
  ShippingMethod,
  ShippingMethodInput,
  ShippingPackageProfile,
  ShippingPackageProfileRequest,
  ShippingSelection,
  ShippingZone,
  ShippingZoneInput,
  StaffProfile,
  StaffRecord,
  SubmittedProductFeedback,
  TaxRule,
  TaxRuleInput,
  TaxSelection,
  UpdateCommercialSettingsRequest,
  UpdateDiscountRequest,
  UpdateProductRequest,
  ModerateProductFeedbackRequest,
  Variant,
  UpdateCartItemRequest,
} from './schema'

import type {
  AddCartItemRequest,
  AdminProductFeedback,
  ApplyDiscountRequest,
  AccountTokenRequest,
  ChangeProductStatusRequest,
  ChangeDiscountStatusRequest,
  AssignCategoriesRequest,
  AdjustInventoryRequest,
  Category,
  Cart,
  CommercialSettings,
  CancelOrderRequest,
  CompleteUploadRequest,
  CreateAccountAddressRequest,
  CreateFulfillmentRequest,
  CreateDiscountRequest,
  CreateRefundRequest,
  CreateOrderRequest,
  CreateProductRequest,
  CreateProductFeedbackRequest,
  CreateCategoryRequest,
  CreateVariantRequest,
  CreateStaffRequest,
  CustomerAddress,
  CustomerAccountProfile,
  CustomerDetail,
  CustomerLoginRequest,
  CustomerRegisterRequest,
  CustomerSummary,
  Discount,
  DisableStaffRequest,
  ErrorBody,
  Health,
  GuestCustomerReceipt,
  GuestCustomerRequest,
  ForgotPasswordRequest,
  InitiateUploadRequest,
  InitiateUploadResponse,
  InventoryMovement,
  InventoryRecord,
  LoginRequest,
  MediaRecord,
  ManualPaymentRequest,
  Order,
  OperationalDashboard,
  OrderSummary,
  PaymentCheckout,
  PaymentOptions,
  Product,
  ProductFeedbackSummary,
  ReorderCategoriesRequest,
  ReplyToProductFeedbackRequest,
  ResetPasswordRequest,
  SelectShippingMethodRequest,
  ShippingPackageProfile,
  ShippingPackageProfileRequest,
  StaffProfile,
  StaffRecord,
  SubmittedProductFeedback,
  UpdateCommercialSettingsRequest,
  UpdateDiscountRequest,
  UpdateProductRequest,
  ModerateProductFeedbackRequest,
  UpdateCartItemRequest,
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
    dashboard: () => send<OperationalDashboard>('/api/admin/dashboard'),
    customerAccount: () =>
      send<CustomerAccountProfile>('/api/account/me'),
    registerCustomer: (input: CustomerRegisterRequest) =>
      send<CustomerAccountProfile>('/api/account/register', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify(input),
      }),
    loginCustomer: (input: CustomerLoginRequest) =>
      send<CustomerAccountProfile>('/api/account/login', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify(input),
      }),
    logoutCustomer: () =>
      send<void>('/api/account/logout', { method: 'POST' }),
    addCustomerAddress: (input: CreateAccountAddressRequest) =>
      send<CustomerAddress>('/api/account/addresses', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify(input),
      }),
    requestCustomerVerification: () =>
      send<void>('/api/account/verification/request', { method: 'POST' }),
    confirmCustomerVerification: (input: AccountTokenRequest) =>
      send<void>('/api/account/verification/confirm', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify(input),
      }),
    forgotCustomerPassword: (input: ForgotPasswordRequest) =>
      send<void>('/api/account/password/forgot', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify(input),
      }),
    resetCustomerPassword: (input: ResetPasswordRequest) =>
      send<void>('/api/account/password/reset', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify(input),
      }),
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
    customerOrders: (customerId: string) =>
      send<Array<OrderSummary>>(`/api/admin/customers/${customerId}/orders`),
    cart: () => send<Cart>('/api/cart'),
    addCartItem: (input: AddCartItemRequest, idempotencyKey: string) =>
      send<Cart>('/api/cart/items', {
        method: 'POST',
        headers: {
          'content-type': 'application/json',
          'idempotency-key': idempotencyKey,
        },
        body: JSON.stringify(input),
      }),
    updateCartItem: (
      lineId: string,
      input: UpdateCartItemRequest,
      idempotencyKey: string,
    ) =>
      send<Cart>(`/api/cart/items/${lineId}`, {
        method: 'PATCH',
        headers: {
          'content-type': 'application/json',
          'idempotency-key': idempotencyKey,
        },
        body: JSON.stringify(input),
      }),
    removeCartItem: (lineId: string, idempotencyKey: string) =>
      send<Cart>(`/api/cart/items/${lineId}`, {
        method: 'DELETE',
        headers: { 'idempotency-key': idempotencyKey },
      }),
    applyCartDiscount: (input: ApplyDiscountRequest, idempotencyKey: string) =>
      send<Cart>('/api/cart/discount', {
        method: 'POST',
        headers: {
          'content-type': 'application/json',
          'idempotency-key': idempotencyKey,
        },
        body: JSON.stringify(input),
      }),
    removeCartDiscount: (idempotencyKey: string) =>
      send<Cart>('/api/cart/discount', {
        method: 'DELETE',
        headers: { 'idempotency-key': idempotencyKey },
      }),
    selectCartShippingMethod: (
      input: SelectShippingMethodRequest,
      idempotencyKey: string,
    ) =>
      send<Cart>('/api/cart/shipping-method', {
        method: 'POST',
        headers: {
          'content-type': 'application/json',
          'idempotency-key': idempotencyKey,
        },
        body: JSON.stringify(input),
      }),
    refreshCartShippingQuotes: () =>
      send<Cart>('/api/cart/shipping-quotes', {
        method: 'POST',
      }),
    setCartDelivery: (input: GuestCustomerRequest, idempotencyKey: string) =>
      send<Cart>('/api/cart/delivery', {
        method: 'POST',
        headers: {
          'content-type': 'application/json',
          'idempotency-key': idempotencyKey,
        },
        body: JSON.stringify(input),
      }),
    createOrder: (input: CreateOrderRequest, idempotencyKey: string) =>
      send<Order>('/api/orders', {
        method: 'POST',
        headers: {
          'content-type': 'application/json',
          'idempotency-key': idempotencyKey,
        },
        body: JSON.stringify(input),
      }),
    customerOrder: (orderId: string) =>
      send<Order>(`/api/orders/${orderId}`),
    paymentOptions: () => send<PaymentOptions>('/api/payments/options'),
    startOrderPayment: (orderId: string) =>
      send<PaymentCheckout>(`/api/orders/${orderId}/payment`, {
        method: 'POST',
      }),
    listOrders: () => send<Array<OrderSummary>>('/api/admin/orders'),
    listDiscounts: () => send<Array<Discount>>('/api/admin/discounts'),
    createDiscount: (input: CreateDiscountRequest) =>
      send<Discount>('/api/admin/discounts', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify(input),
      }),
    updateDiscount: (discountId: string, input: UpdateDiscountRequest) =>
      send<Discount>(`/api/admin/discounts/${discountId}`, {
        method: 'PUT',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify(input),
      }),
    changeDiscountStatus: (
      discountId: string,
      input: ChangeDiscountStatusRequest,
    ) =>
      send<Discount>(`/api/admin/discounts/${discountId}/status`, {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify(input),
      }),
    settings: () => send<CommercialSettings>('/api/admin/settings'),
    updateSettings: (input: UpdateCommercialSettingsRequest) =>
      send<CommercialSettings>('/api/admin/settings', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify(input),
      }),
    order: (orderId: string) =>
      send<Order>(`/api/admin/orders/${orderId}`),
    recordManualPayment: (orderId: string, input: ManualPaymentRequest) =>
      send<Order>(`/api/admin/orders/${orderId}/manual-payment`, {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify(input),
      }),
    createFulfillment: (
      orderId: string,
      input: CreateFulfillmentRequest,
      idempotencyKey: string,
    ) =>
      send<Order>(`/api/admin/orders/${orderId}/fulfillments`, {
        method: 'POST',
        headers: {
          'content-type': 'application/json',
          'idempotency-key': idempotencyKey,
        },
        body: JSON.stringify(input),
      }),
    cancelOrder: (
      orderId: string,
      input: CancelOrderRequest,
      idempotencyKey: string,
    ) =>
      send<Order>(`/api/admin/orders/${orderId}/cancel`, {
        method: 'POST',
        headers: {
          'content-type': 'application/json',
          'idempotency-key': idempotencyKey,
        },
        body: JSON.stringify(input),
      }),
    createRefund: (
      orderId: string,
      input: CreateRefundRequest,
      idempotencyKey: string,
    ) =>
      send<Order>(`/api/admin/orders/${orderId}/refunds`, {
        method: 'POST',
        headers: {
          'content-type': 'application/json',
          'idempotency-key': idempotencyKey,
        },
        body: JSON.stringify(input),
      }),
    listAdminProducts: (query: { q?: string; status?: string } = {}) =>
      send<Array<Product>>(withQuery('/api/admin/products', query)),
    listAdminFeedback: (status = 'pending') =>
      send<Array<AdminProductFeedback>>(
        withQuery('/api/admin/feedback', { status }),
      ),
    moderateProductFeedback: (
      feedbackId: string,
      input: ModerateProductFeedbackRequest,
    ) =>
      send<AdminProductFeedback>(`/api/admin/feedback/${feedbackId}`, {
        method: 'PUT',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify(input),
      }),
    replyToProductFeedback: (
      feedbackId: string,
      input: ReplyToProductFeedbackRequest,
    ) =>
      send<AdminProductFeedback>(`/api/admin/feedback/${feedbackId}/reply`, {
        method: 'PUT',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify(input),
      }),
    adminProduct: (productId: string) =>
      send<Product>(`/api/admin/products/${productId}`),
    createProduct: (input: CreateProductRequest) =>
      send<Product>('/api/admin/products', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify(input),
      }),
    updateProduct: (productId: string, input: UpdateProductRequest) =>
      send<Product>(`/api/admin/products/${productId}`, {
        method: 'PUT',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify(input),
      }),
    deleteProduct: (productId: string) =>
      send<void>(`/api/admin/products/${productId}`, { method: 'DELETE' }),
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
    listShippingPackages: () =>
      send<Array<ShippingPackageProfile>>('/api/admin/shipping-packages'),
    createShippingPackage: (input: ShippingPackageProfileRequest) =>
      send<ShippingPackageProfile>('/api/admin/shipping-packages', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify(input),
      }),
    updateShippingPackage: (
      profileId: string,
      input: ShippingPackageProfileRequest,
    ) =>
      send<ShippingPackageProfile>(`/api/admin/shipping-packages/${profileId}`, {
        method: 'PUT',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify(input),
      }),
    deleteShippingPackage: (profileId: string) =>
      send<void>(`/api/admin/shipping-packages/${profileId}`, {
        method: 'DELETE',
      }),
    createCategory: (input: CreateCategoryRequest) =>
      send<Category>('/api/admin/categories', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify(input),
      }),
    reorderCategories: (input: ReorderCategoriesRequest) =>
      send<Array<Category>>('/api/admin/categories/order', {
        method: 'PUT',
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
    productFeedback: (slug: string) =>
      send<ProductFeedbackSummary>(
        `/api/products/${encodeURIComponent(slug)}/feedback`,
      ),
    submitProductFeedback: (
      slug: string,
      input: CreateProductFeedbackRequest,
    ) =>
      send<SubmittedProductFeedback>(
        `/api/products/${encodeURIComponent(slug)}/feedback`,
        {
          method: 'POST',
          headers: { 'content-type': 'application/json' },
          body: JSON.stringify(input),
        },
      ),
    initiateMediaUpload: (input: InitiateUploadRequest) =>
      send<InitiateUploadResponse>('/api/admin/media/uploads', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify(input),
      }),
    initiatePersonalizationUpload: (input: InitiateUploadRequest) =>
      send<InitiateUploadResponse>('/api/personalization/uploads', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify(input),
      }),
    completePersonalizationUpload: (mediaId: string) =>
      send<{ id: string; preview_url: string }>(`/api/personalization/uploads/${mediaId}/complete`, {
        method: 'POST',
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
