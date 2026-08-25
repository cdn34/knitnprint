import type { ReactNode } from 'react'
import {
  CircleUserRound,
  Menu,
  ShoppingBag,
  Sparkles,
} from 'lucide-react'

function IconButton({
  label,
  children,
}: Readonly<{ label: string; children: ReactNode }>) {
  return (
    <button className="icon-button" aria-label={label} type="button">
      {children}
    </button>
  )
}

export function StorefrontAnnouncement() {
  return (
    <div className="announcement">
      <Sparkles size={14} aria-hidden="true" />
      Small-batch objects, made slowly in Portugal
    </div>
  )
}

export function StorefrontHeader() {
  return (
    <header className="site-header">
      <a className="brand" href="/" aria-label="KnitPrint home">
        <img
          src="/knitprint-wordmark.webp"
          alt="KnitPrint"
          width="750"
          height="195"
        />
      </a>

      <nav className="desktop-nav" aria-label="Main navigation">
        <a href="/products">Shop</a>
        <a href="/collections">Collections</a>
        <a href="/about">Our story</a>
        <a href="/b2b">B2B</a>
      </nav>

      <div className="header-actions">
        <a className="icon-button" aria-label="Account" href="/account">
          <CircleUserRound />
        </a>
        <a className="icon-button" aria-label="View cart" href="/cart">
          <ShoppingBag />
        </a>
        <span className="mobile-action">
          <IconButton label="Open menu">
            <Menu />
          </IconButton>
        </span>
      </div>
    </header>
  )
}

export function StorefrontFooter() {
  return (
    <footer className="site-footer">
      <div className="footer-links">
        <div>
          <h2>About us</h2>
          <a href="/about">About us</a>
          <a href="/discounts">Discount code</a>
          <a href="/personalized-gifts">Personalized gifts</a>
        </div>
        <div>
          <h2>Shop</h2>
          <a href="/products">All pieces</a>
          <a href="/collections">Collections</a>
        </div>
        <div>
          <h2>Help</h2>
          <a href="/terms">Terms and conditions</a>
          <a href="/privacy">Privacy policy</a>
          <a href="/cookies">Cookies policy</a>
          <a href="/returns">Return policy</a>
          <a href="/faq">FAQ's</a>
          <a
            href="https://www.livroreclamacoes.pt/Inicio/"
            target="_blank"
            rel="noreferrer"
          >
            Complaints book
          </a>
        </div>
        <div>
          <h2>Customer support and contacts</h2>
          <a href="mailto:support@knitnprint.com">support@knitnprint.com</a>
          <a
            className="footer-social-link"
            href="/contact#social-media"
            aria-label="Instagram"
          >
            <svg
              aria-hidden="true"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.8"
              strokeLinecap="round"
              strokeLinejoin="round"
            >
              <rect x="3" y="3" width="18" height="18" rx="5" />
              <circle cx="12" cy="12" r="4" />
              <circle cx="17.4" cy="6.6" r="0.8" fill="currentColor" stroke="none" />
            </svg>
          </a>
        </div>
      </div>
      <div className="footer-bottom">
        <span>© 2026 KnitPrint</span>
        <span>Made with care in Portugal</span>
      </div>
    </footer>
  )
}
