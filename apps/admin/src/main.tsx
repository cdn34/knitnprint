import { StrictMode, type FormEvent, useEffect, useState } from 'react'
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
  Boxes,
  LayoutDashboard,
  LoaderCircle,
  LockKeyhole,
  LogOut,
  Package,
  Plus,
  Search,
  Send,
  ShieldCheck,
  UserRoundX,
} from 'lucide-react'
import {
  ApiError,
  createApiClient,
  type Product,
  type InventoryRecord,
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
    ...(profile.capabilities.includes('catalog.read')
      ? [{ id: 'products', label: 'Products', icon: Package }]
      : []),
    ...(profile.capabilities.includes('inventory.adjust')
      ? [{ id: 'inventory', label: 'Inventory', icon: Boxes }]
      : []),
    ...(profile.capabilities.includes('staff.manage')
      ? [{ id: 'staff', label: 'Staff', icon: ShieldCheck }]
      : []),
  ] as const
  type PageId = (typeof availablePages)[number]['id']
  const pageFromHash = () => {
    const requested = window.location.hash.slice(1)
    return (
      availablePages.find((page) => page.id === requested)?.id ?? 'dashboard'
    )
  }
  const [page, setPage] = useState<PageId>(pageFromHash)
  useEffect(() => {
    const changePage = () => setPage(pageFromHash())
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
                : page === 'products'
                  ? 'Product catalog.'
                  : page === 'inventory'
                    ? 'Inventory control.'
                  : 'Staff access.'}
            </h1>
          </div>
          <a className="storefront-link" href="http://localhost:3000">
            View storefront
          </a>
        </header>
        {page === 'dashboard' && (
          <>
            <section className="welcome">
              <div>
                <p>Secure workspace</p>
                <h2>Your KnitPrint operations, in one place.</h2>
                <span>
                  Signed in as {profile.role}. Use the sidebar to move between
                  focused operational areas.
                </span>
              </div>
              <div className="welcome-mark">KP</div>
            </section>
            <section className="metrics" aria-label="Store metrics">
              <article>
                <span>Orders to fulfill</span><strong>—</strong>
                <small>Available after order setup</small>
              </article>
              <article>
                <span>Products</span><strong>—</strong>
                <small>Open Products from the sidebar</small>
              </article>
              <article>
                <span>Low stock</span><strong>—</strong>
                <small>Open Inventory from the sidebar</small>
              </article>
            </section>
          </>
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
            <InventoryManagement />
          )}
        {page === 'staff' && profile.capabilities.includes('staff.manage') && (
          <StaffManagement currentStaffId={profile.id} />
        )}
      </main>
    </div>
  )
}

const inventoryKey = ['inventory'] as const

function InventoryManagement() {
  const client = useQueryClient()
  const [selected, setSelected] = useState<InventoryRecord | null>(null)
  const inventory = useQuery({ queryKey: inventoryKey, queryFn: api.listInventory })
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
        <span>{inventory.data?.filter(({ low_stock }) => low_stock).length ?? '—'} low stock</span>
      </div>
      <div className="inventory-layout">
        <div className="inventory-list">
          {inventory.isPending && <p className="panel-message">Loading inventory…</p>}
          {inventory.isError && <p className="panel-message error" role="alert">Inventory could not be loaded.</p>}
          {inventory.data?.map((record) => (
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
          {inventory.data?.length === 0 && <p className="panel-message">Create a product variant to begin tracking stock.</p>}
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

function CatalogManagement({
  canUpload,
  canWrite,
}: Readonly<{ canUpload: boolean; canWrite: boolean }>) {
  const client = useQueryClient()
  const [search, setSearch] = useState('')
  const [preview, setPreview] = useState<Product | null>(null)
  const [imagePreviews, setImagePreviews] = useState<Record<string, string>>({})
  const products = useQuery({
    queryKey: [...productsKey, search],
    queryFn: () => api.listAdminProducts({ q: search }),
  })
  const categories = useQuery({
    queryKey: categoriesKey,
    queryFn: api.listCategories,
  })
  const createProduct = useMutation({
    mutationFn: api.createProduct,
    onSuccess: (product) => {
      client.setQueriesData<Array<Product>>(
        { queryKey: productsKey },
        (current) =>
          current && !current.some(({ id }) => id === product.id)
            ? [product, ...current]
            : current,
      )
      client.invalidateQueries({ queryKey: productsKey })
      setPreview(product)
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
      altText,
      file,
      product,
    }: {
      altText: string
      file: File
      product: Product
    }) => {
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
      return { file, product }
    },
    onSuccess: ({ file, product }) => {
      const previewUrl = URL.createObjectURL(file)
      setImagePreviews((current) => ({
        ...current,
        [product.id]: previewUrl,
      }))
      setPreview(product)
    },
  })

  function selectImage(product: Product, file?: File) {
    if (!file) return
    const altText = window.prompt(
      `Describe the image of ${product.title} for customers using assistive technology.`,
    )
    if (altText?.trim()) {
      uploadImage.mutate({ altText: altText.trim(), file, product })
    }
  }

  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    const formElement = event.currentTarget
    const form = new FormData(formElement)
    const price = Number(form.get('product-price'))
    createProduct.mutate(
      {
        title: String(form.get('product-title') ?? ''),
        slug: String(form.get('product-slug') ?? ''),
        description: String(form.get('product-description') ?? ''),
        search_keywords: String(form.get('product-keywords') ?? ''),
        variants: [
          {
            title: String(form.get('variant-title') ?? ''),
            sku: String(form.get('variant-sku') ?? ''),
            price_minor: Math.round(price * 100),
            currency: String(form.get('variant-currency') ?? ''),
            option_values: {},
          },
        ],
      },
      { onSuccess: () => formElement.reset() },
    )
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
                {imagePreviews[product.id] ?? product.media[0]?.thumbnail_url ? (
                  <img
                    src={
                      imagePreviews[product.id] ??
                      product.media[0]?.thumbnail_url
                    }
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
                  {product.variants.length} variant
                  {product.variants.length === 1 ? '' : 's'}
                </small>
              </div>
              <div className="product-actions">
                <button type="button" onClick={() => setPreview(product)}>
                  <Eye size={15} /> Preview
                </button>
                {canUpload && (
                  <label className="product-upload">
                    <ImageUp size={15} /> Image
                    <input
                      type="file"
                      accept="image/jpeg,image/png,image/webp"
                      disabled={uploadImage.isPending}
                      onChange={(event) => {
                        selectImage(product, event.currentTarget.files?.[0])
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
                <strong>New product draft</strong>
                <span>Start with the product and its first variant.</span>
              </div>
            </div>
            <label htmlFor="product-title">Product title</label>
            <input id="product-title" name="product-title" required />
            <label htmlFor="product-slug">URL slug</label>
            <input
              id="product-slug"
              name="product-slug"
              pattern="[a-z0-9]+(?:-[a-z0-9]+)*"
              placeholder="woven-planter"
              required
            />
            <label htmlFor="product-description">Description</label>
            <textarea
              id="product-description"
              name="product-description"
              rows={3}
            />
            <label htmlFor="product-keywords">Search keywords</label>
            <input id="product-keywords" name="product-keywords" />
            <div className="variant-heading">First variant</div>
            <label htmlFor="variant-title">Variant title</label>
            <input
              id="variant-title"
              name="variant-title"
              defaultValue="Default"
              required
            />
            <label htmlFor="variant-sku">SKU</label>
            <input id="variant-sku" name="variant-sku" required />
            <div className="price-fields">
              <div>
                <label htmlFor="product-price">Price</label>
                <input
                  id="product-price"
                  name="product-price"
                  type="number"
                  min="0"
                  step="0.01"
                  required
                />
              </div>
              <div>
                <label htmlFor="variant-currency">Currency</label>
                <select
                  id="variant-currency"
                  name="variant-currency"
                  defaultValue="EUR"
                >
                  <option value="EUR">EUR</option>
                  <option value="GBP">GBP</option>
                  <option value="USD">USD</option>
                </select>
              </div>
            </div>
            {createProduct.isError && (
              <p className="panel-error" role="alert">
                {createProduct.error instanceof ApiError
                  ? createProduct.error.message
                  : 'The draft could not be created.'}
              </p>
            )}
            <button className="primary-button" disabled={createProduct.isPending}>
              {createProduct.isPending ? 'Creating…' : 'Create draft'}
            </button>
          </form>
        )}
      </div>
      {preview && (
        <article className="product-preview" aria-label="Product preview">
          <button
            type="button"
            aria-label="Close product preview"
            onClick={() => setPreview(null)}
          >
            ×
          </button>
          {imagePreviews[preview.id] ?? preview.media[0]?.detail_url ? (
            <img
              className="preview-art preview-image"
              src={imagePreviews[preview.id] ?? preview.media[0]?.detail_url}
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

const staffKey = ['staff'] as const
const assignableCapabilities = [
  ['catalog.read', 'View catalog'],
  ['catalog.write', 'Manage catalog'],
  ['orders.read', 'View orders'],
  ['orders.fulfill', 'Fulfill orders'],
  ['orders.refund', 'Refund orders'],
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
