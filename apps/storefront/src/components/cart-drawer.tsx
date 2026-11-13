import { ApiError, type Cart } from '@knitnprint/api-client'
import { ArrowRight, Check, ShoppingBag, X } from 'lucide-react'
import { useCallback, useEffect, useRef, useState } from 'react'
import { createPortal } from 'react-dom'
import { announceCartUpdate, cartApi } from '../cart-api'
import { mediaUrl } from '../catalog-api'
import { useI18n } from '../i18n'
import { personalizationSummary } from '../personalization-summary'

export function CartDrawer({ added, onClose }: Readonly<{ added: boolean; onClose: () => void }>) {
  const { t, formatCurrency } = useI18n()
  const dialog = useRef<HTMLDialogElement>(null)
  const closeButton = useRef<HTMLButtonElement>(null)
  const [cart, setCart] = useState<Cart | null>(null)
  const [status, setStatus] = useState<'loading' | 'ready' | 'guest' | 'error'>('loading')
  const requestId = useRef(0)

  function close() {
    dialog.current?.close()
    onClose()
  }

  const load = useCallback(async () => {
    const request = ++requestId.current
    setStatus('loading')
    try {
      const next = await cartApi.cart()
      if (request !== requestId.current) return
      setCart(next)
      setStatus('ready')
      announceCartUpdate(next)
    } catch (error) {
      if (request !== requestId.current) return
      setCart(null)
      setStatus(error instanceof ApiError && error.status === 401 ? 'guest' : 'error')
    }
  }, [])

  useEffect(() => {
    const element = dialog.current!
    const previousOverflow = document.body.style.overflow
    element.showModal()
    document.body.style.overflow = 'hidden'
    closeButton.current?.focus({ preventScroll: true })
    void load()
    return () => {
      requestId.current++
      element.close()
      document.body.style.overflow = previousOverflow
    }
  }, [load])

  return createPortal(
    <dialog
      className="cart-drawer"
      ref={dialog}
      aria-labelledby="cart-drawer-title"
      onKeyDown={(event) => {
        if (event.key !== 'Tab') return
        const controls = Array.from(event.currentTarget.querySelectorAll<HTMLElement>('a[href], button:not([disabled])')).filter(element => element.getClientRects().length > 0)
        const first = controls[0]
        const last = controls.at(-1)
        if (!first || !last) return
        if (event.shiftKey && (document.activeElement === first || document.activeElement === event.currentTarget)) {
          event.preventDefault()
          last.focus()
        } else if (!event.shiftKey && (document.activeElement === last || document.activeElement === event.currentTarget)) {
          event.preventDefault()
          first.focus()
        }
      }}
      onCancel={(event) => { event.preventDefault(); close() }}
      onClick={(event) => { if (event.target === event.currentTarget) close() }}
    >
      <div className="cart-drawer-panel">
        <header className="cart-drawer-heading">
          <div><p className="eyebrow">{t('cart.eyebrow')}</p><h2 id="cart-drawer-title"><ShoppingBag size={24} aria-hidden="true" />{t('cart.title')}</h2></div>
          <button className="icon-button" ref={closeButton} type="button" onClick={close} aria-label={t('cartDrawer.close')}><X aria-hidden="true" /></button>
        </header>
        <div className="cart-drawer-content">
          {added && <p className="cart-drawer-success" role="status"><Check size={18} aria-hidden="true" />{t('personalization.added')}</p>}
          {status === 'loading' && <p className="cart-drawer-state" role="status">{t('cart.loading')}</p>}
          {status === 'error' && <div className="cart-drawer-state" role="alert"><p>{t('cart.unavailable')}</p><button className="button button--secondary" type="button" onClick={() => { void load() }}>{t('cartDrawer.retry')}</button><a href="/cart">{t('shell.viewCart')}</a></div>}
          {status === 'guest' && <div className="cart-drawer-state"><ShoppingBag size={36} aria-hidden="true" /><p>{t('cartDrawer.signInPrompt')}</p><a className="button button--primary" href="/account">{t('account.signIn')}</a></div>}
          {status === 'ready' && cart && (cart.items.length ? <>
            <p className="cart-drawer-count">{cart.item_count} {t(cart.item_count === 1 ? 'cart.piece' : 'cart.pieces')}</p>
            <ul className="cart-drawer-items">
              {cart.items.map(item => <li className="cart-drawer-item" key={item.id}>
                <a className="cart-drawer-image" href={`/products/${item.product_slug}`} aria-label={t('common.viewProduct', { name: item.product_title })}>
                  {item.image_url ? <img src={mediaUrl(item.image_url)} alt="" /> : <ShoppingBag aria-hidden="true" />}
                </a>
                <div className="cart-drawer-item-copy">
                  <h3><a href={`/products/${item.product_slug}`}>{item.product_title}</a></h3>
                  <p>{item.variant_title}</p>
                  {Boolean(item.customization) && <p className="cart-drawer-personalization">{personalizationSummary(item.customization, item.customization_media_asset_ids ?? [], t)}</p>}
                  {item.personalization_unit_price_minor > 0 && <p className="cart-drawer-price-breakdown">{t('personalization.cartPriceBreakdown', { base: formatCurrency(item.base_unit_price_minor, item.currency), personalization: formatCurrency(item.personalization_unit_price_minor, item.currency) })}</p>}
                  <div className="cart-drawer-item-total"><span>{t('cart.quantity')}: {item.quantity}</span><strong>{formatCurrency(item.line_total_minor, item.currency)}</strong></div>
                  {!item.available && <p className="cart-drawer-item-unavailable">{t('product.unavailable')}</p>}
                </div>
              </li>)}
            </ul>
          </> : <div className="cart-drawer-state"><ShoppingBag size={40} aria-hidden="true" /><h3>{t('cart.emptyTitle')}</h3><p>{t('cart.emptyBody')}</p></div>)}
        </div>
        <footer className="cart-drawer-footer">
          {status === 'ready' && cart && cart.items.length > 0 && <>
            <div className="cart-drawer-subtotal"><span>{t('cart.subtotal')}</span><strong>{formatCurrency(cart.subtotal_minor, cart.currency ?? 'EUR')}</strong></div>
            <p>{t('cartDrawer.deliveryNote')}</p>
            <a className="button button--primary" href="/cart">{t('cartDrawer.checkout')}<ArrowRight size={18} aria-hidden="true" /></a>
          </>}
          <a className="button button--secondary" href="/products" onClick={(event) => {
            if (window.location.pathname.replace(/\/$/, '') === '/products') { event.preventDefault(); close() }
          }}>{t('cart.continueShopping')}</a>
        </footer>
      </div>
    </dialog>,
    document.body,
  )
}
