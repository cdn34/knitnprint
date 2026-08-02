import { createFileRoute, notFound } from '@tanstack/react-router'
import {
  ArrowLeft,
  CircleCheck,
  PackageCheck,
  PackageX,
  ShieldCheck,
  Sparkles,
  TriangleAlert,
} from 'lucide-react'
import { useState } from 'react'
import {
  mediaUrl,
  preferredVariant,
  publishedProduct,
  variantPrice,
  variantStock,
} from '../catalog-api'

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
  const defaultVariant = preferredVariant(product)
  const [selectedVariantId, setSelectedVariantId] = useState(
    defaultVariant?.id ?? '',
  )
  const variant =
    product.variants.find(({ id }) => id === selectedVariantId) ??
    defaultVariant
  const stock = variant ? variantStock(variant) : null

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
          <p className="product-detail-price">
            {variant ? variantPrice(variant) : 'Price unavailable'}
          </p>
          <p className="product-detail-description">{product.description}</p>
          {product.variants.length > 0 && (
            <fieldset className="variant-picker">
              <legend>Choose an option</legend>
              <div className="variant-options">
                {product.variants.map((option) => {
                  const optionStock = variantStock(option)
                  const selected = option.id === variant?.id
                  return (
                    <label
                      className={`variant-option${selected ? ' selected' : ''}${optionStock.state === 'sold-out' ? ' sold-out' : ''}`}
                      key={option.id}
                    >
                      <input
                        type="radio"
                        name="product-variant"
                        value={option.id}
                        checked={selected}
                        disabled={optionStock.state === 'sold-out'}
                        onChange={() => setSelectedVariantId(option.id)}
                      />
                      <span>
                        <strong>{option.title}</strong>
                        <small>{variantPrice(option)} · SKU {option.sku}</small>
                      </span>
                      <em>{optionStock.label}</em>
                    </label>
                  )
                })}
              </div>
            </fieldset>
          )}
          {stock && (
            <div
              className={`stock-status stock-status--${stock.state}`}
              role="status"
              aria-live="polite"
            >
              {stock.state === 'available' && <CircleCheck aria-hidden="true" />}
              {stock.state === 'low' && <TriangleAlert aria-hidden="true" />}
              {stock.state === 'sold-out' && <PackageX aria-hidden="true" />}
              <span><strong>{stock.label}</strong><small>{stock.detail}</small></span>
            </div>
          )}
          <button
            className="button button--primary"
            type="button"
            disabled
          >
            {stock?.state === 'sold-out'
              ? 'Currently unavailable'
              : 'Add to cart · coming in Phase 5'}
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
