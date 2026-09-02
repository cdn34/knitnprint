import { useEffect, useState, type ReactNode } from 'react'
import {
  CircleUserRound,
  Menu,
  ShoppingBag,
  Sparkles,
} from 'lucide-react'
import { cartApi, CART_COUNT_UPDATED } from '../cart-api'
import { localeLabels, supportedLocales, useI18n, type Locale } from '../i18n'

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

  useEffect(() => {
    let active = true
    const updateCount = (event: Event) => setCartCount((event as CustomEvent<number>).detail)
    window.addEventListener(CART_COUNT_UPDATED, updateCount)
    cartApi.cart().then((cart) => {
      if (!active) return
      setCartCount(cart.item_count)
    }).catch(() => undefined)
    return () => {
      active = false
      window.removeEventListener(CART_COUNT_UPDATED, updateCount)
    }
  }, [])

  return (
    <header className="site-header">
      <a className="brand" href="/" aria-label={t('shell.homeLabel')}>
        <img
          src="/knitprint-wordmark.webp"
          alt="KnitnPrint"
          width="750"
          height="195"
        />
      </a>

      <nav className="desktop-nav" aria-label={t('shell.mainNavigation')}>
        <a href="/products">{t('shell.shop')}</a>
        <a href="/collections">{t('shell.collections')}</a>
        <a href="/about">{t('shell.ourStory')}</a>
        <a href="/b2b">B2B</a>
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
        </label>
        <a className="icon-button" aria-label={t('shell.account')} href="/account">
          <CircleUserRound />
        </a>
        <a className="icon-button cart-icon-button" aria-label={`${t('shell.viewCart')} · ${cartCount}`} href="/cart">
          <ShoppingBag />
          {cartCount > 0 && <span className="cart-count-badge" aria-hidden="true">{cartCount > 999 ? '999+' : cartCount}</span>}
        </a>
        <span className="mobile-action">
          <IconButton label={t('shell.openMenu')}>
            <Menu />
          </IconButton>
        </span>
      </div>
    </header>
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
        <span>© 2026 KnitnPrint</span>
        <span>{t('shell.madeInPortugal')}</span>
      </div>
    </footer>
  )
}
