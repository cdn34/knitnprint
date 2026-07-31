import { createFileRoute, notFound } from '@tanstack/react-router'
import { ArrowLeft, PackageCheck, ShieldCheck, Sparkles } from 'lucide-react'
import { mediaUrl, productPrice, publishedProduct } from '../catalog-api'

export const Route = createFileRoute('/products/$slug')({
  loader: async ({ params }) => {
    const product = await publishedProduct(params.slug)
    if (!product) throw notFound()
    return product
  },
  head: ({ loaderData }) => ({
    meta: [
      { title: loaderData ? `${loaderData.title} — KnitPrint` : 'KnitPrint' },
      { name: 'description', content: loaderData?.description ?? '' },
    ],
  }),
  component: ProductPage,
})

function ProductPage() {
  const product = Route.useLoaderData()
  const variant = product.variants[0]

  return (
    <>
      <div className="announcement">
        <Sparkles size={14} aria-hidden="true" />
        Small-batch objects, made slowly in Portugal
      </div>
      <header className="site-header product-header">
        <a className="brand" href="/" aria-label="KnitPrint home">
          <img src="/knitprint-wordmark.webp" alt="KnitPrint" width="750" height="195" />
        </a>
        <a className="text-link" href="/#shop"><ArrowLeft size={17} /> Back to shop</a>
      </header>
      <main className="product-page" id="main-content" tabIndex={-1}>
        <div className="product-detail-art">
          {product.media[0] ? (
            <img
              className="product-detail-photo"
              src={mediaUrl(product.media[0].detail_url)}
              alt={product.media[0].alt_text}
            />
          ) : (
            <span
              className="product-form product-form--planter"
              role="img"
              aria-label={`${product.title} product illustration`}
            />
          )}
        </div>
        <section className="product-detail-copy">
          <p className="eyebrow">KnitPrint collection</p>
          <h1>{product.title}</h1>
          <p className="product-detail-price">{productPrice(product)}</p>
          <p className="product-detail-description">{product.description}</p>
          {variant && (
            <div className="variant-card">
              <span>{variant.title}</span>
              <small>SKU {variant.sku}</small>
            </div>
          )}
          <button className="button button--primary" type="button" disabled>
            Add to cart · coming in Phase 5
          </button>
          <div className="product-promises">
            <span><PackageCheck /> Made in small batches</span>
            <span><ShieldCheck /> Secure checkout</span>
          </div>
        </section>
      </main>
    </>
  )
}
