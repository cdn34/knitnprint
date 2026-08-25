import {
  createApiClient,
  type Category,
  type Product,
  type Variant,
} from '@knitprint/api-client'

const configuredApiBaseUrl = process.env.API_BASE_URL
const api = createApiClient({
  baseUrl: configuredApiBaseUrl ?? 'http://127.0.0.1:8080',
})

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
  const variant = preferredVariant(product)
  if (!variant) return 'Price unavailable'
  return variantPrice(variant)
}

export function variantPrice(variant: Variant) {
  return new Intl.NumberFormat('en', {
    style: 'currency',
    currency: variant.currency,
  }).format(variant.price_minor / 100)
}

export function preferredVariant(product: Product) {
  return (
    product.variants.find(({ available_quantity }) => available_quantity > 0) ??
    product.variants[0]
  )
}

export type StockPresentation = {
  state: 'available' | 'low' | 'sold-out'
  label: string
  detail: string
}

export function variantStock(variant: Variant): StockPresentation {
  if (variant.available_quantity <= 0) {
    return {
      state: 'sold-out',
      label: 'Sold out',
      detail: 'This option is currently unavailable.',
    }
  }
  if (variant.low_stock) {
    return {
      state: 'low',
      label: `Only ${variant.available_quantity} left`,
      detail: 'A small number remains in the studio.',
    }
  }
  return {
    state: 'available',
    label: 'In stock',
    detail: 'Available from our studio inventory.',
  }
}

export function productStock(product: Product) {
  const variant = preferredVariant(product)
  return variant ? variantStock(variant) : null
}

export function mediaUrl(path: string) {
  if (/^https?:\/\//.test(path)) return path
  if (!configuredApiBaseUrl) return path
  return `${configuredApiBaseUrl.replace(/\/$/, '')}${path}`
}
