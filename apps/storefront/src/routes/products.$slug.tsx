import { createFileRoute, notFound } from '@tanstack/react-router'
import {
  ArrowLeft,
  ChevronLeft,
  ChevronRight,
  CircleCheck,
  PackageCheck,
  PackageX,
  ShieldCheck,
} from 'lucide-react'
import { useEffect, useState } from 'react'
import { announceCartUpdate, cartApi, cartMutationKey } from '../cart-api'
import { ContextualFaqs } from '../components/contextual-faqs'
import { StorefrontAnnouncement, StorefrontFooter, StorefrontHeader } from '../components/storefront-shell'
import {
  mediaUrl,
  preferredVariant,
  publishedProduct,
  variantStock,
} from '../catalog-api'
import { useI18n } from '../i18n'
import { useLocalizedCatalog } from '../i18n/catalog'

export const Route = createFileRoute('/products/$slug')({
  loader: async ({ params }) => {
    const product = await publishedProduct(params.slug)
    if (!product) throw notFound()
    return product
  },
  head: ({ loaderData }) => ({
    meta: [
      { title: loaderData ? `${loaderData.title} — KnitnPrint` : 'KnitnPrint' },
      { name: 'description', content: loaderData?.description ?? '' },
    ],
  }),
  component: ProductPage,
})

function ProductPage() {
  const product = Route.useLoaderData()
  const { t } = useI18n()
  const { priceForVariant, stockText } = useLocalizedCatalog()
  const defaultVariant = preferredVariant(product)
  const [selectedVariantId, setSelectedVariantId] = useState(
    defaultVariant?.id ?? '',
  )
  const variant =
    product.variants.find(({ id }) => id === selectedVariantId) ??
    defaultVariant
  const stock = variant ? variantStock(variant) : null
  const localizedStock = stock ? stockText(stock) : null
  const [cartState, setCartState] = useState<
    'idle' | 'adding' | 'added' | 'error'
  >('idle')
  const [hydrated, setHydrated] = useState(false)
  const [selectedMediaIndex, setSelectedMediaIndex] = useState(0)
  const selectedMedia = product.media[selectedMediaIndex] ?? product.media[0]
  const personalizable = product.personalization.mode !== 'none'

  function showPreviousPhoto() {
    setSelectedMediaIndex((current) =>
      current === 0 ? product.media.length - 1 : current - 1,
    )
  }

  function showNextPhoto() {
    setSelectedMediaIndex((current) =>
      current === product.media.length - 1 ? 0 : current + 1,
    )
  }

  useEffect(() => setHydrated(true), [])

  async function addToCart() {
    if (!variant || !stock || stock.state === 'sold-out') return
    setCartState('adding')
    try {
      const cart = await cartApi.addCartItem(
        { variant_id: variant.id, quantity: 1 },
        cartMutationKey(),
      )
      announceCartUpdate(cart)
      setCartState('added')
    } catch {
      setCartState('error')
    }
  }

  const addToCartLabel = stock?.state === 'sold-out'
    ? localizedStock?.label ?? t('product.unavailable')
    : !hydrated
      ? t('product.preparingCart')
      : cartState === 'adding'
        ? t('product.adding')
        : cartState === 'added'
          ? t('product.added')
          : t('product.addToCart')
  const addToCartButton = <button
    className={`button ${personalizable ? 'button--secondary' : 'button--primary'}`}
    type="button"
    disabled={!hydrated || !variant || stock?.state === 'sold-out' || cartState === 'adding'}
    onClick={addToCart}
  >
    {addToCartLabel}
  </button>

  return (
    <>
      <StorefrontAnnouncement />
      <StorefrontHeader />
      <main className="product-page" id="main-content" tabIndex={-1}>
        <a className="text-link page-back-link" href="/#shop"><ArrowLeft size={17} /> {t('product.backToShop')}</a>
        <div className="product-gallery">
          <div className={`product-detail-art${selectedMedia ? ' product-detail-art--photo' : ''}`}>
            {selectedMedia ? (
              <img
                className="product-detail-photo"
                src={mediaUrl(selectedMedia.detail_url)}
                alt={selectedMedia.alt_text}
              />
            ) : (
              <span
                className="product-form product-form--planter"
                role="img"
                aria-label={t('product.illustration', { name: product.title })}
              />
            )}
            {product.media.length > 1 && (
              <div className="product-gallery-controls">
                <button type="button" onClick={showPreviousPhoto} aria-label="Previous product photo"><ChevronLeft aria-hidden="true" /></button>
                <span aria-live="polite">{selectedMediaIndex + 1} / {product.media.length}</span>
                <button type="button" onClick={showNextPhoto} aria-label="Next product photo"><ChevronRight aria-hidden="true" /></button>
              </div>
            )}
          </div>
          {product.media.length > 1 && (
            <div className="product-gallery-thumbnails" aria-label="Product photos">
              {product.media.map((media, index) => (
                <button
                  type="button"
                  key={media.id}
                  className={index === selectedMediaIndex ? 'selected' : ''}
                  aria-label={`Show product photo ${index + 1}`}
                  aria-pressed={index === selectedMediaIndex}
                  onClick={() => setSelectedMediaIndex(index)}
                >
                  <img src={mediaUrl(media.thumbnail_url)} alt="" />
                </button>
              ))}
            </div>
          )}
        </div>
        <section className="product-detail-copy">
          <p className="eyebrow">{t('product.collection')}</p>
          <h1>{product.title}</h1>
          <p className="product-detail-price">
            {priceForVariant(variant)}
          </p>
          <p className="product-detail-description">{product.description}</p>
          {product.variants.length > 0 && (
            <fieldset className="variant-picker">
              <legend>{t('product.chooseOption')}</legend>
              <div className="variant-options">
                {product.variants.map((option) => {
                  const optionStock = variantStock(option)
                  const localizedOptionStock = stockText(optionStock)
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
                        <small>{priceForVariant(option)} · {t('product.sku')} {option.sku}</small>
                      </span>
                      <em>{localizedOptionStock.label}</em>
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
              {stock.state === 'sold-out' && <PackageX aria-hidden="true" />}
              <span><strong>{localizedStock?.label}</strong></span>
            </div>
          )}
          {personalizable && stock?.state !== 'sold-out' ? <div className="product-purchase-actions">
            <a className="button button--primary personalization-start-button" href={`/products/${product.slug}/personalize`}>Começa a personalizar</a>
            {addToCartButton}
          </div> : addToCartButton}
          <div className="cart-action-status" aria-live="polite">
            {cartState === 'added' && <a href="/cart">{t('product.viewCart')}</a>}
            {cartState === 'error' && (
              <span>{t('product.cartError')}</span>
            )}
          </div>
          <div className="product-promises">
            <span><PackageCheck /> {t('product.smallBatches')}</span>
            <span><ShieldCheck /> {t('product.secureCheckout')}</span>
          </div>
        </section>
        <ContextualFaqs
          id="product-faqs"
          eyebrow={t('product.faqEyebrow')}
          title={t('product.faqTitle')}
          items={[
            { question: t('product.faq1Question'), answer: t('product.faq1Answer') },
            { question: t('product.faq2Question'), answer: t('product.faq2Answer') },
            { question: t('product.faq3Question'), answer: t('product.faq3Answer') },
            { question: t('product.faq4Question'), answer: t('product.faq4Answer') },
          ]}
          className="contextual-faqs--product"
        />
      </main>
      <StorefrontFooter />
    </>
  )
}
