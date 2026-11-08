export function productSlugBase(slug: string, sku: string) {
  const suffix = `-${sku.toLowerCase()}`
  return slug.endsWith(suffix) ? slug.slice(0, -suffix.length) : slug
}
export function productSlugUrl(base: string, sku?: string) {
  return sku ? `${productSlugBase(base, sku)}-${sku.toLowerCase()}` : `${base}-[product-sku]`
}
