import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { Boxes, LayoutDashboard, Package, Settings, ShoppingBag, Users } from 'lucide-react'
import './styles.css'

const navigation = [
  [LayoutDashboard, 'Dashboard'],
  [ShoppingBag, 'Orders'],
  [Package, 'Products'],
  [Users, 'Customers'],
  [Boxes, 'Discounts'],
  [Settings, 'Settings'],
] as const

function AdminShell() {
  return (
    <div className="admin-shell">
      <aside>
        <div className="admin-brand">
          <img src="/knitprint-wordmark.webp" alt="KnitPrint" />
          <span>Admin</span>
        </div>
        <nav aria-label="Admin navigation">
          {navigation.map(([Icon, label], index) => (
            <a className={index === 0 ? 'active' : ''} href={`#${label.toLowerCase()}`} key={label}>
              <Icon size={18} /> {label}
            </a>
          ))}
        </nav>
        <div className="admin-user"><span>KP</span><div><strong>Store owner</strong><small>owner@knitprint.pt</small></div></div>
      </aside>
      <main>
        <header><div><small>Wednesday, 29 July</small><h1>Good evening.</h1></div><button type="button">View storefront</button></header>
        <section className="welcome">
          <div><p>Foundation ready</p><h2>Your KnitPrint workspace is taking shape.</h2><span>Catalog, order, and customer tools will arrive as complete feature slices.</span></div>
          <div className="welcome-mark">KP</div>
        </section>
        <section className="metrics" aria-label="Store metrics">
          <article><span>Orders to fulfill</span><strong>—</strong><small>Available after order setup</small></article>
          <article><span>Products</span><strong>—</strong><small>Catalog comes in Phase 2</small></article>
          <article><span>Low stock</span><strong>—</strong><small>Inventory comes in Phase 3</small></article>
        </section>
      </main>
    </div>
  )
}

createRoot(document.getElementById('root')!).render(<StrictMode><AdminShell /></StrictMode>)
