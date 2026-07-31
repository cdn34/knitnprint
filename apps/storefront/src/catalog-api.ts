import {
  createApiClient,
  type Category,
  type Product,
} from '@knitprint/api-client'

const api = createApiClient({
  baseUrl: process.env.API_BASE_URL ?? 'http://127.0.0.1:8080',
})
const apiBaseUrl = process.env.API_BASE_URL ?? 'http://127.0.0.1:8080'

export async function publishedProducts(): Promise<Product[]> {
  try {
    return await api.listProducts()
  } catch {
    return []
  }
}

export async function publishedCategories(): Promise<Category[]> {
  try {
    return await api.listPublicCategories()
  } catch {
    return []
  }
}

export async function publishedCollection(slug: string): Promise<{
  category: Category | null
  products: Product[]
}> {
  try {
    const [categories, products] = await Promise.all([
      api.listPublicCategories(),
      api.listProducts({ category: slug }),
    ])
    return {
      category: categories.find((category) => category.slug === slug) ?? null,
      products,
    }
  } catch {
    return { category: null, products: [] }
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

export function mediaUrl(path: string) {
  return `${apiBaseUrl.replace(/\/$/, '')}${path}`
}
