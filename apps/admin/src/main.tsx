import {
  StrictMode,
  type FormEvent,
  type PointerEvent as ReactPointerEvent,
  type ReactNode,
  useEffect,
  useMemo,
  useRef,
  useState,
} from 'react'
import { createRoot } from 'react-dom/client'
import {
  QueryClient,
  QueryClientProvider,
  useMutation,
  useQuery,
  useQueryClient,
} from '@tanstack/react-query'
import {
  Eye,
  ImageUp,
  Archive,
  ArchiveRestore,
  BadgePercent,
  CircleCheck,
  Boxes,
  History,
  LayoutDashboard,
  LoaderCircle,
  LockKeyhole,
  LogOut,
  Mail,
  MapPin,
  Package,
  ReceiptText,
  Phone,
  Plus,
  Search,
  Send,
  ShieldCheck,
  SlidersHorizontal,
  TriangleAlert,
  UsersRound,
  UserRoundX,
} from 'lucide-react'
import {
  ApiError,
  createApiClient,
  type CustomerDetail,
  type CustomerSummary,
  type CommercialSettings,
  type Discount,
  type Product,
  type InventoryRecord,
  type Order,
  type OrderSummary,
  type StaffProfile,
  type StaffRecord,
} from '@knitprint/api-client'
import './styles.css'

const api = createApiClient()
const profileKey = ['staff-profile'] as const
const categoriesKey = ['categories'] as const
const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      retry: false,
      staleTime: 30_000,
    },
  },
})

function App() {
  const profile = useQuery({
    queryKey: profileKey,
    queryFn: api.profile,
  })

  if (profile.isPending) {
    return (
      <main className="auth-loading" aria-live="polite">
        <LoaderCircle aria-hidden="true" />
        <p>Checking your session…</p>
      </main>
    )
  }

  if (profile.isError) {
    const unavailable =
      profile.error instanceof ApiError && profile.error.status !== 401
    return (
      <LoginScreen
        serviceMessage={
          unavailable
            ? 'The admin service is currently unavailable. Check that the API and database are running.'
            : undefined
        }
      />
    )
  }

  return <AdminShell profile={profile.data} />
}

function LoginScreen({
  serviceMessage,
}: Readonly<{ serviceMessage?: string }>) {
  const client = useQueryClient()
  const login = useMutation({
    mutationFn: api.login,
    onSuccess: (profile) => {
      client.setQueryData(profileKey, profile)
    },
  })

  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    const form = new FormData(event.currentTarget)
    login.mutate({
      email: String(form.get('email') ?? ''),
      password: String(form.get('password') ?? ''),
    })
  }

  const errorMessage =
    login.error instanceof ApiError && login.error.status === 401
      ? 'The email or password is incorrect.'
      : login.isError
        ? 'Sign in could not be completed. Please try again.'
        : serviceMessage

  return (
    <main className="login-page">
      <section className="login-card" aria-labelledby="login-heading">
        <div className="login-brand">
          <img
            src="/knitprint-wordmark.webp"
            alt="KnitPrint"
            width="750"
            height="195"
          />
          <span>Admin</span>
        </div>
        <div className="login-intro">
          <div className="login-icon">
            <LockKeyhole aria-hidden="true" />
          </div>
          <p className="login-eyebrow">Private workspace</p>
          <h1 id="login-heading">Welcome back.</h1>
          <p>Sign in to manage the KnitPrint store.</p>
        </div>
        <form onSubmit={submit}>
          <label htmlFor="email">Email address</label>
          <input
            id="email"
            name="email"
            type="email"
            autoComplete="username"
            required
            disabled={login.isPending}
          />
          <div className="password-label">
            <label htmlFor="password">Password</label>
          </div>
          <input
            id="password"
            name="password"
            type="password"
            autoComplete="current-password"
            required
            disabled={login.isPending}
          />
          {errorMessage && (
            <p className="login-error" role="alert">
              {errorMessage}
            </p>
          )}
          <button className="login-submit" type="submit" disabled={login.isPending}>
            {login.isPending ? (
              <>
                <LoaderCircle className="spinner" aria-hidden="true" />
                Signing in…
              </>
            ) : (
              'Sign in'
            )}
          </button>
        </form>
        <p className="login-help">Access is restricted to authorized staff.</p>
      </section>
      <aside className="login-art" aria-hidden="true">
        <div className="login-thread" />
        <div className="login-object">KP</div>
        <p>Shape the shop.<br />Keep the craft.</p>
      </aside>
    </main>
  )
}

function AdminShell({ profile }: Readonly<{ profile: StaffProfile }>) {
  const availablePages = [
    { id: 'dashboard', label: 'Dashboard', icon: LayoutDashboard },
    ...(profile.capabilities.includes('orders.read')
      ? [{ id: 'orders', label: 'Orders', icon: ReceiptText }]
      : []),
    ...(profile.capabilities.includes('catalog.read')
      ? [{ id: 'products', label: 'Products', icon: Package }]
      : []),
    ...(profile.capabilities.includes('inventory.adjust')
      ? [{ id: 'inventory', label: 'Inventory', icon: Boxes }]
      : []),
    ...(profile.capabilities.includes('customers.read')
      ? [{ id: 'customers', label: 'Customers', icon: UsersRound }]
      : []),
    ...(profile.capabilities.includes('discounts.manage')
      ? [{ id: 'discounts', label: 'Discounts', icon: BadgePercent }]
      : []),
    ...(profile.capabilities.includes('settings.manage')
      ? [{ id: 'settings', label: 'Settings', icon: SlidersHorizontal }]
      : []),
    ...(profile.capabilities.includes('staff.manage')
      ? [{ id: 'staff', label: 'Staff', icon: ShieldCheck }]
      : []),
  ] as const
  type PageId = (typeof availablePages)[number]['id']
  const targetFromHash = () => {
    const [requested, entityId] = window.location.hash.slice(1).split('/')
    return {
      page: availablePages.find((page) => page.id === requested)?.id ?? 'dashboard',
      entityId: entityId || undefined,
    }
  }
  const [target, setTarget] = useState<{ page: PageId; entityId?: string }>(targetFromHash)
  const page = target.page
  useEffect(() => {
    const changePage = () => setTarget(targetFromHash())
    window.addEventListener('hashchange', changePage)
    return () => window.removeEventListener('hashchange', changePage)
  }, [])
  const client = useQueryClient()
  const logout = useMutation({
    mutationFn: api.logout,
    onSuccess: () => {
      client.resetQueries({ queryKey: profileKey })
    },
  })
  const initials = profile.display_name
    .split(/\s+/)
    .map((part) => part[0])
    .join('')
    .slice(0, 2)
    .toUpperCase()

  return (
    <div className="admin-shell">
      <aside className="admin-sidebar">
        <div className="admin-brand">
          <img src="/knitprint-wordmark.webp" alt="KnitPrint" />
          <span>Admin</span>
        </div>
        <nav aria-label="Admin navigation">
          {availablePages.map(({ icon: Icon, id, label }) => (
            <a
              aria-current={page === id ? 'page' : undefined}
              className={page === id ? 'active' : ''}
              href={`#${id}`}
              key={id}
            >
              <Icon size={18} /> {label}
            </a>
          ))}
        </nav>
        <div className="admin-user">
          <span>{initials}</span>
          <div>
            <strong>{profile.display_name}</strong>
            <small>{profile.email}</small>
          </div>
          <button
            type="button"
            onClick={() => logout.mutate()}
            disabled={logout.isPending}
            aria-label="Sign out"
          >
            <LogOut size={17} />
          </button>
        </div>
      </aside>
      <main className="admin-main">
        <header>
          <div>
            <small>
              {new Intl.DateTimeFormat('en', {
                weekday: 'long',
                day: 'numeric',
                month: 'long',
              }).format(new Date())}
            </small>
            <h1>
              {page === 'dashboard'
                ? `Good to see you, ${profile.display_name.split(' ')[0]}.`
                : page === 'orders'
                  ? 'Order operations.'
                : page === 'products'
                  ? 'Product catalog.'
                  : page === 'inventory'
                    ? 'Inventory control.'
                    : page === 'customers'
                      ? 'Customer directory.'
                      : page === 'discounts'
                        ? 'Discount codes.'
                        : page === 'settings'
                          ? 'Store settings.'
                          : 'Staff access.'}
            </h1>
          </div>
          <a className="storefront-link" href="http://localhost:3000">
            View storefront
          </a>
        </header>
        {page === 'dashboard' && <Dashboard profile={profile} />}
        {page === 'orders' && profile.capabilities.includes('orders.read') && (
          <OrderManagement
            canRecordPayment={profile.capabilities.includes('orders.fulfill')}
            canRefund={profile.capabilities.includes('orders.refund')}
            initialOrderId={target.entityId}
          />
        )}
        {page === 'products' &&
          profile.capabilities.includes('catalog.read') && (
          <CatalogManagement
            canUpload={profile.capabilities.includes('media.upload')}
            canWrite={profile.capabilities.includes('catalog.write')}
          />
        )}
        {page === 'inventory' &&
          profile.capabilities.includes('inventory.adjust') && (
            <InventoryManagement initialVariantId={target.entityId} />
          )}
        {page === 'customers' &&
          profile.capabilities.includes('customers.read') && (
            <CustomerManagement />
          )}
        {page === 'discounts' &&
          profile.capabilities.includes('discounts.manage') && (
            <DiscountManagement />
          )}
        {page === 'settings' &&
          profile.capabilities.includes('settings.manage') && (
            <SettingsManagement />
          )}
        {page === 'staff' && profile.capabilities.includes('staff.manage') && (
          <StaffManagement currentStaffId={profile.id} />
        )}
      </main>
    </div>
  )
}

const ordersKey = ['orders'] as const

function orderDate(value: string) {
  return new Intl.DateTimeFormat('en', {
    day: 'numeric',
    month: 'short',
    year: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  }).format(new Date(value))
}

function OrderManagement({
  canRecordPayment,
  canRefund,
  initialOrderId,
}: Readonly<{ canRecordPayment: boolean; canRefund: boolean; initialOrderId?: string }>) {
  const client = useQueryClient()
  const [selectedId, setSelectedId] = useState<string | null>(initialOrderId ?? null)
  useEffect(() => {
    if (initialOrderId) setSelectedId(initialOrderId)
  }, [initialOrderId])
  const [queueOnly, setQueueOnly] = useState(false)
  const orders = useQuery({ queryKey: ordersKey, queryFn: api.listOrders })
  const detail = useQuery({
    queryKey: ['order', selectedId],
    queryFn: () => api.order(selectedId ?? ''),
    enabled: Boolean(selectedId),
  })
  const payment = useMutation({
    mutationFn: ({ id, reason }: { id: string; reason: string }) =>
      api.recordManualPayment(id, { reason }),
    onSuccess: (order) => {
      client.setQueryData(['order', order.id], order)
      client.invalidateQueries({ queryKey: ordersKey })
    },
  })
  const fulfillment = useMutation({
    mutationFn: ({
      id,
      input,
    }: {
      id: string
      input: Parameters<typeof api.createFulfillment>[1]
    }) => api.createFulfillment(id, input, crypto.randomUUID()),
    onSuccess: (order) => {
      client.setQueryData(['order', order.id], order)
      client.invalidateQueries({ queryKey: ordersKey })
    },
  })
  const cancellation = useMutation({
    mutationFn: ({ id, reason, internalNote }: { id: string; reason: string; internalNote: string }) =>
      api.cancelOrder(id, { reason, internal_note: internalNote }, crypto.randomUUID()),
    onSuccess: (order) => {
      client.setQueryData(['order', order.id], order)
      client.invalidateQueries({ queryKey: ordersKey })
    },
  })
  const refund = useMutation({
    mutationFn: ({
      id,
      input,
    }: {
      id: string
      input: Parameters<typeof api.createRefund>[1]
    }) => api.createRefund(id, input, crypto.randomUUID()),
    onSuccess: (order) => {
      client.setQueryData(['order', order.id], order)
      client.invalidateQueries({ queryKey: ordersKey })
    },
  })

  function recordPayment(order: Order) {
    const reason = window.prompt(
      `Record manual payment for ${order.order_number}. Why was it accepted?`,
    )
    if (reason?.trim()) payment.mutate({ id: order.id, reason: reason.trim() })
  }

  function fulfillOrder(event: FormEvent<HTMLFormElement>, order: Order) {
    event.preventDefault()
    const form = new FormData(event.currentTarget)
    const lines = order.lines
      .map((line) => ({
        order_line_id: line.id,
        quantity: Number(form.get(`quantity-${line.id}`) ?? 0),
      }))
      .filter((line) => line.quantity > 0)
    fulfillment.mutate({
      id: order.id,
      input: {
        carrier: String(form.get('carrier') ?? ''),
        tracking_number: String(form.get('tracking_number') ?? ''),
        tracking_url: String(form.get('tracking_url') ?? ''),
        reason: String(form.get('reason') ?? ''),
        lines,
      },
    })
  }

  function cancelOrder(order: Order) {
    const reason = window.prompt(`Cancel ${order.order_number}. Give the customer-facing reason.`)
    if (!reason?.trim()) return
    const internalNote = window.prompt('Optional internal note (not shown to the customer).') ?? ''
    cancellation.mutate({ id: order.id, reason: reason.trim(), internalNote: internalNote.trim() })
  }

  function refundOrder(event: FormEvent<HTMLFormElement>, order: Order) {
    event.preventDefault()
    const form = new FormData(event.currentTarget)
    const mode = String(form.get('refund_mode') ?? 'partial')
    const lines = mode === 'partial'
      ? order.lines
          .map((line) => ({
            order_line_id: line.id,
            quantity: Number(form.get(`refund-quantity-${line.id}`) ?? 0),
          }))
          .filter((line) => line.quantity > 0)
      : []
    refund.mutate({
      id: order.id,
      input: {
        mode,
        lines,
        restock: form.get('restock') === 'on',
        reason: String(form.get('refund_reason') ?? ''),
        internal_note: String(form.get('refund_internal_note') ?? ''),
      },
    })
  }

  const visibleOrders = useMemo(
    () =>
      queueOnly
        ? orders.data?.filter(
            (order) =>
              order.payment_status === 'paid' &&
              order.fulfillment_status !== 'fulfilled',
          )
        : orders.data,
    [orders.data, queueOnly],
  )

  return (
    <section className="orders-section" id="orders" aria-labelledby="orders-heading">
      <div className="section-heading">
        <div>
          <p>Phase 9 · Cancellations & refunds</p>
          <h2 id="orders-heading">Orders</h2>
        </div>
        <div className="order-queue-controls" aria-label="Order queue">
          <button type="button" aria-pressed={queueOnly} onClick={() => setQueueOnly(true)}>Needs fulfillment</button>
          <button type="button" aria-pressed={!queueOnly} onClick={() => setQueueOnly(false)}>All orders</button>
        </div>
      </div>
      <div className="orders-layout">
        <div className="order-list">
          {orders.isPending && <p className="panel-message">Loading orders…</p>}
          {orders.isError && <p className="panel-message error" role="alert">Orders could not be loaded.</p>}
          {visibleOrders?.length === 0 && (
            <div className="order-empty"><ReceiptText aria-hidden="true" /><strong>{queueOnly ? 'Fulfillment queue is clear' : 'No orders yet'}</strong><span>{queueOnly ? 'Paid orders awaiting shipment will appear here.' : 'Completed cart checkouts will appear here.'}</span></div>
          )}
          {visibleOrders?.map((order: OrderSummary) => (
            <button
              className={selectedId === order.id ? 'selected' : ''}
              type="button"
              key={order.id}
              aria-pressed={selectedId === order.id}
              onClick={() => setSelectedId(order.id)}
            >
              <span className="order-list-number"><strong>{order.order_number}</strong><small>{orderDate(order.created_at)}</small></span>
              <span className="order-list-customer"><strong>{order.customer_name}</strong><small>{order.customer_email}</small></span>
              <span className={`order-state ${order.payment_status}`}>{order.payment_status}</span>
              <b>{formatMoney(order.total_minor, order.currency)}</b>
            </button>
          ))}
        </div>
        <OrderDetail
          order={detail.data}
          loading={detail.isPending && Boolean(selectedId)}
          error={detail.isError}
          canRecordPayment={canRecordPayment}
          canRefund={canRefund}
          paymentPending={payment.isPending}
          paymentError={payment.isError}
          onRecordPayment={recordPayment}
          fulfillmentPending={fulfillment.isPending}
          fulfillmentError={fulfillment.isError}
          onFulfill={fulfillOrder}
          cancellationPending={cancellation.isPending}
          cancellationError={cancellation.isError}
          onCancel={cancelOrder}
          refundPending={refund.isPending}
          refundError={refund.isError}
          onRefund={refundOrder}
        />
      </div>
    </section>
  )
}

function OrderDetail({
  order,
  loading,
  error,
  canRecordPayment,
  canRefund,
  paymentPending,
  paymentError,
  onRecordPayment,
  fulfillmentPending,
  fulfillmentError,
  onFulfill,
  cancellationPending,
  cancellationError,
  onCancel,
  refundPending,
  refundError,
  onRefund,
}: Readonly<{
  order?: Order
  loading: boolean
  error: boolean
  canRecordPayment: boolean
  canRefund: boolean
  paymentPending: boolean
  paymentError: boolean
  onRecordPayment: (order: Order) => void
  fulfillmentPending: boolean
  fulfillmentError: boolean
  onFulfill: (event: FormEvent<HTMLFormElement>, order: Order) => void
  cancellationPending: boolean
  cancellationError: boolean
  onCancel: (order: Order) => void
  refundPending: boolean
  refundError: boolean
  onRefund: (event: FormEvent<HTMLFormElement>, order: Order) => void
}>) {
  if (loading) return <aside className="order-detail"><p className="panel-message">Loading order…</p></aside>
  if (error) return <aside className="order-detail"><p className="panel-message error" role="alert">The order could not be loaded.</p></aside>
  if (!order) return <aside className="order-detail order-detail-empty"><ReceiptText aria-hidden="true" /><strong>Select an order</strong><span>Commercial snapshots and its timeline will appear here.</span></aside>
  return (
    <aside className="order-detail" aria-label={`Order ${order.order_number}`}>
      <div className="order-detail-heading">
        <div><p>Order</p><h3>{order.order_number}</h3><span>{orderDate(order.created_at)}</span></div>
        <strong>{formatMoney(order.total_minor, order.currency)}</strong>
      </div>
      <div className="order-status-grid">
        <div><span>Order</span><b>{order.order_status}</b></div>
        <div><span>Payment</span><b>{order.payment_status}</b></div>
        <div><span>Fulfillment</span><b>{order.fulfillment_status}</b></div>
      </div>
      {order.discount && (
        <p className="panel-message">
          Discount {order.discount.code}: −{formatMoney(order.discount.amount_minor, order.discount.currency)}
        </p>
      )}
      <div className="order-commercial-summary">
        <span>Subtotal <b>{formatMoney(order.subtotal_minor, order.currency)}</b></span>
        <span>{order.shipping.method_name} <b>{formatMoney(order.shipping_minor, order.currency)}</b></span>
        <span>Tax ({order.tax.rate_basis_points / 100}%) <b>{formatMoney(order.tax_minor, order.currency)}</b></span>
      </div>
      {canRecordPayment && order.payment.provider === 'manual' && order.payment_status === 'pending' && (
        <button className="primary-button" type="button" disabled={paymentPending} onClick={() => onRecordPayment(order)}>
          {paymentPending ? 'Recording…' : 'Record manual payment'}
        </button>
      )}
      {paymentError && <p className="panel-error" role="alert">The payment could not be recorded.</p>}
      {canRefund && order.operations.can_cancel && (
        <section className="order-detail-section order-operation">
          <h4>Cancel order</h4>
          <p>This releases reserved stock. Paid or shipped orders cannot use this operation.</p>
          <button className="danger-button" type="button" disabled={cancellationPending} onClick={() => onCancel(order)}>
            {cancellationPending ? 'Cancelling…' : 'Cancel order'}
          </button>
          {cancellationError && <p className="panel-error" role="alert">The order could not be cancelled. Its state was left unchanged.</p>}
        </section>
      )}
      <section className="order-detail-section">
        <h4>Payment activity</h4>
        <p className="panel-message">
          Provider: {order.payment.provider} · {order.payment.status}
        </p>
        {order.payment.attempts.map((attempt) => (
          <div className="order-line" key={attempt.id}>
            <span>
              <strong>Attempt {attempt.attempt_number}</strong>
              <small>{attempt.provider}{attempt.expires_at ? ` · expires ${orderDate(attempt.expires_at)}` : ''}</small>
            </span>
            <b>{attempt.status}</b>
          </div>
        ))}
        {order.payment.history.length > 0 && (
          <ol className="order-timeline">
            {order.payment.history.map((event) => (
              <li key={event.id}>
                <span aria-hidden="true" />
                <div>
                  <strong>{event.event_type}</strong>
                  {event.detail && <small>{event.detail}</small>}
                  <time dateTime={event.created_at}>{orderDate(event.created_at)} · {event.provider_status}</time>
                </div>
              </li>
            ))}
          </ol>
        )}
      </section>
      <section className="order-detail-section">
        <h4>Items</h4>
        {order.lines.map((line) => (
          <div className="order-line order-line--customizable" key={line.id}>
            {line.customization_media_asset_id && <a href={`/api/admin/personalization/media/${line.customization_media_asset_id}/detail`} target="_blank" rel="noreferrer"><img className="order-customization-image" src={`/api/admin/personalization/media/${line.customization_media_asset_id}/thumbnail`} alt="Fotografia enviada pelo cliente" /></a>}
            <span><strong>{line.product_title}</strong><small>{line.variant_title} · {line.sku} · Qty {line.quantity} · Shipped {line.fulfilled_quantity}</small>{Boolean(line.customization) && <small className="order-customization-summary">Personalização: {customizationLabel(line.customization)}</small>}</span>
            <b>{formatMoney(line.line_total_minor, line.currency)}</b>
          </div>
        ))}
      </section>
      {canRecordPayment && order.payment_status === 'paid' && order.fulfillment_status !== 'fulfilled' && (
        <form className="fulfillment-form order-detail-section" onSubmit={(event) => onFulfill(event, order)}>
          <h4>Create fulfillment</h4>
          <p>Select quantities for this shipment. Tracking is optional.</p>
          {order.lines.map((line) => {
            const remaining = line.quantity - line.fulfilled_quantity
            return remaining > 0 ? (
              <label key={line.id}>
                <span>{line.product_title} · {line.variant_title}</span>
                <input name={`quantity-${line.id}`} type="number" min="0" max={remaining} defaultValue={remaining} aria-label={`${line.product_title} quantity to ship`} />
              </label>
            ) : null
          })}
          <label>Carrier<input name="carrier" maxLength={100} placeholder="CTT" /></label>
          <label>Tracking number<input name="tracking_number" maxLength={200} /></label>
          <label>Tracking URL<input name="tracking_url" type="url" placeholder="https://…" /></label>
          <label>Internal reason<textarea name="reason" minLength={3} maxLength={500} required defaultValue="Packed and dispatched" /></label>
          <button className="primary-button" disabled={fulfillmentPending}>
            {fulfillmentPending ? 'Creating shipment…' : 'Create shipment'}
          </button>
          {fulfillmentError && <p className="panel-error" role="alert">The shipment could not be created. Check remaining quantities and tracking details.</p>}
        </form>
      )}
      {order.fulfillments.length > 0 && (
        <section className="order-detail-section">
          <h4>Fulfillment history</h4>
          {order.fulfillments.map((fulfillment) => (
            <article className="fulfillment-record" key={fulfillment.id}>
              <strong>{fulfillment.carrier || 'Shipment recorded'}</strong>
              <time dateTime={fulfillment.created_at}>{orderDate(fulfillment.created_at)}</time>
              {fulfillment.tracking_number && (
                fulfillment.tracking_url
                  ? <a href={fulfillment.tracking_url} target="_blank" rel="noreferrer">{fulfillment.tracking_number}</a>
                  : <span>{fulfillment.tracking_number}</span>
              )}
              <small>{fulfillment.lines.map((line) => `${line.quantity} × ${line.product_title}`).join(', ')}</small>
            </article>
          ))}
        </section>
      )}
      {canRefund && order.operations.can_refund && (
        <form className="fulfillment-form order-detail-section" onSubmit={(event) => onRefund(event, order)}>
          <h4>Create refund</h4>
          <p>The server calculates the amount from the immutable order lines. Refundable balance: {formatMoney(order.operations.refundable_minor, order.currency)}.</p>
          <fieldset className="refund-mode">
            <legend>Refund scope</legend>
            <label><input type="radio" name="refund_mode" value="partial" defaultChecked /> Partial</label>
            <label><input type="radio" name="refund_mode" value="full" /> Full remaining balance</label>
          </fieldset>
          {order.lines.map((line) => (
            <label key={line.id}>
              <span>{line.product_title} · {line.variant_title}</span>
              <input name={`refund-quantity-${line.id}`} type="number" min="0" max={line.quantity} defaultValue="0" aria-label={`${line.product_title} quantity to refund`} />
            </label>
          ))}
          <label className="check-row"><input name="restock" type="checkbox" /> Return selected quantities to available stock</label>
          <label>Customer-facing reason<textarea name="refund_reason" minLength={3} maxLength={500} required /></label>
          <label>Internal note<textarea name="refund_internal_note" maxLength={2000} /></label>
          <button className="primary-button" disabled={refundPending}>{refundPending ? 'Submitting refund…' : 'Create refund'}</button>
          {refundError && <p className="panel-error" role="alert">The refund could not be completed. Review the eligible balance and quantities.</p>}
        </form>
      )}
      {order.refunds.length > 0 && (
        <section className="order-detail-section">
          <h4>Refund history</h4>
          {order.refunds.map((record) => (
            <article className="fulfillment-record" key={record.id}>
              <strong>{formatMoney(record.amount_minor, record.currency)} · {record.status}</strong>
              <time dateTime={record.created_at}>{orderDate(record.created_at)}</time>
              <span>{record.reason}</span>
              {record.internal_note && <small>Internal: {record.internal_note}</small>}
              <small>{record.mode} refund · {record.restock ? 'Restocked' : 'Not restocked'}{record.actor_display_name ? ` · ${record.actor_display_name}` : ''}</small>
              {record.failure_message && <small className="panel-error">{record.failure_message}</small>}
            </article>
          ))}
        </section>
      )}
      {order.notifications.length > 0 && (
        <section className="order-detail-section">
          <h4>Customer notifications</h4>
          {order.notifications.map((notification) => (
            <div className="order-line" key={notification.id}>
              <span><strong>{notification.kind.replaceAll('_', ' ')}</strong><small>{notification.last_error ?? `Created ${orderDate(notification.created_at)}`}</small></span>
              <b>{notification.status}</b>
            </div>
          ))}
        </section>
      )}
      <section className="order-detail-section order-contact">
        <h4>Customer & delivery</h4>
        <strong>{order.customer.first_name} {order.customer.last_name}</strong>
        <a href={`mailto:${order.customer.email}`}>{order.customer.email}</a>
        <address>
          {order.shipping_address.recipient_name}<br />
          {order.shipping_address.line1}<br />
          {order.shipping_address.line2 && <>{order.shipping_address.line2}<br /></>}
          {order.shipping_address.postal_code} {order.shipping_address.city}<br />
          {order.shipping_address.country_code}
        </address>
      </section>
      <section className="order-detail-section">
        <h4>Timeline</h4>
        <ol className="order-timeline">
          {order.timeline.map((event) => (
            <li key={event.id}>
              <span aria-hidden="true" />
              <div><strong>{event.title}</strong><small>{event.detail}</small><time dateTime={event.created_at}>{orderDate(event.created_at)}{event.actor_display_name ? ` · ${event.actor_display_name}` : ''}</time></div>
            </li>
          ))}
        </ol>
      </section>
    </aside>
  )
}

function customizationLabel(value: unknown) {
  if (!value || typeof value !== 'object') return 'configuração guardada'
  const text = (value as { text?: { content?: unknown }; photo?: unknown }).text?.content
  const parts = [(value as { photo?: unknown }).photo ? 'fotografia' : '', typeof text === 'string' ? `texto “${text}”` : ''].filter(Boolean)
  return parts.join(' + ') || 'configuração guardada'
}

const discountsKey = ['discounts'] as const

function DiscountManagement() {
  const client = useQueryClient()
  const [kind, setKind] = useState<'percentage' | 'fixed'>('percentage')
  const discounts = useQuery({ queryKey: discountsKey, queryFn: api.listDiscounts })
  const createDiscount = useMutation({
    mutationFn: api.createDiscount,
    onSuccess: (discount) => {
      client.setQueryData<Discount[]>(discountsKey, (current = []) => [discount, ...current])
    },
  })
  const status = useMutation({
    mutationFn: ({ id, enabled, reason }: { id: string; enabled: boolean; reason: string }) =>
      api.changeDiscountStatus(id, { enabled, reason }),
    onSuccess: (discount) => {
      client.setQueryData<Discount[]>(discountsKey, (current = []) =>
        current.map((item) => item.id === discount.id ? discount : item),
      )
    },
  })

  function optionalDate(value: FormDataEntryValue | null) {
    const text = String(value ?? '').trim()
    return text ? new Date(text).toISOString() : null
  }

  function optionalPositive(value: FormDataEntryValue | null) {
    const number = Number(value ?? 0)
    return number > 0 ? number : null
  }

  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    const form = new FormData(event.currentTarget)
    const enteredValue = Number(form.get('discount-value') ?? 0)
    createDiscount.mutate({
      code: String(form.get('discount-code') ?? ''),
      kind,
      value: Math.round(enteredValue * 100),
      currency: String(form.get('discount-currency') ?? 'EUR'),
      minimum_order_minor: Math.round(Number(form.get('discount-minimum') ?? 0) * 100),
      starts_at: optionalDate(form.get('discount-starts')),
      ends_at: optionalDate(form.get('discount-ends')),
      usage_limit: optionalPositive(form.get('discount-usage-limit')),
      per_customer_limit: optionalPositive(form.get('discount-customer-limit')),
      reason: String(form.get('discount-reason') ?? ''),
    })
  }

  function toggle(discount: Discount) {
    const enabled = discount.status !== 'active'
    const reason = window.prompt(
      `${enabled ? 'Enable' : 'Disable'} ${discount.code}. Why is this changing?`,
    )
    if (reason?.trim()) status.mutate({ id: discount.id, enabled, reason: reason.trim() })
  }

  return (
    <section className="discounts-section" aria-labelledby="discounts-heading">
      <div className="section-heading">
        <div><p>Phase 10 · Pricing</p><h2 id="discounts-heading">Discounts</h2></div>
      </div>
      <div className="discounts-layout">
        <form className="discount-form" onSubmit={submit}>
          <h3>Create discount code</h3>
          <label>Code<input name="discount-code" minLength={3} maxLength={32} required placeholder="WELCOME10" /></label>
          <label>Type<select name="discount-kind" value={kind} onChange={(event) => setKind(event.target.value as 'percentage' | 'fixed')}><option value="percentage">Percentage</option><option value="fixed">Fixed amount</option></select></label>
          <label>{kind === 'percentage' ? 'Percentage' : 'Amount'}<input name="discount-value" type="number" min="0.01" max={kind === 'percentage' ? '100' : undefined} step="0.01" required /></label>
          <label>Currency<input name="discount-currency" pattern="[A-Za-z]{3}" defaultValue="EUR" required /></label>
          <label>Minimum order amount<input name="discount-minimum" type="number" min="0" step="0.01" defaultValue="0" /></label>
          <div className="discount-field-row">
            <label>Starts<input name="discount-starts" type="datetime-local" /></label>
            <label>Ends<input name="discount-ends" type="datetime-local" /></label>
          </div>
          <div className="discount-field-row">
            <label>Global usage limit<input name="discount-usage-limit" type="number" min="1" /></label>
            <label>Per-customer limit<input name="discount-customer-limit" type="number" min="1" /></label>
          </div>
          <label>Audit reason<textarea name="discount-reason" minLength={3} maxLength={500} required defaultValue="New storefront promotion" /></label>
          <button className="primary-button" disabled={createDiscount.isPending}>{createDiscount.isPending ? 'Creating…' : 'Create discount'}</button>
          {createDiscount.isError && <p className="panel-error" role="alert">The discount could not be created. Check the code, dates, value, and limits.</p>}
        </form>
        <div className="discount-list">
          {discounts.isPending && <p className="panel-message">Loading discounts…</p>}
          {discounts.isError && <p className="panel-error" role="alert">Discounts could not be loaded.</p>}
          {discounts.data?.length === 0 && <div className="order-empty"><BadgePercent aria-hidden="true" /><strong>No discount codes yet</strong><span>Create a bounded promotion when the store needs one.</span></div>}
          {discounts.data?.map((discount) => (
            <article key={discount.id} className="discount-record">
              <div><strong>{discount.code}</strong><span className={`order-state ${discount.status}`}>{discount.status}</span></div>
              <p>{discount.kind === 'percentage' ? `${discount.value / 100}% off` : `${formatMoney(discount.value, discount.currency)} off`} · minimum {formatMoney(discount.minimum_order_minor, discount.currency)}</p>
              <small>{discount.usage_count}{discount.usage_limit ? ` / ${discount.usage_limit}` : ''} uses{discount.per_customer_limit ? ` · ${discount.per_customer_limit} per customer` : ''}</small>
              {(discount.starts_at || discount.ends_at) && <small>{discount.starts_at ? `Starts ${orderDate(discount.starts_at)}` : 'Active immediately'} · {discount.ends_at ? `Ends ${orderDate(discount.ends_at)}` : 'No end date'}</small>}
              <button type="button" disabled={status.isPending} onClick={() => toggle(discount)}>{discount.status === 'active' ? 'Disable' : 'Enable'}</button>
            </article>
          ))}
        </div>
      </div>
    </section>
  )
}

const settingsKey = ['commercial-settings'] as const

function settingsInput(settings: CommercialSettings) {
  return {
    store_name: settings.store_name,
    support_email: settings.support_email,
    currency: settings.currency,
    tax_enabled: settings.tax_enabled,
    shipping_zones: settings.shipping_zones.map((zone) => ({
      name: zone.name,
      country_codes: zone.country_codes,
      active: zone.active,
      methods: zone.methods.map((method) => ({
        name: method.name,
        flat_rate_minor: method.flat_rate_minor,
        active: method.active,
      })),
    })),
    tax_rules: settings.tax_rules.map((rule) => ({
      name: rule.name,
      country_codes: rule.country_codes,
      rate_basis_points: rule.rate_basis_points,
      active: rule.active,
    })),
  }
}

function countryList(value: FormDataEntryValue | null) {
  return String(value ?? '')
    .split(',')
    .map((country) => country.trim().toUpperCase())
    .filter(Boolean)
}

function SettingsManagement() {
  const client = useQueryClient()
  const settings = useQuery({ queryKey: settingsKey, queryFn: api.settings })
  const updateSettings = useMutation({
    mutationFn: api.updateSettings,
    onSuccess: (next) => client.setQueryData(settingsKey, next),
  })

  function update(
    changes: Partial<ReturnType<typeof settingsInput>>,
    reason: string,
  ) {
    if (!settings.data) return
    updateSettings.mutate({
      ...settingsInput(settings.data),
      ...changes,
      reason,
    })
  }

  function saveIdentity(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    const form = new FormData(event.currentTarget)
    update(
      {
        store_name: String(form.get('store-name') ?? ''),
        support_email: String(form.get('support-email') ?? ''),
        currency: String(form.get('store-currency') ?? ''),
        tax_enabled: form.get('tax-enabled') === 'on',
      },
      String(form.get('settings-reason') ?? ''),
    )
  }

  function addZone(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    if (!settings.data) return
    const form = new FormData(event.currentTarget)
    const current = settingsInput(settings.data)
    update(
      {
        shipping_zones: [
          ...current.shipping_zones,
          {
            name: String(form.get('zone-name') ?? ''),
            country_codes: countryList(form.get('zone-countries')),
            active: true,
            methods: [{
              name: String(form.get('method-name') ?? ''),
              flat_rate_minor: Math.round(Number(form.get('shipping-rate') ?? 0) * 100),
              active: true,
            }],
          },
        ],
      },
      String(form.get('zone-reason') ?? ''),
    )
  }

  function removeZone(index: number, name: string) {
    if (!settings.data || settings.data.shipping_zones.length <= 1) return
    const reason = window.prompt(`Remove shipping zone ${name}. Why is this changing?`)
    if (!reason?.trim()) return
    const current = settingsInput(settings.data)
    update(
      { shipping_zones: current.shipping_zones.filter((_, position) => position !== index) },
      reason.trim(),
    )
  }

  function addShippingMethod(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    if (!settings.data) return
    const form = new FormData(event.currentTarget)
    const zoneId = String(form.get('shipping-method-zone') ?? '')
    const zoneIndex = settings.data.shipping_zones.findIndex((zone) => zone.id === zoneId)
    if (zoneIndex < 0) return
    const current = settingsInput(settings.data)
    current.shipping_zones[zoneIndex].methods.push({
      name: String(form.get('shipping-method-name') ?? ''),
      flat_rate_minor: Math.round(Number(form.get('shipping-method-rate') ?? 0) * 100),
      active: true,
    })
    update(
      { shipping_zones: current.shipping_zones },
      String(form.get('shipping-method-reason') ?? ''),
    )
  }

  function removeShippingMethod(zoneIndex: number, methodIndex: number, name: string) {
    if (!settings.data || settings.data.shipping_zones[zoneIndex].methods.length <= 1) return
    const reason = window.prompt(`Remove shipping method ${name}. Why is this changing?`)
    if (!reason?.trim()) return
    const current = settingsInput(settings.data)
    current.shipping_zones[zoneIndex].methods = current.shipping_zones[zoneIndex].methods.filter(
      (_, position) => position !== methodIndex,
    )
    update({ shipping_zones: current.shipping_zones }, reason.trim())
  }

  function addTaxRule(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    if (!settings.data) return
    const form = new FormData(event.currentTarget)
    const current = settingsInput(settings.data)
    update(
      {
        tax_rules: [
          ...current.tax_rules,
          {
            name: String(form.get('tax-name') ?? ''),
            country_codes: countryList(form.get('tax-countries')),
            rate_basis_points: Math.round(Number(form.get('tax-rate') ?? 0) * 100),
            active: true,
          },
        ],
      },
      String(form.get('tax-reason') ?? ''),
    )
  }

  function removeTaxRule(index: number, name: string) {
    if (!settings.data) return
    const reason = window.prompt(`Remove tax rule ${name}. Why is this changing?`)
    if (!reason?.trim()) return
    const current = settingsInput(settings.data)
    update(
      { tax_rules: current.tax_rules.filter((_, position) => position !== index) },
      reason.trim(),
    )
  }

  return (
    <section className="settings-section" aria-labelledby="settings-heading">
      <div className="section-heading">
        <div><p>Phase 11 · Commercial configuration</p><h2 id="settings-heading">Settings</h2></div>
      </div>
      {settings.isPending && <p className="panel-message">Loading commercial settings…</p>}
      {settings.isError && <p className="panel-error" role="alert">Settings could not be loaded.</p>}
      {settings.data && (
        <div className="settings-layout">
          <div className="settings-column">
            <form className="settings-card" key={settings.data.updated_at} onSubmit={saveIdentity}>
              <h3>Store identity</h3>
              <label>Store name<input name="store-name" defaultValue={settings.data.store_name} minLength={2} maxLength={100} required /></label>
              <label>Support email<input name="support-email" type="email" defaultValue={settings.data.support_email} required /></label>
              <label>Store currency<input name="store-currency" defaultValue={settings.data.currency} pattern="[A-Za-z]{3}" required /></label>
              <label className="check-row"><input name="tax-enabled" type="checkbox" defaultChecked={settings.data.tax_enabled} /> Enable destination tax calculation</label>
              <label>Audit reason<textarea name="settings-reason" minLength={3} maxLength={500} defaultValue="Update store identity and pricing behavior" required /></label>
              <button className="primary-button" disabled={updateSettings.isPending}>Save store settings</button>
            </form>

            <div className="settings-card">
              <h3>Shipping zones</h3>
              {settings.data.shipping_zones.map((zone, index) => (
                <article className="settings-record" key={zone.id}>
                  <div><strong>{zone.name}</strong><span>{zone.country_codes.length ? zone.country_codes.join(', ') : 'Worldwide fallback'}</span></div>
                  {zone.methods.map((method, methodIndex) => (
                    <div className="settings-method-row" key={method.id}>
                      <small>{method.name} · {formatMoney(method.flat_rate_minor, method.currency)}</small>
                      <button type="button" disabled={zone.methods.length <= 1 || updateSettings.isPending} onClick={() => removeShippingMethod(index, methodIndex, method.name)}>Remove method</button>
                    </div>
                  ))}
                  <button type="button" disabled={settings.data.shipping_zones.length <= 1 || updateSettings.isPending} onClick={() => removeZone(index, zone.name)}>Remove zone</button>
                </article>
              ))}
              <form className="settings-inline-form" onSubmit={addZone}>
                <h4>Add shipping zone</h4>
                <label>Zone name<input name="zone-name" required placeholder="Portugal" /></label>
                <label>Countries<input name="zone-countries" placeholder="PT, ES" /><small>Comma-separated ISO codes. Leave empty only for one worldwide fallback.</small></label>
                <label>Initial method name<input name="method-name" required placeholder="Standard tracked" /></label>
                <label>Flat rate ({settings.data.currency})<input name="shipping-rate" type="number" min="0" step="0.01" defaultValue="0" required /></label>
                <label>Audit reason<textarea name="zone-reason" minLength={3} maxLength={500} defaultValue="Add a shipping destination and method" required /></label>
                <button disabled={updateSettings.isPending}>Add zone</button>
              </form>
              <form className="settings-inline-form" onSubmit={addShippingMethod}>
                <h4>Add shipping method</h4>
                <label>Shipping zone<select name="shipping-method-zone" required>{settings.data.shipping_zones.map((zone) => <option value={zone.id} key={zone.id}>{zone.name}</option>)}</select></label>
                <label>Additional method name<input name="shipping-method-name" required placeholder="Express tracked" /></label>
                <label>Flat rate ({settings.data.currency})<input name="shipping-method-rate" type="number" min="0" step="0.01" required /></label>
                <label>Audit reason<textarea name="shipping-method-reason" minLength={3} maxLength={500} defaultValue="Add a shipping service level" required /></label>
                <button disabled={updateSettings.isPending}>Add method</button>
              </form>
            </div>
          </div>

          <div className="settings-column">
            <div className="settings-card">
              <h3>Tax rules</h3>
              <p className="settings-note">Rates are exclusive and apply to discounted merchandise plus shipping. Configure them only after confirming the store’s tax obligations.</p>
              {settings.data.tax_rules.length === 0 && <p className="panel-message">No destination tax rules configured.</p>}
              {settings.data.tax_rules.map((rule, index) => (
                <article className="settings-record" key={rule.id}>
                  <div><strong>{rule.name}</strong><span>{rule.rate_basis_points / 100}%</span></div>
                  <small>{rule.country_codes.length ? rule.country_codes.join(', ') : 'Worldwide fallback'}</small>
                  <button type="button" disabled={updateSettings.isPending} onClick={() => removeTaxRule(index, rule.name)}>Remove rule</button>
                </article>
              ))}
              <form className="settings-inline-form" onSubmit={addTaxRule}>
                <h4>Add destination tax rule</h4>
                <label>Rule name<input name="tax-name" required placeholder="Portugal standard rate" /></label>
                <label>Countries<input name="tax-countries" placeholder="PT" required /></label>
                <label>Rate (%)<input name="tax-rate" type="number" min="0" max="100" step="0.01" required /></label>
                <label>Audit reason<textarea name="tax-reason" minLength={3} maxLength={500} defaultValue="Add a confirmed destination tax rate" required /></label>
                <button disabled={updateSettings.isPending}>Add tax rule</button>
              </form>
            </div>

            <div className="settings-card">
              <h3>Integration health</h3>
              <div className="integration-grid">
                {Object.entries(settings.data.integrations).map(([name, status]) => (
                  <div key={name}><span>{name.replaceAll('_', ' ')}</span><strong>{status.replaceAll('_', ' ')}</strong></div>
                ))}
              </div>
              <p className="settings-note">This page reports configuration state only. Credentials and secrets remain in the runtime environment.</p>
            </div>

            <div className="settings-card">
              <h3>Recent settings history</h3>
              {settings.data.history.length === 0 && <p className="panel-message">No settings changes recorded yet.</p>}
              {settings.data.history.map((record) => (
                <article className="settings-history" key={record.id}>
                  <strong>{record.reason}</strong>
                  <small>{orderDate(record.created_at)}{record.actor_display_name ? ` · ${record.actor_display_name}` : ''}</small>
                </article>
              ))}
            </div>
          </div>
          {updateSettings.isError && <p className="panel-error" role="alert">Settings were not changed. Check country overlaps, values, and the audit reason.</p>}
        </div>
      )}
    </section>
  )
}

const customersKey = ['customers'] as const

function customerName(customer: CustomerSummary | CustomerDetail) {
  return `${customer.first_name} ${customer.last_name}`.trim()
}

function formatCustomerDate(value: string) {
  return new Intl.DateTimeFormat('en', {
    day: 'numeric',
    month: 'short',
    year: 'numeric',
  }).format(new Date(value))
}

function CustomerManagement() {
  const [search, setSearch] = useState('')
  const [debouncedSearch, setDebouncedSearch] = useState('')
  const [selectedId, setSelectedId] = useState<string | null>(null)
  useEffect(() => {
    const timeout = window.setTimeout(
      () => setDebouncedSearch(search.trim()),
      250,
    )
    return () => window.clearTimeout(timeout)
  }, [search])
  const customers = useQuery({
    queryKey: [...customersKey, debouncedSearch],
    queryFn: () => api.listCustomers({ q: debouncedSearch || undefined }),
  })
  const detail = useQuery({
    queryKey: ['customer', selectedId],
    queryFn: () => api.customer(selectedId ?? ''),
    enabled: Boolean(selectedId),
  })

  return (
    <section
      className="customers-section"
      id="customers"
      aria-labelledby="customers-heading"
    >
      <div className="section-heading">
        <div>
          <p>Phase 4 · Relationships</p>
          <h2 id="customers-heading">Customers</h2>
        </div>
        <span>{customers.isPending ? '—' : (customers.data?.length ?? 0)} shown</span>
      </div>
      <label className="customer-search">
        <Search size={16} aria-hidden="true" />
        <span className="sr-only">Search customers</span>
        <input
          type="search"
          placeholder="Search by name or email"
          value={search}
          onChange={(event) => setSearch(event.target.value)}
        />
      </label>
      <div className="customer-layout">
        <div className="customer-list">
          {customers.isPending && (
            <p className="panel-message">Loading customers…</p>
          )}
          {customers.isError && (
            <p className="panel-message error" role="alert">
              Customers could not be loaded.
            </p>
          )}
          {customers.data?.map((customer) => (
            <button
              aria-controls="customer-detail"
              aria-pressed={selectedId === customer.id}
              className={selectedId === customer.id ? 'selected' : ''}
              key={customer.id}
              type="button"
              onClick={() => setSelectedId(customer.id)}
            >
              <span className="customer-avatar" aria-hidden="true">
                {customer.first_name.slice(0, 1).toUpperCase()}
                {customer.last_name.slice(0, 1).toUpperCase()}
              </span>
              <span className="customer-identity">
                <strong>{customerName(customer)}</strong>
                <small>{customer.email}</small>
              </span>
              <span className="customer-summary">
                <b>{customer.customer_type}</b>
                <small>
                  {customer.address_count}{' '}
                  {customer.address_count === 1 ? 'address' : 'addresses'}
                </small>
              </span>
            </button>
          ))}
          {customers.isSuccess && customers.data.length === 0 && (
            <div className="customer-empty">
              <UsersRound aria-hidden="true" />
              <strong>No customers found</strong>
              <span>
                {search.trim()
                  ? 'Try a different name or email.'
                  : 'Customer profiles will appear after checkout.'}
              </span>
            </div>
          )}
        </div>
        <div className="customer-detail-shell" id="customer-detail">
          {!selectedId ? (
            <div className="customer-placeholder">
              <UsersRound aria-hidden="true" />
              <strong>Select a customer</strong>
              <span>Review contact details, addresses, and order history.</span>
            </div>
          ) : detail.isPending ? (
            <p className="panel-message">Loading customer details…</p>
          ) : detail.isError ? (
            <p className="panel-message error" role="alert">
              Customer details could not be loaded.
            </p>
          ) : (
            <CustomerDetailPanel customer={detail.data} />
          )}
        </div>
      </div>
    </section>
  )
}

function CustomerDetailPanel({
  customer,
}: Readonly<{ customer: CustomerDetail }>) {
  const name = customerName(customer)

  return (
    <section className="customer-detail" aria-labelledby="customer-detail-heading">
      <div className="customer-detail-heading">
        <div className="customer-avatar large" aria-hidden="true">
          {customer.first_name.slice(0, 1).toUpperCase()}
          {customer.last_name.slice(0, 1).toUpperCase()}
        </div>
        <div>
          <p>{customer.customer_type} customer</p>
          <h3 id="customer-detail-heading">{name}</h3>
          <small>Customer since {formatCustomerDate(customer.created_at)}</small>
        </div>
      </div>
      <section className="customer-contact" aria-labelledby="customer-contact-heading">
        <h4 id="customer-contact-heading">Contact</h4>
        <dl>
          <div>
            <dt><Mail aria-hidden="true" /> Email</dt>
            <dd><a href={`mailto:${customer.email}`}>{customer.email}</a></dd>
          </div>
          <div>
            <dt><Phone aria-hidden="true" /> Phone</dt>
            <dd>{customer.phone || 'Not provided'}</dd>
          </div>
        </dl>
      </section>
      <section className="customer-addresses" aria-labelledby="customer-addresses-heading">
        <h4 id="customer-addresses-heading">
          Addresses <span>{customer.addresses.length}</span>
        </h4>
        {customer.addresses.map((address) => (
          <article key={address.id} aria-label={`Address for ${address.recipient_name}`}>
            <MapPin aria-hidden="true" />
            <address>
              <strong>{address.recipient_name}</strong>
              <span>{address.line1}</span>
              {address.line2 && <span>{address.line2}</span>}
              <span>
                {[address.city, address.region, address.postal_code]
                  .filter(Boolean)
                  .join(', ')}
              </span>
              <span>{address.country_code}</span>
            </address>
            <small>{address.address_type}</small>
          </article>
        ))}
        {customer.addresses.length === 0 && (
          <p className="panel-message">No saved addresses.</p>
        )}
      </section>
      <section className="customer-orders" aria-labelledby="customer-orders-heading">
        <div>
          <History aria-hidden="true" />
          <h4 id="customer-orders-heading">Order history</h4>
          <span>{customer.order_count}</span>
        </div>
        {customer.order_count === 0 && (
          <p>No orders yet. Completed checkouts will appear here.</p>
        )}
      </section>
      <p className="customer-retention">
        Profile retained until {formatCustomerDate(customer.retention_expires_at)}.
      </p>
    </section>
  )
}

const inventoryKey = ['inventory'] as const

function Dashboard({ profile }: Readonly<{ profile: StaffProfile }>) {
  const dashboard = useQuery({
    queryKey: ['operational-dashboard'],
    queryFn: api.dashboard,
    staleTime: 0,
    refetchOnMount: 'always',
  })
  const data = dashboard.data
  const metric = (value?: number | null) => value ?? '—'
  const definition = (key: string) =>
    data?.definitions.find((item) => item.key === key)?.description ?? ''

  return (
    <>
      <section className="welcome">
        <div>
          <p>Operational workspace</p>
          <h2>Your store’s next actions, in one place.</h2>
          <span>
            Signed in as {profile.role}. Metrics use UTC and link directly to
            the records that need attention.
          </span>
        </div>
        <div className="welcome-mark">KP</div>
      </section>
      {dashboard.isPending && <p className="panel-message dashboard-loading">Loading operations…</p>}
      {dashboard.isError && <p className="panel-error dashboard-loading" role="alert">Operational data could not be loaded.</p>}
      {data && (
        <>
          <section className="metrics operational-metrics" aria-label="Operational metrics">
            {data.access.orders && (
              <>
                <article><span>Orders today</span><strong>{metric(data.metrics.orders_today)}</strong><small>{metric(data.metrics.orders_total)} total orders</small><a href="#orders">Review orders</a></article>
                <article><span>Net revenue</span><strong>{formatMoney(data.metrics.net_revenue_minor ?? 0, data.currency)}</strong><small>{formatMoney(data.metrics.gross_revenue_minor ?? 0, data.currency)} captured · {formatMoney(data.metrics.refunds_minor ?? 0, data.currency)} refunded</small><a href="#orders">Review payments</a></article>
                <article><span>Awaiting fulfillment</span><strong>{metric(data.metrics.paid_awaiting_fulfillment)}</strong><small>{definition('paid_awaiting_fulfillment')}</small><a href="#orders">Open queue</a></article>
                <article><span>Failed payments</span><strong>{metric(data.metrics.failed_payments)}</strong><small>{definition('failed_payments')}</small><a href="#orders">Inspect failures</a></article>
              </>
            )}
            {data.access.inventory && (
              <article><span>Low-stock variants</span><strong>{metric(data.metrics.low_stock_variants)}</strong><small>{definition('low_stock_variants')}</small><a href="#inventory">Review stock</a></article>
            )}
          </section>

          <div className="dashboard-grid">
            {data.access.orders && (
              <DashboardPanel title="Paid orders awaiting fulfillment" eyebrow="Fulfillment queue" href="#orders" empty="No paid orders are waiting for fulfillment.">
                {data.paid_awaiting_fulfillment.map((order) => (
                  <a className="dashboard-row" href={`#orders/${order.id}`} key={order.id}>
                    <Send aria-hidden="true" />
                    <span><strong>{order.order_number} · {order.customer_name}</strong><small>{order.payment_status} · {order.fulfillment_status} · {orderDate(order.created_at)}</small></span>
                    <b>{formatMoney(order.total_minor, order.currency)}</b>
                  </a>
                ))}
              </DashboardPanel>
            )}

            {data.access.orders && (
              <DashboardPanel title="Recent orders" eyebrow="Latest activity" href="#orders" empty="No orders have been created yet.">
                {data.recent_orders.map((order) => (
                  <a className="dashboard-row" href={`#orders/${order.id}`} key={order.id}>
                    <ReceiptText aria-hidden="true" />
                    <span><strong>{order.order_number} · {order.customer_name}</strong><small>{order.payment_status} · {orderDate(order.created_at)}</small></span>
                    <b>{formatMoney(order.total_minor, order.currency)}</b>
                  </a>
                ))}
              </DashboardPanel>
            )}

            {data.access.inventory && (
              <DashboardPanel title="Low-stock variants" eyebrow="Inventory attention" href="#inventory" empty="Stock levels are above every configured threshold.">
                {data.low_stock_variants.map((record) => (
                  <a className="dashboard-row" href={`#inventory/${record.variant_id}`} key={record.variant_id}>
                    <TriangleAlert aria-hidden="true" />
                    <span><strong>{record.product_title} · {record.variant_title}</strong><small>{record.sku} · threshold {record.low_stock_threshold}</small></span>
                    <b>{record.available_quantity}</b>
                  </a>
                ))}
              </DashboardPanel>
            )}

            {data.access.orders && (
              <DashboardPanel title="Failed payments" eyebrow="Payment attention" href="#orders" empty="No payment is currently failed.">
                {data.failed_payments.map((payment) => (
                  <a className="dashboard-row" href={`#orders/${payment.order_id}`} key={payment.order_id}>
                    <TriangleAlert aria-hidden="true" />
                    <span><strong>{payment.order_number} · {payment.customer_name}</strong><small>{payment.failure_message} · {orderDate(payment.updated_at)}</small></span>
                    <b>{formatMoney(payment.amount_minor, payment.currency)}</b>
                  </a>
                ))}
              </DashboardPanel>
            )}

            {data.access.orders && (
              <DashboardPanel title="Recent refunds" eyebrow="Returns & refunds" href="#orders" empty="No refunds have been requested yet.">
                {data.recent_refunds.map((refund) => (
                  <a className="dashboard-row" href={`#orders/${refund.order_id}`} key={refund.id}>
                    <History aria-hidden="true" />
                    <span><strong>{refund.order_number} · {refund.status}</strong><small>{refund.reason} · {orderDate(refund.created_at)}</small></span>
                    <b>{formatMoney(refund.amount_minor, refund.currency)}</b>
                  </a>
                ))}
              </DashboardPanel>
            )}
          </div>

          <details className="metric-definitions">
            <summary>Metric definitions · generated {orderDate(data.generated_at)} · {data.timezone}</summary>
            <dl>{data.definitions.map((item) => <div key={item.key}><dt>{item.key.replaceAll('_', ' ')}</dt><dd>{item.description}</dd></div>)}</dl>
          </details>
        </>
      )}
    </>
  )
}

function DashboardPanel({
  title,
  eyebrow,
  href,
  empty,
  children,
}: Readonly<{
  title: string
  eyebrow: string
  href: string
  empty: string
  children: ReactNode
}>) {
  const hasItems = Array.isArray(children) ? children.length > 0 : Boolean(children)
  return (
    <section className="dashboard-panel" aria-label={title}>
      <div className="dashboard-panel-heading"><div><p>{eyebrow}</p><h2>{title}</h2></div><a href={href}>View all</a></div>
      {hasItems ? <div className="dashboard-list">{children}</div> : <div className="dashboard-empty"><CircleCheck aria-hidden="true" /><span>{empty}</span></div>}
    </section>
  )
}

type InventoryFilter = 'all' | 'attention' | 'out' | 'healthy'

function InventoryManagement({ initialVariantId }: Readonly<{ initialVariantId?: string }>) {
  const client = useQueryClient()
  const [selected, setSelected] = useState<InventoryRecord | null>(null)
  const [query, setQuery] = useState('')
  const [filter, setFilter] = useState<InventoryFilter>('all')
  const inventory = useQuery({ queryKey: inventoryKey, queryFn: api.listInventory })
  const records = inventory.data ?? []
  useEffect(() => {
    if (!initialVariantId || !inventory.data) return
    const record = inventory.data.find((item) => item.variant_id === initialVariantId)
    if (record) setSelected(record)
  }, [initialVariantId, inventory.data])
  const counts = {
    all: records.length,
    attention: records.filter(({ low_stock }) => low_stock).length,
    out: records.filter(({ available_quantity }) => available_quantity === 0).length,
    healthy: records.filter(({ low_stock }) => !low_stock).length,
  }
  const visibleInventory = useMemo(() => {
    const normalized = query.trim().toLowerCase()
    return records.filter((record) => {
      const matchesQuery =
        !normalized ||
        [record.product_title, record.variant_title, record.sku].some(
          (value) => value.toLowerCase().includes(normalized),
        )
      const matchesFilter =
        filter === 'all' ||
        (filter === 'attention' && record.low_stock) ||
        (filter === 'out' && record.available_quantity === 0) ||
        (filter === 'healthy' && !record.low_stock)
      return matchesQuery && matchesFilter
    })
  }, [filter, query, records])
  const movements = useQuery({
    queryKey: ['inventory-movements', selected?.variant_id],
    queryFn: () => api.inventoryMovements(selected?.variant_id ?? ''),
    enabled: Boolean(selected),
  })
  const adjust = useMutation({
    mutationFn: ({
      delta,
      reason,
      threshold,
      variantId,
    }: {
      delta: number
      reason: string
      threshold?: number
      variantId: string
    }) =>
      api.adjustInventory(variantId, {
        quantity_delta: delta,
        reason,
        low_stock_threshold: threshold,
      }),
    onSuccess: (record) => {
      setSelected(record)
      client.invalidateQueries({ queryKey: inventoryKey })
      client.invalidateQueries({ queryKey: ['inventory-movements', record.variant_id] })
    },
  })

  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    if (!selected) return
    const formElement = event.currentTarget
    const form = new FormData(formElement)
    const threshold = String(form.get('stock-threshold') ?? '').trim()
    adjust.mutate(
      {
        variantId: selected.variant_id,
        delta: Number(form.get('stock-delta')),
        reason: String(form.get('stock-reason') ?? ''),
        threshold: threshold ? Number(threshold) : undefined,
      },
      { onSuccess: () => formElement.reset() },
    )
  }

  return (
    <section className="inventory-section" aria-labelledby="inventory-heading">
      <div className="section-heading">
        <div><p>Phase 3 · Operations</p><h2 id="inventory-heading">Inventory</h2></div>
        <span>{inventory.isPending ? '—' : counts.attention} low stock</span>
      </div>
      <div className="inventory-toolbar">
        <label className="inventory-search">
          <Search size={16} aria-hidden="true" />
          <span className="sr-only">Search inventory</span>
          <input
            type="search"
            placeholder="Search product, variant, or SKU"
            value={query}
            onChange={(event) => setQuery(event.target.value)}
          />
        </label>
        <div
          className="inventory-filters"
          role="group"
          aria-label="Filter by stock state"
        >
          {([
            ['all', 'All'],
            ['attention', 'Needs attention'],
            ['out', 'Out of stock'],
            ['healthy', 'Healthy'],
          ] as const).map(([value, label]) => (
            <button
              aria-pressed={filter === value}
              className={filter === value ? 'active' : ''}
              key={value}
              type="button"
              onClick={() => setFilter(value)}
            >
              {label} <span>{counts[value]}</span>
            </button>
          ))}
        </div>
      </div>
      <div className="inventory-layout">
        <div className="inventory-list">
          {inventory.isPending && <p className="panel-message">Loading inventory…</p>}
          {inventory.isError && <p className="panel-message error" role="alert">Inventory could not be loaded.</p>}
          {visibleInventory.map((record) => (
            <button
              className={selected?.variant_id === record.variant_id ? 'selected' : ''}
              key={record.variant_id}
              type="button"
              onClick={() => setSelected(record)}
            >
              <span><strong>{record.product_title}</strong><small>{record.variant_title} · {record.sku}</small></span>
              <span className={record.low_stock ? 'stock-count low' : 'stock-count'}>
                <strong>{record.available_quantity}</strong><small>available</small>
              </span>
            </button>
          ))}
          {inventory.isSuccess && records.length === 0 && <p className="panel-message">Create a product variant to begin tracking stock.</p>}
          {inventory.isSuccess && records.length > 0 && visibleInventory.length === 0 && (
            <p className="panel-message">No inventory matches the current search and stock filter.</p>
          )}
        </div>
        <div className="inventory-editor">
          {!selected ? (
            <div className="inventory-placeholder"><Boxes aria-hidden="true" /><strong>Select a variant</strong><span>Adjust stock and inspect its immutable history.</span></div>
          ) : (
            <>
              <div className="panel-title"><Boxes size={17} /><div><strong>{selected.product_title} · {selected.variant_title}</strong><span>{selected.available_quantity} available · low at {selected.low_stock_threshold}</span></div></div>
              <form className="compact-form" onSubmit={submit}>
                <label htmlFor="stock-delta">Quantity change</label>
                <input id="stock-delta" name="stock-delta" type="number" step="1" placeholder="Use + to add or − to remove" required />
                <label htmlFor="stock-reason">Reason</label>
                <textarea id="stock-reason" name="stock-reason" rows={3} minLength={3} maxLength={500} required />
                <label htmlFor="stock-threshold">Low-stock threshold (optional)</label>
                <input id="stock-threshold" name="stock-threshold" type="number" min="0" step="1" />
                {adjust.isError && <p className="panel-error" role="alert">{adjust.error.message}</p>}
                <button className="primary-button" disabled={adjust.isPending}>{adjust.isPending ? 'Saving…' : 'Apply adjustment'}</button>
              </form>
              <div className="movement-history">
                <div className="variant-heading">Movement history</div>
                {movements.isPending && <p className="panel-message">Loading history…</p>}
                {movements.data?.map((movement) => (
                  <article key={movement.id}>
                    <strong>{movement.quantity_delta > 0 ? '+' : ''}{movement.quantity_delta}</strong>
                    <span>{movement.reason}<small>{typeof movement.actor_display_name === 'string' ? movement.actor_display_name : 'System'} · {new Date(movement.created_at).toLocaleString()}</small></span>
                    <b>{movement.resulting_available_quantity}</b>
                  </article>
                ))}
                {movements.data?.length === 0 && <p className="panel-message">No adjustments yet.</p>}
              </div>
            </>
          )}
        </div>
      </div>
    </section>
  )
}

const productsKey = ['admin-products'] as const
const GOOGLE_FONT_OPTIONS = ['Roboto', 'Montserrat', 'Playfair Display', 'Dancing Script', 'Pacifico'] as const
const PERSONALIZATION_COLOR_OPTIONS = [
  { value: '#111111', label: 'Preto' }, { value: '#ffffff', label: 'Branco' },
  { value: '#9c5263', label: 'Rosa antigo' }, { value: '#1f4f78', label: 'Azul' },
  { value: '#b3232f', label: 'Vermelho' },
] as const

type PrintArea = { x: number; y: number; width: number; height: number }

function EditablePrintArea({ area, label, kind, active, onActivate, onChange }: Readonly<{
  area: PrintArea
  label: string
  kind: 'photo' | 'text'
  active: boolean
  onActivate: () => void
  onChange: (area: PrintArea) => void
}>) {
  const areaElement = useRef<HTMLDivElement>(null)
  const interaction = useRef<{ startX: number; startY: number; area: PrintArea; handle: string } | undefined>(undefined)
  function start(event: ReactPointerEvent<HTMLElement>, handle: string) {
    event.preventDefault()
    onActivate()
    event.currentTarget.setPointerCapture(event.pointerId)
    interaction.current = { startX: event.clientX, startY: event.clientY, area, handle }
  }
  function move(event: ReactPointerEvent<HTMLElement>) {
    const active = interaction.current
    const bounds = areaElement.current?.parentElement?.getBoundingClientRect()
    if (!active || !bounds) return
    const dx = (event.clientX - active.startX) / bounds.width * 100
    const dy = (event.clientY - active.startY) / bounds.height * 100
    let { x, y, width, height } = active.area
    if (active.handle === 'move') {
      x = Math.max(0, Math.min(100 - width, x + dx)); y = Math.max(0, Math.min(100 - height, y + dy))
    } else {
      if (active.handle.includes('w')) { const next = Math.max(0, Math.min(x + width - 5, x + dx)); width += x - next; x = next }
      if (active.handle.includes('e')) width = Math.max(5, Math.min(100 - x, width + dx))
      if (active.handle.includes('n')) { const next = Math.max(0, Math.min(y + height - 5, y + dy)); height += y - next; y = next }
      if (active.handle.includes('s')) height = Math.max(5, Math.min(100 - y, height + dy))
    }
    onChange({ x, y, width, height })
  }
  return <div ref={areaElement} className={`editable-print-area editable-print-area--${kind}${active ? ' editable-print-area--active' : ''}`} aria-label={`Zona de ${label}`} aria-current={active ? 'true' : undefined} style={{ left: `${area.x}%`, top: `${area.y}%`, width: `${area.width}%`, height: `${area.height}%` }} onPointerDown={(event) => start(event, 'move')} onPointerMove={move} onPointerUp={() => { interaction.current = undefined }}>
    <span>{label}</span>
    {(['nw', 'ne', 'sw', 'se'] as const).map((handle) => <button key={handle} type="button" className={`resize-handle resize-handle--${handle}`} aria-label={`Redimensionar zona de ${label}`} onPointerDown={(event) => { event.stopPropagation(); start(event, handle) }} onPointerMove={move} onPointerUp={() => { interaction.current = undefined }}>{handle === 'nw' ? '↖' : handle === 'ne' ? '↗' : handle === 'sw' ? '↙' : '↘'}</button>)}
  </div>
}

function CatalogManagement({
  canUpload,
  canWrite,
}: Readonly<{ canUpload: boolean; canWrite: boolean }>) {
  const client = useQueryClient()
  const [search, setSearch] = useState('')
  const [preview, setPreview] = useState<Product | null>(null)
  const [productTitle, setProductTitle] = useState('')
  const [productSlug, setProductSlug] = useState('')
  const [productSlugEdited, setProductSlugEdited] = useState(false)
  const [productDescription, setProductDescription] = useState('')
  const [productKeywords, setProductKeywords] = useState('')
  const [productSku, setProductSku] = useState('')
  const [productPrice, setProductPrice] = useState('')
  const [productQuantity, setProductQuantity] = useState('0')
  const [productCategoryIds, setProductCategoryIds] = useState<string[]>([])
  const [personalizationMode, setPersonalizationMode] = useState<'none' | 'photo' | 'text' | 'photo_text'>('none')
  const [photoArea, setPhotoArea] = useState<PrintArea>({ x: 25, y: 25, width: 50, height: 50 })
  const [textArea, setTextArea] = useState<PrintArea>({ x: 25, y: 65, width: 50, height: 20 })
  const [activePrintArea, setActivePrintArea] = useState<'photo' | 'text'>('text')
  const [previewMediaId, setPreviewMediaId] = useState<string>()
  const [textMaxCharacters, setTextMaxCharacters] = useState(35)
  const [textMinSize, setTextMinSize] = useState(12)
  const [textMaxSize, setTextMaxSize] = useState(72)
  const [allowedFonts, setAllowedFonts] = useState('Roboto, Montserrat, Playfair Display, Dancing Script, Pacifico')
  const [allowedColors, setAllowedColors] = useState('#111111, #ffffff, #9c5263, #1f4f78, #b3232f')
  const personalization = {
    mode: personalizationMode,
    preview_media_id: previewMediaId,
    area_x: Math.round(photoArea.x * 100), area_y: Math.round(photoArea.y * 100),
    area_width: Math.round(photoArea.width * 100), area_height: Math.round(photoArea.height * 100),
    text_area_x: Math.round(textArea.x * 100), text_area_y: Math.round(textArea.y * 100),
    text_area_width: Math.round(textArea.width * 100), text_area_height: Math.round(textArea.height * 100),
    text_max_characters: textMaxCharacters, text_min_size: textMinSize, text_max_size: textMaxSize,
    allowed_fonts: allowedFonts.split(',').map((value) => value.trim()).filter(Boolean),
    allowed_colors: allowedColors.split(',').map((value) => value.trim()).filter(Boolean),
  }
  const personalizationPreviewMedia = preview?.media.find(({ id }) => id === previewMediaId) ?? preview?.media[0]
  const editorDirty = useMemo(() => {
    if (!preview) {
      return Boolean(productTitle || productSlug || productDescription || productKeywords || productSku || productPrice || productQuantity !== '0')
    }
    const base = preview.variants[0]
    return productTitle !== preview.title
      || productSlug !== preview.slug
      || productDescription !== preview.description
      || productKeywords !== preview.search_keywords
      || productSku !== (base?.sku ?? '')
      || Number(productPrice) !== (base?.price_minor ?? 0) / 100
      || Number(productQuantity) !== (base?.available_quantity ?? 0)
  }, [preview, productTitle, productSlug, productDescription, productKeywords, productSku, productPrice, productQuantity])
  useEffect(() => {
    function warnBeforeLeaving(event: BeforeUnloadEvent) {
      if (!editorDirty) return
      event.preventDefault()
    }
    window.addEventListener('beforeunload', warnBeforeLeaving)
    return () => window.removeEventListener('beforeunload', warnBeforeLeaving)
  }, [editorDirty])
  const products = useQuery({
    queryKey: [...productsKey, search],
    queryFn: () => api.listAdminProducts({ q: search }),
  })
  const categories = useQuery({
    queryKey: categoriesKey,
    queryFn: api.listCategories,
  })
  const createProduct = useMutation({
    mutationFn: async ({ categoryIds, ...input }: Parameters<typeof api.createProduct>[0] & { categoryIds: string[] }) => {
      const product = await api.createProduct(input)
      return categoryIds.length > 0
        ? api.assignProductCategories(product.id, { category_ids: categoryIds })
        : product
    },
    onMutate: async () => {
      await client.cancelQueries({ queryKey: productsKey })
    },
    onSuccess: (product) => {
      client.setQueriesData<Array<Product>>(
        { queryKey: productsKey },
        (current = []) =>
          current.some(({ id }) => id === product.id)
            ? current
            : [product, ...current],
      )
      client.invalidateQueries({ queryKey: productsKey })
      client.invalidateQueries({ queryKey: inventoryKey })
      loadProduct(product)
    },
  })
  const updateProduct = useMutation({
    mutationFn: async ({ id, categoryIds, ...input }: Parameters<typeof api.updateProduct>[1] & { id: string; categoryIds: string[] }) => {
      const product = await api.updateProduct(id, input)
      return api.assignProductCategories(product.id, { category_ids: categoryIds })
    },
    onSuccess: (product) => {
      client.invalidateQueries({ queryKey: productsKey })
      client.invalidateQueries({ queryKey: inventoryKey })
      setPreview(product)
      loadProduct(product)
    },
  })
  const deleteProduct = useMutation({
    mutationFn: api.deleteProduct,
    onSuccess: () => {
      client.invalidateQueries({ queryKey: productsKey })
      client.invalidateQueries({ queryKey: inventoryKey })
      clearEditor()
    },
  })
  const changeStatus = useMutation({
    mutationFn: ({
      id,
      status,
    }: {
      id: string
      status: 'active' | 'archived'
    }) => api.changeProductStatus(id, { status }),
    onSuccess: (product) => {
      client.invalidateQueries({ queryKey: productsKey })
      setPreview(product)
    },
  })
  const addVariant = useMutation({
    mutationFn: ({
      productId,
      price,
      sku,
      title,
      currency,
    }: {
      productId: string
      price: number
      sku: string
      title: string
      currency: string
    }) =>
      api.addProductVariant(productId, {
        title,
        sku,
        price_minor: Math.round(price * 100),
        currency,
        option_values: {},
      }),
    onSuccess: (product) => {
      client.invalidateQueries({ queryKey: productsKey })
      client.invalidateQueries({ queryKey: inventoryKey })
      setPreview(product)
    },
  })
  const createCategory = useMutation({
    mutationFn: api.createCategory,
    onSuccess: () => client.invalidateQueries({ queryKey: categoriesKey }),
  })
  const assignCategories = useMutation({
    mutationFn: ({ productId, categoryIds }: { productId: string; categoryIds: string[] }) =>
      api.assignProductCategories(productId, { category_ids: categoryIds }),
    onSuccess: (product) => {
      client.invalidateQueries({ queryKey: productsKey })
      setPreview(product)
    },
  })
  const uploadImage = useMutation({
    mutationFn: async ({
      images,
      product,
    }: {
      images: Array<{ altText: string; file: File }>
      product: Product
    }) => {
      for (const { altText, file } of images) {
        const upload = await api.initiateMediaUpload({
          filename: file.name,
          content_type: file.type,
          byte_size: file.size,
        })
        await api.uploadMediaObject(upload.upload_url, file, file.type)
        await api.completeMediaUpload(upload.id, {
          product_id: product.id,
          alt_text: altText,
        })
      }
      return api.adminProduct(product.id)
    },
    onSuccess: (product) => {
      client.invalidateQueries({ queryKey: productsKey })
      loadProduct(product)
    },
  })

  function selectImages(product: Product, files?: FileList | null) {
    if (!files?.length) return
    const images: Array<{ altText: string; file: File }> = []
    for (const file of Array.from(files)) {
      const altText = window.prompt(
        `Describe ${file.name}, an image of ${product.title}, for customers using assistive technology.`,
      )
      if (altText?.trim()) images.push({ altText: altText.trim(), file })
    }
    if (images.length > 0) uploadImage.mutate({ images, product })
  }

  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    const price = Number(productPrice)
    if (preview) {
      updateProduct.mutate({
        id: preview.id,
        categoryIds: productCategoryIds,
        title: productTitle,
        slug: productSlug,
        description: productDescription,
        search_keywords: productKeywords,
        sku: productSku,
        price_minor: Math.round(price * 100),
        currency: 'EUR',
        available_quantity: Number(productQuantity),
        personalization,
      })
      return
    }
    createProduct.mutate(
      {
        title: productTitle,
        slug: productSlug,
        description: productDescription,
        search_keywords: productKeywords,
        variants: [
          {
            title: 'Default',
            sku: productSku,
            price_minor: Math.round(price * 100),
            currency: 'EUR',
            option_values: {},
            available_quantity: Number(productQuantity),
          },
        ],
        personalization,
        categoryIds: productCategoryIds,
      },
    )
  }

  function loadProduct(product: Product) {
    const base = product.variants[0]
    setPreview(product)
    setProductTitle(product.title)
    setProductSlug(product.slug)
    setProductSlugEdited(true)
    setProductDescription(product.description)
    setProductKeywords(product.search_keywords)
    setProductSku(base?.sku ?? '')
    setProductPrice(base ? String(base.price_minor / 100) : '')
    setProductQuantity(String(base?.available_quantity ?? 0))
    setProductCategoryIds(product.categories.map(({ id }) => id))
    const config = product.personalization
    setPersonalizationMode(config.mode as typeof personalizationMode)
    setActivePrintArea(config.mode === 'photo' ? 'photo' : 'text')
    setPhotoArea({ x: config.area_x / 100, y: config.area_y / 100, width: config.area_width / 100, height: config.area_height / 100 })
    setTextArea({ x: config.text_area_x / 100, y: config.text_area_y / 100, width: config.text_area_width / 100, height: config.text_area_height / 100 })
    setPreviewMediaId(config.preview_media_id ?? product.media[0]?.id)
    setTextMaxCharacters(config.text_max_characters)
    setTextMinSize(config.text_min_size)
    setTextMaxSize(config.text_max_size)
    const configuredFonts = Array.isArray(config.allowed_fonts) ? config.allowed_fonts.filter((font): font is string => GOOGLE_FONT_OPTIONS.includes(font as typeof GOOGLE_FONT_OPTIONS[number])) : []
    const configuredColors = Array.isArray(config.allowed_colors) ? config.allowed_colors.filter((color): color is string => typeof color === 'string' && /^#[0-9a-f]{6}$/i.test(color)) : []
    setAllowedFonts((configuredFonts.length ? configuredFonts : [...GOOGLE_FONT_OPTIONS]).join(', '))
    setAllowedColors((configuredColors.length ? configuredColors : PERSONALIZATION_COLOR_OPTIONS.map(({ value }) => value)).join(', '))
  }

  function clearEditor() {
    setPreview(null)
    setProductTitle('')
    setProductSlug('')
    setProductSlugEdited(false)
    setProductDescription('')
    setProductKeywords('')
    setProductSku('')
    setProductPrice('')
    setProductQuantity('0')
    setProductCategoryIds([])
    setPersonalizationMode('none')
    setActivePrintArea('text')
    setPhotoArea({ x: 25, y: 25, width: 50, height: 50 })
    setTextArea({ x: 25, y: 65, width: 50, height: 20 })
    setPreviewMediaId(undefined)
    setTextMaxCharacters(35)
    setTextMinSize(12)
    setTextMaxSize(72)
    setAllowedFonts('Roboto, Montserrat, Playfair Display, Dancing Script, Pacifico')
    setAllowedColors('#111111, #ffffff, #9c5263, #1f4f78, #b3232f')
  }

  function submitVariant(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    if (!preview) return
    const formElement = event.currentTarget
    const form = new FormData(formElement)
    addVariant.mutate(
      {
        productId: preview.id,
        title: String(form.get('new-variant-title') ?? ''),
        sku: String(form.get('new-variant-sku') ?? ''),
        price: Number(form.get('new-variant-price')),
        currency: String(form.get('new-variant-currency') ?? ''),
      },
      { onSuccess: () => formElement.reset() },
    )
  }

  function submitCategory(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    const formElement = event.currentTarget
    const form = new FormData(formElement)
    createCategory.mutate(
      {
        name: String(form.get('category-name') ?? ''),
        slug: String(form.get('category-slug') ?? ''),
        description: '',
      },
      { onSuccess: () => formElement.reset() },
    )
  }

  function submitAssignments(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    if (!preview) return
    const form = new FormData(event.currentTarget)
    assignCategories.mutate({
      productId: preview.id,
      categoryIds: form.getAll('product-category').map(String),
    })
  }

  return (
    <section
      className="catalog-section"
      id="products"
      aria-labelledby="catalog-heading"
    >
      <div className="section-heading">
        <div>
          <p>Phase 2 · Catalog</p>
          <h2 id="catalog-heading">Products</h2>
        </div>
        <span>{products.data?.length ?? '—'} products</span>
      </div>
      <div className="catalog-toolbar">
        <Search size={16} aria-hidden="true" />
        <label className="sr-only" htmlFor="product-search">
          Search products
        </label>
        <input
          id="product-search"
          type="search"
          placeholder="Search title, description, keywords, or slug"
          value={search}
          onChange={(event) => setSearch(event.target.value)}
        />
      </div>
      <div className="catalog-layout">
        <div className="product-list">
          {products.isPending && <p className="panel-message">Loading products…</p>}
          {products.isError && (
            <p className="panel-message error" role="alert">
              Products could not be loaded.
            </p>
          )}
          {products.data?.length === 0 && (
            <div className="catalog-empty">
              <Package aria-hidden="true" />
              <strong>No products yet</strong>
              <span>Create the first draft using the editor.</span>
            </div>
          )}
          {products.data?.map((product) => (
            <article className="product-row" key={product.id}>
              <div className="product-thumbnail" aria-hidden="true">
                {product.media[0]?.thumbnail_url ? (
                  <img
                    src={product.media[0]?.thumbnail_url}
                    alt=""
                  />
                ) : (
                  'KP'
                )}
              </div>
              <div className="product-identity">
                <div>
                  <strong>{product.title}</strong>
                  <span className={`product-state ${product.status}`}>
                    {product.status}
                  </span>
                </div>
                <span>/{product.slug}</span>
                <small>
                  {formatMoney(
                    product.variants[0]?.price_minor,
                    product.variants[0]?.currency,
                  )}
                  {' · '}
                  {product.variants[0]?.sku ?? 'No SKU'} · {product.variants[0]?.available_quantity ?? 0} in stock
                </small>
              </div>
              <div className="product-actions">
                <button type="button" onClick={() => loadProduct(product)}>
                  <Eye size={15} /> Edit
                </button>
                {canUpload && (
                  <label className="product-upload" aria-label="Add product photos">
                    <ImageUp size={15} /> Photos
                    <input
                      type="file"
                      multiple
                      accept="image/jpeg,image/png,image/webp"
                      disabled={uploadImage.isPending}
                      onChange={(event) => {
                        selectImages(product, event.currentTarget.files)
                        event.currentTarget.value = ''
                      }}
                    />
                  </label>
                )}
                {canWrite && product.status === 'draft' && (
                  <button
                    type="button"
                    disabled={changeStatus.isPending}
                    onClick={() =>
                      changeStatus.mutate({ id: product.id, status: 'active' })
                    }
                  >
                    <Send size={15} /> Publish
                  </button>
                )}
                {canWrite && product.status === 'active' && (
                  <button
                    type="button"
                    disabled={changeStatus.isPending}
                    onClick={() =>
                      changeStatus.mutate({
                        id: product.id,
                        status: 'archived',
                      })
                    }
                  >
                    <Archive size={15} /> Archive
                  </button>
                )}
                {canWrite && product.status === 'archived' && (
                  <button
                    type="button"
                    disabled={changeStatus.isPending}
                    onClick={() =>
                      changeStatus.mutate({
                        id: product.id,
                        status: 'active',
                      })
                    }
                  >
                    <ArchiveRestore size={15} /> Restore
                  </button>
                )}
              </div>
            </article>
          ))}
          {uploadImage.isError && (
            <p className="panel-message error" role="alert">
              {uploadImage.error.message}
            </p>
          )}
        </div>
        {canWrite && (
          <form className="product-form" onSubmit={submit}>
            <div className="panel-title">
              <Plus size={17} />
              <div>
                <strong>{preview ? 'Edit product' : 'New product draft'}</strong>
                <span>Product details, price and available stock.</span>
              </div>
            </div>
            <label htmlFor="product-title">Product title</label>
            <input
              id="product-title"
              name="product-title"
              value={productTitle}
              onChange={(event) => {
                const title = event.target.value
                setProductTitle(title)
                if (!productSlugEdited) setProductSlug(slugify(title))
              }}
              required
            />
            <label htmlFor="product-slug">URL slug</label>
            <input
              id="product-slug"
              name="product-slug"
              pattern="[a-z0-9]+(?:-[a-z0-9]+)*"
              placeholder="woven-planter"
              value={productSlug}
              onChange={(event) => {
                setProductSlug(event.target.value)
                setProductSlugEdited(true)
              }}
              aria-describedby="product-slug-help"
              required
            />
            <small className="field-help" id="product-slug-help">
              Generated from the title. You can edit it before saving.
            </small>
            {productSlug && <small className="url-preview">/products/{productSlug}</small>}
            <label htmlFor="product-description">Description</label>
            <textarea
              id="product-description"
              name="product-description"
              rows={3}
              maxLength={50000}
              value={productDescription}
              onChange={(event) => setProductDescription(event.target.value)}
            />
            <small className="field-count">{productDescription.length.toLocaleString()} / 50,000</small>
            <label htmlFor="product-keywords">Search keywords</label>
            <input id="product-keywords" name="product-keywords" maxLength={2000} value={productKeywords} onChange={(event) => setProductKeywords(event.target.value)} />
            <small className="field-help">Only administrators can see these keywords. They also support storefront search.</small>
            <small className="field-count">{productKeywords.length.toLocaleString()} / 2,000</small>
            <label htmlFor="product-sku">SKU</label>
            <input id="product-sku" name="product-sku" maxLength={120} value={productSku} onChange={(event) => setProductSku(event.target.value)} required />
            <div className="price-fields">
              <div>
                <label htmlFor="product-price">Price</label>
                <input
                  id="product-price"
                  name="product-price"
                  type="number"
                  min="0"
                  step="0.01"
                  value={productPrice}
                  onChange={(event) => setProductPrice(event.target.value)}
                  required
                />
              </div>
              <div>
                <label htmlFor="product-quantity">Stock</label>
                <input id="product-quantity" name="product-quantity" type="number" min="0" step="1" value={productQuantity} onChange={(event) => setProductQuantity(event.target.value)} required />
              </div>
            </div>
            <fieldset className="product-categories">
              <legend>Categories</legend>
              {categories.isPending && <span>Loading categories…</span>}
              {categories.data?.length === 0 && <span>No categories have been created yet.</span>}
              {categories.data?.map((category) => (
                <label key={category.id}>
                  <input
                    type="checkbox"
                    checked={productCategoryIds.includes(category.id)}
                    onChange={(event) => setProductCategoryIds((current) =>
                      event.target.checked
                        ? [...current, category.id]
                        : current.filter((id) => id !== category.id)
                    )}
                  />
                  <span><strong>{category.name}</strong><small>/{category.slug}</small></span>
                </label>
              ))}
            </fieldset>
            <fieldset className="personalization-settings">
              <legend>Personalização</legend>
              <label htmlFor="personalization-mode">O cliente pode adicionar</label>
              <select id="personalization-mode" value={personalizationMode} onChange={(event) => { const mode = event.target.value as typeof personalizationMode; setPersonalizationMode(mode); setActivePrintArea(mode === 'photo' ? 'photo' : 'text') }}>
                <option value="none">Sem personalização</option>
                <option value="photo">Só fotografia</option>
                <option value="text">Só texto</option>
                <option value="photo_text">Fotografia e texto</option>
              </select>
              {personalizationMode !== 'none' && (
                <>
                  <p className="field-help">Escolhe a fotografia que mostra melhor a área a personalizar. Arrasta cada caixa e usa as setas nos cantos para a dimensionar.</p>
                  {preview?.media.length ? <label htmlFor="personalization-preview-media">Fotografia apresentada no editor<select id="personalization-preview-media" value={personalizationPreviewMedia?.id ?? ''} onChange={(event) => setPreviewMediaId(event.target.value)}>{preview.media.map((media, index) => <option key={media.id} value={media.id}>{index === 0 ? 'Fotografia principal' : `Fotografia ${index + 1}`} · {media.alt_text}</option>)}</select></label> : <p className="field-help">Guarda o produto e adiciona fotografias para poderes escolher a vista do editor.</p>}
                  {personalizationMode === 'photo_text' && <div className="print-area-layer-picker" role="group" aria-label="Zona a editar"><span>Editar zona:</span><button type="button" className={activePrintArea === 'text' ? 'active' : ''} aria-pressed={activePrintArea === 'text'} onClick={() => setActivePrintArea('text')}>Texto</button><button type="button" className={activePrintArea === 'photo' ? 'active' : ''} aria-pressed={activePrintArea === 'photo'} onClick={() => setActivePrintArea('photo')}>Fotografia</button></div>}
                  <div className="print-area-preview">
                    {personalizationPreviewMedia ? <div className="print-area-canvas"><img src={personalizationPreviewMedia.detail_url} alt="" />
                      {(personalizationMode === 'photo' || personalizationMode === 'photo_text') && <EditablePrintArea area={photoArea} active={activePrintArea === 'photo'} onActivate={() => setActivePrintArea('photo')} onChange={setPhotoArea} label="Fotografia" kind="photo" />}
                      {(personalizationMode === 'text' || personalizationMode === 'photo_text') && <EditablePrintArea area={textArea} active={activePrintArea === 'text'} onActivate={() => setActivePrintArea('text')} onChange={setTextArea} label="Texto" kind="text" />}
                    </div> : <span>Adiciona uma fotografia para posicionar as zonas.</span>}
                  </div>
                </>
              )}
              {(personalizationMode === 'text' || personalizationMode === 'photo_text') && (
                <div className="text-personalization-settings">
                  <label>Limite de caracteres<input type="number" min="1" max="500" value={textMaxCharacters} onChange={(event) => setTextMaxCharacters(Number(event.target.value))} /></label>
                  <div className="print-area-fields">
                    <label>Tamanho mínimo<input type="number" min="8" max="200" value={textMinSize} onChange={(event) => setTextMinSize(Number(event.target.value))} /></label>
                    <label>Tamanho máximo<input type="number" min={textMinSize} max="300" value={textMaxSize} onChange={(event) => setTextMaxSize(Number(event.target.value))} /></label>
                  </div>
                  <fieldset className="personalization-option-grid"><legend>Tipos de letra disponíveis</legend>{GOOGLE_FONT_OPTIONS.map((font) => { const values = allowedFonts.split(',').map((value) => value.trim()).filter(Boolean); const selected = values.includes(font); return <label key={font} style={{ fontFamily: font }}><input type="checkbox" checked={selected} disabled={selected && values.length === 1} onChange={(event) => setAllowedFonts((current) => { const currentValues = current.split(',').map((value) => value.trim()).filter(Boolean); return (event.target.checked ? [...new Set([...currentValues, font])] : currentValues.filter((value) => value !== font)).join(', ') })} /><b>Ag</b><span>{font}</span></label> })}</fieldset>
                  <fieldset className="personalization-color-grid"><legend>Cores disponíveis</legend>{PERSONALIZATION_COLOR_OPTIONS.map(({ value, label }) => { const values = allowedColors.split(',').map((color) => color.trim().toLowerCase()).filter(Boolean); const selected = values.includes(value); return <label key={value}><input type="checkbox" checked={selected} disabled={selected && values.length === 1} onChange={(event) => setAllowedColors((current) => { const currentValues = current.split(',').map((color) => color.trim()).filter(Boolean); return (event.target.checked ? [...new Set([...currentValues, value])] : currentValues.filter((color) => color.toLowerCase() !== value)).join(', ') })} /><i style={{ background: value }} /><span>{label}</span></label> })}</fieldset>
                </div>
              )}
            </fieldset>
            {(createProduct.isError || updateProduct.isError || deleteProduct.isError) && (
              <p className="panel-error" role="alert">
                {(createProduct.error ?? updateProduct.error ?? deleteProduct.error)?.message ?? 'The product could not be saved.'}
              </p>
            )}
            <button className="primary-button" disabled={createProduct.isPending || updateProduct.isPending}>
              {createProduct.isPending || updateProduct.isPending ? 'Saving…' : preview ? 'Save product' : 'Create draft'}
            </button>
            {preview && <button className="secondary-button" type="button" onClick={clearEditor}>Create another product</button>}
            {preview && <button className="danger-button" type="button" disabled={deleteProduct.isPending} onClick={() => {
              if (window.confirm(`Permanently delete ${preview.title}? Products with sales cannot be deleted.`)) deleteProduct.mutate(preview.id)
            }}>Delete product</button>}
            {preview && (
              <div className="admin-product-photos">
                <div><strong>Product photos</strong><span>{preview.media.length} uploaded</span></div>
                {preview.media.length > 0 && (
                  <div className="admin-product-photo-grid">
                    {preview.media.map((media, index) => (
                      <figure key={media.id}>
                        <img src={media.thumbnail_url} alt={media.alt_text} />
                        <figcaption>{index === 0 ? 'Main photo' : `Photo ${index + 1}`}</figcaption>
                      </figure>
                    ))}
                  </div>
                )}
                <label className="product-upload admin-photo-upload" aria-label="Add product photos">
                  <ImageUp size={15} /> {uploadImage.isPending ? 'Uploading…' : 'Add photos'}
                  <input type="file" multiple accept="image/jpeg,image/png,image/webp" disabled={uploadImage.isPending} onChange={(event) => {
                    selectImages(preview, event.currentTarget.files)
                    event.currentTarget.value = ''
                  }} />
                </label>
              </div>
            )}
          </form>
        )}
      </div>
      {preview && (
        <article className="product-preview" aria-label="Product preview">
          <button
            type="button"
            aria-label="Close product preview"
            onClick={clearEditor}
          >
            ×
          </button>
          {preview.media[0]?.detail_url ? (
            <img
              className="preview-art preview-image"
              src={preview.media[0]?.detail_url}
              alt={preview.media[0]?.alt_text ?? ''}
            />
          ) : (
            <div className="preview-art" aria-hidden="true">
              KP
            </div>
          )}
          <div>
            <p>{preview.status} preview</p>
            <h3>{preview.title}</h3>
            <span>{preview.description || 'No description yet.'}</span>
            <strong>
              {formatMoney(
                preview.variants[0]?.price_minor,
                preview.variants[0]?.currency,
              )}
            </strong>
          </div>
        </article>
      )}
      {preview && canWrite && (
        <div className="catalog-editor" aria-label={`Edit ${preview.title}`}>
          <section>
            <div className="panel-title">
              <Plus size={17} />
              <div>
                <strong>Variants</strong>
                <span>{preview.variants.length} configured for this product.</span>
              </div>
            </div>
            <ul className="variant-list">
              {preview.variants.map((variant) => (
                <li key={variant.id}>
                  <span><strong>{variant.title}</strong><small>{variant.sku}</small></span>
                  <b>{formatMoney(variant.price_minor, variant.currency)}</b>
                </li>
              ))}
            </ul>
            <form className="compact-form" onSubmit={submitVariant}>
              <label htmlFor="new-variant-title">Variant title</label>
              <input id="new-variant-title" name="new-variant-title" required />
              <label htmlFor="new-variant-sku">SKU</label>
              <input id="new-variant-sku" name="new-variant-sku" required />
              <div className="price-fields">
                <div>
                  <label htmlFor="new-variant-price">Price</label>
                  <input id="new-variant-price" name="new-variant-price" type="number" min="0" step="0.01" required />
                </div>
                <div>
                  <label htmlFor="new-variant-currency">Currency</label>
                  <select id="new-variant-currency" name="new-variant-currency" defaultValue="EUR">
                    <option value="EUR">EUR</option><option value="GBP">GBP</option><option value="USD">USD</option>
                  </select>
                </div>
              </div>
              {addVariant.isError && <p className="panel-error" role="alert">{addVariant.error.message}</p>}
              <button className="primary-button" disabled={addVariant.isPending}>Add variant</button>
            </form>
          </section>
          <section>
            <div className="panel-title">
              <Package size={17} />
              <div><strong>Categories</strong><span>Group products for future collections.</span></div>
            </div>
            <form className="category-assignments" key={`${preview.id}-${preview.categories.map(({ id }) => id).join('-')}`} onSubmit={submitAssignments}>
              {categories.isPending && <p className="panel-message">Loading categories…</p>}
              {categories.data?.length === 0 && <p className="panel-message">No categories yet.</p>}
              {categories.data?.map((category) => (
                <label key={category.id}>
                  <input name="product-category" type="checkbox" value={category.id} defaultChecked={preview.categories.some(({ id }) => id === category.id)} />
                  <span><strong>{category.name}</strong><small>/{category.slug}</small></span>
                </label>
              ))}
              {assignCategories.isError && <p className="panel-error" role="alert">{assignCategories.error.message}</p>}
              <button className="primary-button" disabled={assignCategories.isPending}>Save categories</button>
            </form>
            <form className="compact-form new-category-form" onSubmit={submitCategory}>
              <div className="variant-heading">New category</div>
              <label htmlFor="category-name">Name</label>
              <input id="category-name" name="category-name" required />
              <label htmlFor="category-slug">URL slug</label>
              <input id="category-slug" name="category-slug" pattern="[a-z0-9]+(?:-[a-z0-9]+)*" required />
              {createCategory.isError && <p className="panel-error" role="alert">{createCategory.error.message}</p>}
              <button className="primary-button" disabled={createCategory.isPending}>Create category</button>
            </form>
          </section>
        </div>
      )}
    </section>
  )
}

function formatMoney(amount?: number, currency?: string) {
  if (amount === undefined || !currency) return 'No price'
  return new Intl.NumberFormat('en', {
    style: 'currency',
    currency,
  }).format(amount / 100)
}

function slugify(value: string) {
  return value
    .normalize('NFD')
    .replace(/[\u0300-\u036f]/g, '')
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')
}

const staffKey = ['staff'] as const
const assignableCapabilities = [
  ['catalog.read', 'View catalog'],
  ['catalog.write', 'Manage catalog'],
  ['orders.read', 'View orders'],
  ['orders.fulfill', 'Fulfill orders'],
  ['orders.refund', 'Refund orders'],
  ['discounts.manage', 'Manage discounts'],
  ['customers.read', 'View customers'],
  ['inventory.adjust', 'Adjust inventory'],
  ['media.upload', 'Upload media'],
  ['media.review', 'Review media'],
  ['settings.manage', 'Manage settings'],
  ['staff.manage', 'Manage staff'],
] as const

function StaffManagement({
  currentStaffId,
}: Readonly<{ currentStaffId: string }>) {
  const client = useQueryClient()
  const staff = useQuery({ queryKey: staffKey, queryFn: api.listStaff })
  const createStaff = useMutation({
    mutationFn: api.createStaff,
    onSuccess: () => {
      client.invalidateQueries({ queryKey: staffKey })
    },
  })
  const disableStaff = useMutation({
    mutationFn: ({ id, reason }: { id: string; reason: string }) =>
      api.disableStaff(id, { reason }),
    onSuccess: () => {
      client.invalidateQueries({ queryKey: staffKey })
    },
  })

  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    const formElement = event.currentTarget
    const form = new FormData(formElement)
    createStaff.mutate(
      {
        email: String(form.get('staff-email') ?? ''),
        display_name: String(form.get('staff-name') ?? ''),
        password: String(form.get('staff-password') ?? ''),
        capabilities: form.getAll('capabilities').map(String),
      },
      { onSuccess: () => formElement.reset() },
    )
  }

  function disable(member: StaffRecord) {
    const reason = window.prompt(
      `Why are you disabling ${member.display_name}? This is recorded in the audit log.`,
    )
    if (reason?.trim()) {
      disableStaff.mutate({ id: member.id, reason: reason.trim() })
    }
  }

  return (
    <section className="staff-section" id="staff" aria-labelledby="staff-heading">
      <div className="section-heading">
        <div>
          <p>Access control</p>
          <h2 id="staff-heading">Staff accounts</h2>
        </div>
        <span>{staff.data?.filter((member) => !member.disabled).length ?? '—'} active</span>
      </div>
      <div className="staff-layout">
        <div className="staff-list">
          {staff.isPending && <p className="staff-message">Loading staff…</p>}
          {staff.isError && (
            <p className="staff-message error" role="alert">
              Staff accounts could not be loaded.
            </p>
          )}
          {staff.data?.map((member) => (
            <article className={member.disabled ? 'disabled' : ''} key={member.id}>
              <div className="staff-avatar">
                {member.display_name.slice(0, 1).toUpperCase()}
              </div>
              <div className="staff-identity">
                <strong>{member.display_name}</strong>
                <span>{member.email}</span>
                <small>
                  {member.role === 'owner'
                    ? 'Owner · all capabilities'
                    : `${member.capabilities.length} capabilities`}
                </small>
              </div>
              {member.disabled ? (
                <span className="status disabled">Disabled</span>
              ) : member.id === currentStaffId ? (
                <span className="status">You</span>
              ) : (
                <button
                  className="disable-button"
                  type="button"
                  disabled={disableStaff.isPending}
                  onClick={() => disable(member)}
                >
                  <UserRoundX size={15} /> Disable
                </button>
              )}
            </article>
          ))}
        </div>
        <form className="staff-form" onSubmit={submit}>
          <div className="staff-form-title">
            <Plus size={17} />
            <div>
              <strong>Add staff member</strong>
              <span>Create access with only the permissions they need.</span>
            </div>
          </div>
          <label htmlFor="staff-name">Display name</label>
          <input id="staff-name" name="staff-name" required />
          <label htmlFor="staff-email">Email address</label>
          <input id="staff-email" name="staff-email" type="email" required />
          <label htmlFor="staff-password">Temporary password</label>
          <input
            id="staff-password"
            name="staff-password"
            type="password"
            minLength={12}
            required
          />
          <fieldset>
            <legend>Capabilities</legend>
            {assignableCapabilities.map(([value, label]) => (
              <label className="capability" key={value}>
                <input name="capabilities" type="checkbox" value={value} />
                <span>{label}</span>
              </label>
            ))}
          </fieldset>
          {createStaff.isError && (
            <p className="staff-form-error" role="alert">
              {createStaff.error instanceof ApiError
                ? createStaff.error.message
                : 'The account could not be created.'}
            </p>
          )}
          <button className="create-staff-button" disabled={createStaff.isPending}>
            {createStaff.isPending ? 'Creating…' : 'Create staff account'}
          </button>
        </form>
      </div>
    </section>
  )
}

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <App />
    </QueryClientProvider>
  </StrictMode>,
)
