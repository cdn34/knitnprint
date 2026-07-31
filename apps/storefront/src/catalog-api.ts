import { createApiClient, type Product } from '@knitprint/api-client'

const api = createApiClient({
  baseUrl: process.env.API_BASE_URL ?? 'http://127.0.0.1:8080',
})

export async function publishedProducts(): Promise<Product[]> {
  try {
    return await api.listProducts()
  } catch {
    return []
  }
}

export async function publishedProduct(slug: string): Promise<Product | null> {
  try {
    return await api.product(slug)
  } catch {
    return null
  }
}

export function productPrice(product: Product) {
  const variant = product.variants[0]
  if (!variant) return 'Price unavailable'
  return new Intl.NumberFormat('en', {
    style: 'currency',
    currency: variant.currency,
  }).format(variant.price_minor / 100)
}
