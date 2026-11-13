import { useCallback, useEffect, useRef, useState, type MouseEvent } from 'react'
import {
  ChevronDown,
  CircleUserRound,
  Menu,
  ShoppingBag,
  Sparkles,
  X,
} from 'lucide-react'
import { cartApi, CART_COUNT_UPDATED } from '../cart-api'
import { localeLabels, supportedLocales, useI18n, type Locale } from '../i18n'
import { CartDrawer } from './cart-drawer'

export function StorefrontAnnouncement() {
  const { t } = useI18n()
  return (
    <div className="announcement">
      <Sparkles size={14} aria-hidden="true" />
      {t('shell.announcement')}
    </div>
  )
}

export function StorefrontHeader() {
  const { locale, setLocale, t } = useI18n()
  const [cartCount, setCartCount] = useState(0)
  const [menuOpen, setMenuOpen] = useState(false)
  const [interactive, setInteractive] = useState(false)
  const menuButtonRef = useRef<HTMLButtonElement>(null)
  const cartLinkRef = useRef<HTMLAnchorElement>(null)
  const [cartOpen, setCartOpen] = useState(false)
  const [cartAdded, setCartAdded] = useState(false)
  const closeCart = useCallback(() => {
    setCartOpen(false)
    cartLinkRef.current?.focus({ preventScroll: true })
  }, [])
  const navigationLinks = [
    { href: '/products', label: t('shell.shop') },
    { href: '/collections', label: t('shell.collections') },
    { href: '/about', label: t('shell.ourStory') },
    { href: '/b2b', label: 'B2B' },
  ]

  useEffect(() => {
    setInteractive(true)
    let active = true
    const updateCount = (event: Event) => setCartCount((event as CustomEvent<number>).detail)
    window.addEventListener(CART_COUNT_UPDATED, updateCount)
    const search = new URLSearchParams(window.location.search)
    if (search.get('cart') === 'open') {
      setCartAdded(true)
      setCartOpen(true)
      search.delete('cart')
      const query = search.toString()
      window.history.replaceState(window.history.state, '', `${window.location.pathname}${query ? `?${query}` : ''}${window.location.hash}`)
    } else cartApi.cart().then((cart) => {
      if (!active) return
      setCartCount(cart.item_count)
    }).catch(() => undefined)
    return () => {
      active = false
      window.removeEventListener(CART_COUNT_UPDATED, updateCount)
    }
  }, [])

  useEffect(() => {
    if (!menuOpen) return
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        setMenuOpen(false)
        menuButtonRef.current?.focus()
      }
    }
    window.addEventListener('keydown', closeOnEscape)
    return () => window.removeEventListener('keydown', closeOnEscape)
  }, [menuOpen])

  function openCart(event: MouseEvent<HTMLAnchorElement>) {
    if (event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return
    event.preventDefault()
    setMenuOpen(false)
    setCartAdded(false)
    setCartOpen(true)
  }

  return (
    <div className="site-header-shell">
      <header className="site-header">
        <a className="brand" href="/" aria-label={t('shell.homeLabel')}>
          <img
            src="/knitnprint-wordmark.webp"
            alt="KnitNPrint"
            width="750"
            height="195"
          />
        </a>

        <nav className="desktop-nav" aria-label={t('shell.mainNavigation')}>
          {navigationLinks.map((link) => <a key={link.href} href={link.href}>{link.label}</a>)}
        </nav>

        <div className="header-actions">
          <label className="language-selector">
            <span className="sr-only">{t('shell.language')}</span>
            <select
              aria-label={t('shell.language')}
              value={locale}
              onChange={(event) => setLocale(event.target.value as Locale)}
            >
              {supportedLocales.map((option) => (
                <option value={option} key={option}>{localeLabels[option]}</option>
              ))}
            </select>
            <span className="language-selector-display" aria-hidden="true">
              {locale.toUpperCase()}
              <ChevronDown size={13} strokeWidth={2} />
            </span>
          </label>
          <a className="icon-button" aria-label={t('shell.account')} href="/account">
            <CircleUserRound />
          </a>
          <a className="icon-button cart-icon-button" ref={cartLinkRef} aria-label={`${t('shell.viewCart')} · ${cartCount}`} aria-haspopup={interactive ? 'dialog' : undefined} aria-expanded={interactive ? cartOpen : undefined} href="/cart" onClick={openCart}>
            <ShoppingBag />
            {cartCount > 0 && <span className="cart-count-badge" aria-hidden="true">{cartCount > 999 ? '999+' : cartCount}</span>}
          </a>
          <span className="mobile-action">
            <button ref={menuButtonRef} className="icon-button" type="button" disabled={!interactive} aria-label={t(menuOpen ? 'shell.closeMenu' : 'shell.openMenu')} aria-controls="mobile-primary-nav" aria-expanded={menuOpen} onClick={() => setMenuOpen((open) => !open)}>
              {menuOpen ? <X /> : <Menu />}
            </button>
          </span>
        </div>
      </header>
      <nav id="mobile-primary-nav" className="mobile-nav" aria-label={t('shell.mainNavigation')} hidden={!menuOpen}>
        {navigationLinks.map((link) => <a key={link.href} href={link.href} onClick={() => setMenuOpen(false)}>{link.label}</a>)}
      </nav>
      {cartOpen && <CartDrawer added={cartAdded} onClose={closeCart} />}
    </div>
  )
}

export function StorefrontFooter() {
  const { t } = useI18n()
  return (
    <footer className="site-footer">
      <div className="footer-links">
        <div>
          <h2>{t('shell.aboutUs')}</h2>
          <a href="/about">{t('shell.aboutUs')}</a>
          <a href="/discounts">{t('shell.discountCode')}</a>
          <a href="/personalized-gifts">{t('shell.personalizedGifts')}</a>
        </div>
        <div>
          <h2>{t('shell.shop')}</h2>
          <a href="/products">{t('shell.allPieces')}</a>
          <a href="/collections">{t('shell.collections')}</a>
        </div>
        <div>
          <h2>{t('shell.help')}</h2>
          <a href="/terms">{t('shell.terms')}</a>
          <a href="/privacy">{t('shell.privacy')}</a>
          <a href="/cookies">{t('shell.cookies')}</a>
          <a href="/returns">{t('shell.returns')}</a>
          <a href="/faq">{t('shell.faqs')}</a>
          <a
            href="https://www.livroreclamacoes.pt/Inicio/"
            target="_blank"
            rel="noreferrer"
          >
            {t('shell.complaintsBook')}
          </a>
        </div>
        <div>
          <h2>{t('shell.support')}</h2>
          <a href="mailto:support@knitnprint.com">support@knitnprint.com</a>
          <a
            className="footer-social-link"
            href="https://www.instagram.com/_knitnprint_/"
            target="_blank"
            rel="noopener noreferrer"
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
        <span>© 2026 KnitNPrint</span>
        <span>{t('shell.madeInPortugal')}</span>
      </div>
    </footer>
  )
}
