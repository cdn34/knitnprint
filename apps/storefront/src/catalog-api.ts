import {
  createApiClient,
  type Category,
  type Product,
  type ProductFeedbackSummary,
  type CreateProductFeedbackRequest,
  type SubmittedProductFeedback,
  type Variant,
} from '@knitprint/api-client'

const configuredApiBaseUrl = process.env.API_BASE_URL
const api = createApiClient({
  baseUrl: configuredApiBaseUrl ?? 'http://127.0.0.1:8080',
})
const browserApi = createApiClient()

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

export async function publishedProductFeedback(
  slug: string,
): Promise<ProductFeedbackSummary> {
  try {
    return await api.productFeedback(slug)
  } catch {
    return {
      average_rating: null,
      total_reviews: 0,
      rating_counts: [5, 4, 3, 2, 1].map((rating) => ({ rating, count: 0 })),
      reviews: [],
    }
  }
}

export function submitProductFeedback(
  slug: string,
  input: CreateProductFeedbackRequest,
): Promise<SubmittedProductFeedback> {
  return browserApi.submitProductFeedback(slug, input)
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
  }).format(variant.display_price_minor / 100)
}

export function preferredVariant(product: Product) {
  return product.variants[0]
}

export type StockPresentation = {
  state: 'available' | 'sold-out'
  label: string
  detail: string
}

export function variantStock(_variant: Variant): StockPresentation {
  return {
    state: 'available',
    label: 'In stock',
    detail: 'Available to order.',
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
