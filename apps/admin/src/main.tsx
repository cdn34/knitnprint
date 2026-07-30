import { StrictMode, type FormEvent } from 'react'
import { createRoot } from 'react-dom/client'
import {
  QueryClient,
  QueryClientProvider,
  useMutation,
  useQuery,
  useQueryClient,
} from '@tanstack/react-query'
import {
  Boxes,
  LayoutDashboard,
  LoaderCircle,
  LockKeyhole,
  LogOut,
  Package,
  Settings,
  ShoppingBag,
  Users,
} from 'lucide-react'
import {
  ApiError,
  createApiClient,
  type StaffProfile,
} from '@knitprint/api-client'
import './styles.css'

const api = createApiClient()
const profileKey = ['staff-profile'] as const
const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      retry: false,
      staleTime: 30_000,
    },
  },
})

const navigation = [
  [LayoutDashboard, 'Dashboard'],
  [ShoppingBag, 'Orders'],
  [Package, 'Products'],
  [Users, 'Customers'],
  [Boxes, 'Discounts'],
  [Settings, 'Settings'],
] as const

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
          {navigation.map(([Icon, label], index) => (
            <a
              className={index === 0 ? 'active' : ''}
              href={`#${label.toLowerCase()}`}
              key={label}
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
            <h1>Good to see you, {profile.display_name.split(' ')[0]}.</h1>
          </div>
          <a className="storefront-link" href="http://localhost:3000">
            View storefront
          </a>
        </header>
        <section className="welcome">
          <div>
            <p>Secure workspace</p>
            <h2>Your KnitPrint operations, in one place.</h2>
            <span>
              Signed in as {profile.role}. Catalog, order, and customer tools
              arrive as complete feature slices.
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
            <small>Catalog comes in Phase 2</small>
          </article>
          <article>
            <span>Low stock</span><strong>—</strong>
            <small>Inventory comes in Phase 3</small>
          </article>
        </section>
      </main>
    </div>
  )
}

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <App />
    </QueryClientProvider>
  </StrictMode>,
)

