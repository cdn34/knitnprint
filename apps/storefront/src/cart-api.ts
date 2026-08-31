import { createApiClient, type Cart } from '@knitprint/api-client'

export const cartApi = createApiClient()
export const CART_COUNT_UPDATED = 'knitprint:cart-count-updated'

export function announceCartUpdate(cart: Pick<Cart, 'item_count'>) {
  if (typeof window === 'undefined') return
  window.dispatchEvent(new CustomEvent<number>(CART_COUNT_UPDATED, { detail: cart.item_count }))
}

export function cartMutationKey() {
  return crypto.randomUUID()
}

export function formatMoney(minor: number, currency: string) {
  return new Intl.NumberFormat('en', {
    style: 'currency',
    currency,
  }).format(minor / 100)
}
