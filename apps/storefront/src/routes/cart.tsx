import type {
  Cart,
  GuestCustomerRequest,
  Order,
  PaymentOptions,
} from '@knitprint/api-client'
import { createFileRoute } from '@tanstack/react-router'
import {
  ArrowLeft,
  CircleCheck,
  PackageOpen,
  ReceiptText,
  ShoppingBag,
  Trash2,
  TriangleAlert,
} from 'lucide-react'
import { useEffect, useState, type FormEvent } from 'react'
import { cartApi, cartMutationKey } from '../cart-api'
import { ContextualFaqs } from '../components/contextual-faqs'
import { StorefrontAnnouncement, StorefrontFooter, StorefrontHeader } from '../components/storefront-shell'
import { useI18n } from '../i18n'

export const Route = createFileRoute('/cart')({
  head: () => ({
    meta: [
      { title: 'Your cart — KnitnPrint' },
      {
        name: 'description',
        content: 'Review your KnitnPrint pieces and prepare delivery details.',
      },
    ],
  }),
  component: CartPage,
})

function CartPage() {
  const { t, formatCurrency } = useI18n()
  const [cart, setCart] = useState<Cart | null>(null)
  const [loading, setLoading] = useState(true)
  const [busyLine, setBusyLine] = useState<string | null>(null)
  const [message, setMessage] = useState('')
  const [order, setOrder] = useState<Order | null>(null)
  const [submittingOrder, setSubmittingOrder] = useState(false)
  const [paymentOptions, setPaymentOptions] = useState<PaymentOptions | null>(null)
  const [discountBusy, setDiscountBusy] = useState(false)
  const [shippingBusy, setShippingBusy] = useState(false)

  useEffect(() => {
    let active = true
    const orderId = new URLSearchParams(window.location.search).get('order_id')
    Promise.all([
      cartApi.paymentOptions(),
      orderId ? cartApi.customerOrder(orderId) : cartApi.cart(),
    ])
      .then(([options, resource]) => {
        if (!active) return
        setPaymentOptions(options)
        if (orderId) setOrder(resource as Order)
        else setCart(resource as Cart)
      })
      .catch(() => {
        if (active) setMessage(t('cart.unavailable'))
      })
      .finally(() => {
        if (active) setLoading(false)
      })
    return () => {
      active = false
    }
  }, [])

  useEffect(() => {
    if (
      !order ||
      order.payment.provider !== 'stripe' ||
      order.payment_status !== 'pending' ||
      new URLSearchParams(window.location.search).get('payment') !== 'return'
    ) {
      return
    }
    let active = true
    let checks = 0
    let nextTimeout: number | undefined
    const refresh = async () => {
      try {
        const nextOrder = await cartApi.customerOrder(order.id)
        if (!active) return
        setOrder(nextOrder)
        checks += 1
        if (nextOrder.payment_status === 'pending' && checks < 10) {
          nextTimeout = window.setTimeout(refresh, 1500)
        }
      } catch {
        // Keep the owned order visible while a delayed webhook is retried.
      }
    }
    nextTimeout = window.setTimeout(refresh, 750)
    return () => {
      active = false
      if (nextTimeout) window.clearTimeout(nextTimeout)
    }
  }, [order?.id, order?.payment.provider, order?.payment_status])

  async function updateQuantity(lineId: string, quantity: number) {
    setBusyLine(lineId)
    setMessage('')
    try {
      setCart(
        await cartApi.updateCartItem(
          lineId,
          { quantity },
          cartMutationKey(),
        ),
      )
    } catch {
      setMessage(t('cart.quantityUnavailable'))
    } finally {
      setBusyLine(null)
    }
  }

  async function removeItem(lineId: string) {
    setBusyLine(lineId)
    setMessage('')
    try {
      setCart(await cartApi.removeCartItem(lineId, cartMutationKey()))
    } catch {
      setMessage(t('cart.removeError'))
    } finally {
      setBusyLine(null)
    }
  }

  async function saveDelivery(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    setMessage('')
    const form = new FormData(event.currentTarget)
    const value = (name: string) => String(form.get(name) ?? '')
    const input: GuestCustomerRequest = {
      email: value('email'),
      first_name: value('first_name'),
      last_name: value('last_name'),
      phone: value('phone'),
      address: {
        recipient_name: value('recipient_name'),
        line1: value('line1'),
        line2: value('line2'),
        city: value('city'),
        region: value('region'),
        postal_code: value('postal_code'),
        country_code: value('country_code'),
        phone: value('address_phone'),
      },
    }
    try {
      const nextCart = await cartApi.setCartDelivery(input, cartMutationKey())
      setCart(nextCart)
      setMessage(t('cart.deliverySaved'))
    } catch {
      setMessage(t('cart.deliveryError'))
    }
  }

  async function applyDiscount(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    setDiscountBusy(true)
    setMessage('')
    const code = String(new FormData(event.currentTarget).get('discount_code') ?? '')
    try {
      setCart(await cartApi.applyCartDiscount({ code }, cartMutationKey()))
      setMessage(t('cart.discountApplied'))
      event.currentTarget.reset()
    } catch {
      setMessage(t('cart.discountError'))
    } finally {
      setDiscountBusy(false)
    }
  }

  async function removeDiscount() {
    setDiscountBusy(true)
    setMessage('')
    try {
      setCart(await cartApi.removeCartDiscount(cartMutationKey()))
      setMessage(t('cart.discountRemoved'))
    } catch {
      setMessage(t('cart.discountRemoveError'))
    } finally {
      setDiscountBusy(false)
    }
  }

  async function selectShippingMethod(shippingMethodId: string) {
    setShippingBusy(true)
    setMessage('')
    try {
      setCart(
        await cartApi.selectCartShippingMethod(
          { shipping_method_id: shippingMethodId },
          cartMutationKey(),
        ),
      )
      setMessage(t('cart.shippingUpdated'))
    } catch {
      setMessage(t('cart.shippingError'))
    } finally {
      setShippingBusy(false)
    }
  }

  async function createOrder() {
    setSubmittingOrder(true)
    setMessage('')
    try {
      if (!paymentOptions) return
      const method = paymentOptions.stripe ? 'stripe' : 'manual'
      const nextOrder = await cartApi.createOrder(
        { payment_method: method },
        cartMutationKey(),
      )
      setOrder(nextOrder)
      if (method === 'stripe') {
        window.history.replaceState(
          null,
          '',
          `/cart?payment=pending&order_id=${nextOrder.id}`,
        )
        await redirectToPayment(nextOrder.id)
      }
    } catch {
      setMessage(t('cart.orderError'))
      try {
        setCart(await cartApi.cart())
      } catch {
        // Preserve the actionable checkout error when reconciliation is unavailable.
      }
    } finally {
      setSubmittingOrder(false)
    }
  }

  async function redirectToPayment(orderId: string) {
    setSubmittingOrder(true)
    setMessage(t('cart.openingSecureCheckout'))
    try {
      const checkout = await cartApi.startOrderPayment(orderId)
      window.location.assign(checkout.checkout_url)
    } catch {
      setMessage(t('cart.secureCheckoutError'))
      setSubmittingOrder(false)
    }
  }

  const currency = cart?.currency ?? 'EUR'

  return (
    <>
      <StorefrontAnnouncement />
      <StorefrontHeader />

      <main className="cart-page" id="main-content" tabIndex={-1}>
        <a className="text-link page-back-link" href="/#shop">
          <ArrowLeft size={17} /> {t('cart.continueShopping')}
        </a>
        <div className="cart-heading">
          <p className="eyebrow">{t('cart.eyebrow')}</p>
          <h1>{order ? t('cart.orderReceived') : t('cart.title')}</h1>
          {!order && cart && <p>{cart.item_count} {t(cart.item_count === 1 ? 'cart.piece' : 'cart.pieces')}</p>}
        </div>

        {loading && <p className="cart-notice" role="status">{t('cart.loading')}</p>}
        {!loading && !cart && <p className="cart-notice" role="alert">{message}</p>}

        {order && (
          <OrderConfirmation
            order={order}
            busy={submittingOrder}
            onResumePayment={redirectToPayment}
          />
        )}

        {!order && cart && cart.items.length === 0 && (
          <section className="cart-empty">
            <PackageOpen aria-hidden="true" />
            <h2>{t('cart.emptyTitle')}</h2>
            <p>{t('cart.emptyBody')}</p>
            <a className="button button--primary" href="/#shop">{t('cart.shopCollection')}</a>
          </section>
        )}

        {!order && cart && cart.items.length > 0 && (
          <div className="cart-layout">
            <div className="cart-main">
              {cart.issues.length > 0 && (
                <section className="cart-issues" aria-labelledby="cart-issues-title">
                  <TriangleAlert aria-hidden="true" />
                  <div>
                    <h2 id="cart-issues-title">{t('cart.changedTitle')}</h2>
                    <ul>{cart.issues.map((issue, index) => <li key={`${issue.code}-${index}`}>{issue.message}</li>)}</ul>
                  </div>
                </section>
              )}

              <section className="cart-items" aria-labelledby="cart-items-title">
                <h2 id="cart-items-title" className="sr-only">{t('cart.itemsLabel')}</h2>
                {cart.items.map((item) => (
                  <article className="cart-item" key={item.id}>
                    <a className="cart-item-image" href={`/products/${item.product_slug}`}>
                      {item.image_url ? (
                        <img src={item.image_url} alt="" />
                      ) : (
                        <ShoppingBag aria-hidden="true" />
                      )}
                    </a>
                    <div className="cart-item-copy">
                      <h3><a href={`/products/${item.product_slug}`}>{item.product_title}</a></h3>
                      <p>{item.variant_title} · {t('cart.sku')} {item.sku}</p>
                      {Boolean(item.customization) && <p className="cart-item-personalization">Personalizado{item.customization_media_asset_id ? ' com fotografia' : ''}{typeof (item.customization as { text?: { content?: unknown } }).text?.content === 'string' ? ` · “${(item.customization as { text: { content: string } }).text.content}”` : ''}</p>}
                      <label>
                        <span>{t('cart.quantity')}</span>
                        <select
                          value={item.quantity}
                          disabled={busyLine === item.id || !item.available}
                          onChange={(event) => updateQuantity(item.id, Number(event.target.value))}
                        >
                          {Array.from(
                            { length: Math.min(10, Math.max(item.available_quantity, item.quantity)) },
                            (_, index) => index + 1,
                          ).map((quantity) => <option key={quantity}>{quantity}</option>)}
                        </select>
                      </label>
                    </div>
                    <div className="cart-item-actions">
                      <strong>{formatCurrency(item.line_total_minor, item.currency)}</strong>
                      <button
                        className="text-button"
                        type="button"
                        disabled={busyLine === item.id}
                        onClick={() => removeItem(item.id)}
                      >
                        <Trash2 size={16} aria-hidden="true" /> {t('cart.remove')}
                      </button>
                    </div>
                  </article>
                ))}
              </section>

              <DeliveryForm cart={cart} onSubmit={saveDelivery} />
            </div>

            <aside className="cart-summary" aria-labelledby="cart-summary-title">
              <h2 id="cart-summary-title">{t('cart.summary')}</h2>
              <div><span>{t('cart.subtotal')}</span><strong>{formatCurrency(cart.subtotal_minor, currency)}</strong></div>
              {cart.discount && (
                <div className="cart-discount-line">
                  <span>{cart.discount.code}</span>
                  <strong>−{formatCurrency(cart.discount_minor, currency)}</strong>
                </div>
              )}
              {cart.shipping ? (
                <div><span>{t('cart.shipping')} · {cart.shipping.method_name}</span><strong>{formatCurrency(cart.shipping_minor, currency)}</strong></div>
              ) : (
                <div><span>{t('cart.shipping')}</span><span>{t('cart.addDelivery')}</span></div>
              )}
              {cart.tax && (
                <div><span>{t('cart.tax')} · {cart.tax.rate_basis_points / 100}%</span><strong>{formatCurrency(cart.tax_minor, currency)}</strong></div>
              )}
              <div className="cart-total"><span>{t('cart.total')}</span><strong>{formatCurrency(cart.total_minor, currency)}</strong></div>
              {cart.shipping_methods.length > 1 && cart.shipping && (
                <label className="cart-shipping-select">
                  {t('cart.shippingMethod')}
                  <select
                    value={cart.shipping.id}
                    disabled={shippingBusy}
                    onChange={(event) => selectShippingMethod(event.target.value)}
                  >
                    {cart.shipping_methods.map((method) => (
                      <option value={method.id} key={method.id}>
                        {method.method_name} · {formatCurrency(method.amount_minor, method.currency)}
                      </option>
                    ))}
                  </select>
                </label>
              )}
              {cart.discount ? (
                <button className="text-button cart-discount-remove" type="button" disabled={discountBusy} onClick={removeDiscount}>{t('cart.removeDiscount')}</button>
              ) : (
                <form className="cart-discount-form" onSubmit={applyDiscount}>
                  <label htmlFor="discount-code">{t('cart.discountCode')}</label>
                  <div><input id="discount-code" name="discount_code" minLength={3} maxLength={32} required autoCapitalize="characters" /><button type="submit" disabled={discountBusy}>{t(discountBusy ? 'cart.applying' : 'cart.apply')}</button></div>
                </form>
              )}
              <p>{t('cart.recalculationNote')}</p>
              <button
                className="button button--primary"
                type="button"
                disabled={!cart.checkout_ready || submittingOrder || !paymentOptions}
                onClick={createOrder}
              >
                {submittingOrder
                  ? t('cart.openingCheckout')
                  : paymentOptions?.stripe
                    ? t('cart.paySecurely')
                    : t('cart.createOrder')}
              </button>
              <span className="cart-ready-state">
                {cart.checkout_ready ? <CircleCheck aria-hidden="true" /> : <TriangleAlert aria-hidden="true" />}
                {cart.checkout_ready
                  ? t('cart.ready')
                  : t('cart.notReady')}
              </span>
            </aside>
          </div>
        )}
        {message && cart && <p className="cart-notice" role="status">{message}</p>}
        {!order && (
          <ContextualFaqs
            id="cart-faqs"
            eyebrow={t('cart.faqEyebrow')}
            title={t('cart.faqTitle')}
            items={[
              { question: t('cart.faq1Question'), answer: t('cart.faq1Answer') },
              { question: t('cart.faq2Question'), answer: t('cart.faq2Answer') },
              { question: t('cart.faq3Question'), answer: t('cart.faq3Answer') },
              { question: t('cart.faq4Question'), answer: t('cart.faq4Answer') },
            ]}
            className="contextual-faqs--cart"
          />
        )}
      </main>
      <StorefrontFooter />
    </>
  )
}

function OrderConfirmation({
  order,
  busy,
  onResumePayment,
}: Readonly<{
  order: Order
  busy: boolean
  onResumePayment: (orderId: string) => Promise<void>
}>) {
  const { t, formatCurrency } = useI18n()
  const stripePending =
    order.payment.provider === 'stripe' && order.payment_status === 'pending'
  const paid = order.payment_status === 'paid'
  const thankCustomer = paid || order.payment.provider === 'manual'
  return (
    <section className="order-confirmation" aria-labelledby="order-confirmation-title">
      <div className="order-confirmation-mark"><ReceiptText aria-hidden="true" /></div>
      <p className="eyebrow">{order.order_number}</p>
      <h2 id="order-confirmation-title">
        {thankCustomer
          ? t('order.thankYou', { name: order.customer.first_name })
          : t('order.reserved')}
      </h2>
      <p>
        {paid
          ? t('order.paidBody')
          : stripePending
            ? t('order.pendingBody')
            : order.payment.failure_message ?? t('order.manualBody')}
        {' '}{t('order.reference')}
      </p>
      <dl className="order-confirmation-summary">
        <div><dt>{t('order.status')}</dt><dd>{order.order_status}</dd></div>
        <div><dt>{t('order.payment')}</dt><dd>{order.payment_status}</dd></div>
        <div><dt>{t('cart.subtotal')}</dt><dd>{formatCurrency(order.subtotal_minor, order.currency)}</dd></div>
        {order.discount && <div><dt>{t('order.discount')} ({order.discount.code})</dt><dd>−{formatCurrency(order.discount_minor, order.currency)}</dd></div>}
        <div><dt>{t('cart.shipping')} ({order.shipping.method_name})</dt><dd>{formatCurrency(order.shipping_minor, order.currency)}</dd></div>
        <div><dt>{t('cart.tax')} ({order.tax.rate_basis_points / 100}%)</dt><dd>{formatCurrency(order.tax_minor, order.currency)}</dd></div>
        <div><dt>{t('cart.total')}</dt><dd>{formatCurrency(order.total_minor, order.currency)}</dd></div>
      </dl>
      <div className="order-confirmation-lines">
        {order.lines.map((line) => (
          <div key={line.id}>
            <span>{line.quantity} × {line.product_title} · {line.variant_title}</span>
            <strong>{formatCurrency(line.line_total_minor, line.currency)}</strong>
          </div>
        ))}
      </div>
      <address>
        <strong>{t('order.deliverTo', { name: order.shipping_address.recipient_name })}</strong>
        <span>{order.shipping_address.line1}</span>
        {order.shipping_address.line2 && <span>{order.shipping_address.line2}</span>}
        <span>{order.shipping_address.postal_code} {order.shipping_address.city}</span>
        <span>{order.shipping_address.country_code}</span>
      </address>
      {stripePending && (
        <button
          className="button button--primary"
          type="button"
          disabled={busy}
          onClick={() => onResumePayment(order.id)}
        >
          {busy ? t('cart.openingCheckout') : t('order.continuePayment')}
        </button>
      )}
      <a className="button button--primary" href="/#shop">{t('cart.continueShopping')}</a>
    </section>
  )
}

function DeliveryForm({
  cart,
  onSubmit,
}: Readonly<{
  cart: Cart
  onSubmit: (event: FormEvent<HTMLFormElement>) => Promise<void>
}>) {
  const { t } = useI18n()
  const delivery = cart.delivery
  return (
    <section className="cart-delivery" aria-labelledby="delivery-title">
      <div>
        <p className="eyebrow">{t('delivery.eyebrow')}</p>
        <h2 id="delivery-title">{t('delivery.title')}</h2>
        <p>{t('delivery.intro')}</p>
      </div>
      <form className="delivery-form" onSubmit={onSubmit}>
        <label>{t('delivery.email')}<input required type="email" name="email" defaultValue={delivery?.email} /></label>
        <div className="field-row">
          <label>{t('delivery.firstName')}<input required name="first_name" defaultValue={delivery?.first_name} /></label>
          <label>{t('delivery.lastName')}<input required name="last_name" defaultValue={delivery?.last_name} /></label>
        </div>
        <label>{t('delivery.contactPhone')}<input name="phone" defaultValue={delivery?.phone} /></label>
        <label>{t('delivery.recipient')}<input required name="recipient_name" defaultValue={delivery?.address.recipient_name} /></label>
        <label>{t('delivery.address')}<input required name="line1" defaultValue={delivery?.address.line1} /></label>
        <label>{t('delivery.addressExtra')}<input name="line2" defaultValue={delivery?.address.line2} /></label>
        <div className="field-row">
          <label>{t('delivery.city')}<input required name="city" defaultValue={delivery?.address.city} /></label>
          <label>{t('delivery.region')}<input name="region" defaultValue={delivery?.address.region} /></label>
        </div>
        <div className="field-row field-row--postal">
          <label>{t('delivery.postalCode')}<input required name="postal_code" defaultValue={delivery?.address.postal_code} /></label>
          <label>{t('delivery.countryCode')}<input required name="country_code" minLength={2} maxLength={2} defaultValue={delivery?.address.country_code ?? 'PT'} /></label>
        </div>
        <label>{t('delivery.phone')}<input name="address_phone" defaultValue={delivery?.address.phone} /></label>
        <button className="button button--secondary" type="submit">{t('delivery.save')}</button>
      </form>
    </section>
  )
}
