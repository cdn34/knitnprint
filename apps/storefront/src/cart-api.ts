import { createApiClient } from '@knitprint/api-client'

export const cartApi = createApiClient()

export function cartMutationKey() {
  return crypto.randomUUID()
}

export function formatMoney(minor: number, currency: string) {
  return new Intl.NumberFormat('en', {
    style: 'currency',
    currency,
  }).format(minor / 100)
}
