import type { Cart, GuestCustomerRequest, Order } from '@knitprint/api-client'
import { createFileRoute } from '@tanstack/react-router'
import {
  ArrowLeft,
  CircleCheck,
  PackageOpen,
  ReceiptText,
  ShieldCheck,
  ShoppingBag,
  Trash2,
  TriangleAlert,
} from 'lucide-react'
import { useEffect, useState, type FormEvent } from 'react'
import { cartApi, cartMutationKey, formatMoney } from '../cart-api'

export const Route = createFileRoute('/cart')({
  head: () => ({
    meta: [
      { title: 'Your cart — KnitPrint' },
      {
        name: 'description',
        content: 'Review your KnitPrint pieces and prepare delivery details.',
      },
    ],
  }),
  component: CartPage,
})

function CartPage() {
  const [cart, setCart] = useState<Cart | null>(null)
  const [loading, setLoading] = useState(true)
  const [busyLine, setBusyLine] = useState<string | null>(null)
  const [message, setMessage] = useState('')
  const [order, setOrder] = useState<Order | null>(null)
  const [submittingOrder, setSubmittingOrder] = useState(false)

  useEffect(() => {
    let active = true
    cartApi
      .cart()
      .then((nextCart) => {
        if (active) setCart(nextCart)
      })
      .catch(() => {
        if (active) setMessage('Your cart is temporarily unavailable.')
      })
      .finally(() => {
        if (active) setLoading(false)
      })
    return () => {
      active = false
    }
  }, [])

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
      setMessage('That quantity is no longer available.')
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
      setMessage('We couldn’t remove that item. Please try again.')
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
      setMessage('Delivery details saved.')
    } catch {
      setMessage('Check each required delivery field and try again.')
    }
  }

  async function createOrder() {
    setSubmittingOrder(true)
    setMessage('')
    try {
      setOrder(
        await cartApi.createOrder(
          { payment_method: 'manual' },
          cartMutationKey(),
        ),
      )
    } catch {
      setMessage('Your order could not be created. Review the cart and try again.')
      try {
        setCart(await cartApi.cart())
      } catch {
        // Preserve the actionable checkout error when reconciliation is unavailable.
      }
    } finally {
      setSubmittingOrder(false)
    }
  }

  const currency = cart?.currency ?? 'EUR'

  return (
    <>
      <div className="announcement">
        <ShieldCheck size={14} aria-hidden="true" />
        Prices and availability are checked by our studio
      </div>
      <header className="site-header product-header">
        <a className="brand" href="/" aria-label="KnitPrint home">
          <img
            src="/knitprint-wordmark.webp"
            alt="KnitPrint"
            width="750"
            height="195"
          />
        </a>
        <a className="text-link" href="/#shop">
          <ArrowLeft size={17} /> Continue shopping
        </a>
      </header>

      <main className="cart-page" id="main-content" tabIndex={-1}>
        <div className="cart-heading">
          <p className="eyebrow">Your selection</p>
          <h1>{order ? 'Order received' : 'Cart'}</h1>
          {!order && cart && <p>{cart.item_count} {cart.item_count === 1 ? 'piece' : 'pieces'}</p>}
        </div>

        {loading && <p className="cart-notice" role="status">Loading your cart…</p>}
        {!loading && !cart && <p className="cart-notice" role="alert">{message}</p>}

        {order && <OrderConfirmation order={order} />}

        {!order && cart && cart.items.length === 0 && (
          <section className="cart-empty">
            <PackageOpen aria-hidden="true" />
            <h2>Your cart is waiting for its first piece.</h2>
            <p>Explore the latest small-batch objects from the studio.</p>
            <a className="button button--primary" href="/#shop">Shop the collection</a>
          </section>
        )}

        {!order && cart && cart.items.length > 0 && (
          <div className="cart-layout">
            <div className="cart-main">
              {cart.issues.length > 0 && (
                <section className="cart-issues" aria-labelledby="cart-issues-title">
                  <TriangleAlert aria-hidden="true" />
                  <div>
                    <h2 id="cart-issues-title">Your cart changed</h2>
                    <ul>{cart.issues.map((issue, index) => <li key={`${issue.code}-${index}`}>{issue.message}</li>)}</ul>
                  </div>
                </section>
              )}

              <section className="cart-items" aria-labelledby="cart-items-title">
                <h2 id="cart-items-title" className="sr-only">Cart items</h2>
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
                      <p>{item.variant_title} · SKU {item.sku}</p>
                      <label>
                        <span>Quantity</span>
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
                      <strong>{formatMoney(item.line_total_minor, item.currency)}</strong>
                      <button
                        className="text-button"
                        type="button"
                        disabled={busyLine === item.id}
                        onClick={() => removeItem(item.id)}
                      >
                        <Trash2 size={16} aria-hidden="true" /> Remove
                      </button>
                    </div>
                  </article>
                ))}
              </section>

              <DeliveryForm cart={cart} onSubmit={saveDelivery} />
            </div>

            <aside className="cart-summary" aria-labelledby="cart-summary-title">
              <h2 id="cart-summary-title">Summary</h2>
              <div><span>Subtotal</span><strong>{formatMoney(cart.subtotal_minor, currency)}</strong></div>
              <div><span>Shipping</span><span>Calculated with your order</span></div>
              <p>Taxes and final availability will be validated before an order is created.</p>
              <button
                className="button button--primary"
                type="button"
                disabled={!cart.checkout_ready || submittingOrder}
                onClick={createOrder}
              >
                {submittingOrder ? 'Creating order…' : 'Create order'}
              </button>
              <span className="cart-ready-state">
                {cart.checkout_ready ? <CircleCheck aria-hidden="true" /> : <TriangleAlert aria-hidden="true" />}
                {cart.checkout_ready
                  ? 'Cart and delivery details are ready.'
                  : 'Resolve cart changes and add delivery details.'}
              </span>
            </aside>
          </div>
        )}
        {message && cart && !order && <p className="cart-notice" role="status">{message}</p>}
      </main>
    </>
  )
}

function OrderConfirmation({ order }: Readonly<{ order: Order }>) {
  return (
    <section className="order-confirmation" aria-labelledby="order-confirmation-title">
      <div className="order-confirmation-mark"><ReceiptText aria-hidden="true" /></div>
      <p className="eyebrow">{order.order_number}</p>
      <h2 id="order-confirmation-title">Thank you, {order.customer.first_name}.</h2>
      <p>
        Your pieces are reserved and the order is awaiting manual payment confirmation.
        Keep the order number above for reference.
      </p>
      <dl className="order-confirmation-summary">
        <div><dt>Status</dt><dd>{order.order_status}</dd></div>
        <div><dt>Payment</dt><dd>{order.payment_status}</dd></div>
        <div><dt>Total</dt><dd>{formatMoney(order.total_minor, order.currency)}</dd></div>
      </dl>
      <div className="order-confirmation-lines">
        {order.lines.map((line) => (
          <div key={line.id}>
            <span>{line.quantity} × {line.product_title} · {line.variant_title}</span>
            <strong>{formatMoney(line.line_total_minor, line.currency)}</strong>
          </div>
        ))}
      </div>
      <address>
        <strong>Deliver to {order.shipping_address.recipient_name}</strong>
        <span>{order.shipping_address.line1}</span>
        {order.shipping_address.line2 && <span>{order.shipping_address.line2}</span>}
        <span>{order.shipping_address.postal_code} {order.shipping_address.city}</span>
        <span>{order.shipping_address.country_code}</span>
      </address>
      <a className="button button--primary" href="/#shop">Continue shopping</a>
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
  const delivery = cart.delivery
  return (
    <section className="cart-delivery" aria-labelledby="delivery-title">
      <div>
        <p className="eyebrow">Checkout preparation</p>
        <h2 id="delivery-title">Contact and delivery</h2>
        <p>These details stay attached to this cart and will be revalidated when an order is created.</p>
      </div>
      <form className="delivery-form" onSubmit={onSubmit}>
        <label>Email<input required type="email" name="email" defaultValue={delivery?.email} /></label>
        <div className="field-row">
          <label>First name<input required name="first_name" defaultValue={delivery?.first_name} /></label>
          <label>Last name<input required name="last_name" defaultValue={delivery?.last_name} /></label>
        </div>
        <label>Contact phone<input name="phone" defaultValue={delivery?.phone} /></label>
        <label>Recipient<input required name="recipient_name" defaultValue={delivery?.address.recipient_name} /></label>
        <label>Address<input required name="line1" defaultValue={delivery?.address.line1} /></label>
        <label>Apartment, suite, or studio<input name="line2" defaultValue={delivery?.address.line2} /></label>
        <div className="field-row">
          <label>City<input required name="city" defaultValue={delivery?.address.city} /></label>
          <label>Region<input name="region" defaultValue={delivery?.address.region} /></label>
        </div>
        <div className="field-row field-row--postal">
          <label>Postal code<input required name="postal_code" defaultValue={delivery?.address.postal_code} /></label>
          <label>Country code<input required name="country_code" minLength={2} maxLength={2} defaultValue={delivery?.address.country_code ?? 'PT'} /></label>
        </div>
        <label>Delivery phone<input name="address_phone" defaultValue={delivery?.address.phone} /></label>
        <button className="button button--secondary" type="submit">Save delivery details</button>
      </form>
    </section>
  )
}
