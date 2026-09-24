import { ApiError } from '@knitnprint/api-client'
import { createFileRoute, notFound, redirect } from '@tanstack/react-router'
import { ArrowLeft, Eye, ShoppingBag } from 'lucide-react'
import { useState } from 'react'
import { announceCartUpdate, cartApi, cartMutationKey } from '../cart-api'
import { mediaUrl, preferredVariant, publishedProduct, variantStock } from '../catalog-api'
import { ProductPersonalizer, type CustomerCustomization } from '../components/product-personalizer'
import { StorefrontAnnouncement, StorefrontHeader } from '../components/storefront-shell'
import { useI18n } from '../i18n'

export const Route = createFileRoute('/products/$slug_/personalize')({
  loader: async ({ params }) => {
    const product = await publishedProduct(params.slug)
    if (!product || product.personalization.mode === 'none') throw notFound()
    if (!product.variants.some(({ available_quantity }) => available_quantity > 0)) {
      throw redirect({ to: '/products/$slug', params: { slug: params.slug }, replace: true })
    }
    return product
  },
  head: ({ loaderData }) => ({ meta: [{ title: loaderData ? `${loaderData.title} — KnitNPrint` : 'KnitNPrint' }] }),
  component: PersonalizeProductPage,
})

function PersonalizeProductPage() {
  const { t } = useI18n()
  const product = Route.useLoaderData()
  const defaultVariant = preferredVariant(product)
  const [variantId, setVariantId] = useState(defaultVariant?.id ?? '')
  const variant = product.variants.find(({ id }) => id === variantId) ?? defaultVariant
  const [design, setDesign] = useState<{ customization: CustomerCustomization | null; mediaIds: string[]; ready: boolean; missing: string[] }>({ customization: null, mediaIds: [], ready: false, missing: [] })
  const [status, setStatus] = useState<'idle' | 'adding' | 'added' | 'error'>('idle')
  const [errorMessage, setErrorMessage] = useState('')
  const [confirmingIncomplete, setConfirmingIncomplete] = useState(false)
  const [previewOpen, setPreviewOpen] = useState(false)
  const stock = variant ? variantStock(variant) : null
  const soldOut = !stock || stock.state === 'sold-out'

  async function addToCart() {
    if (!variant) return
    setStatus('adding')
    setErrorMessage('')
    setConfirmingIncomplete(false)
    try {
      const cart = await cartApi.addCartItem({ variant_id: variant.id, quantity: 1, ...(design.customization ? { customization: design.customization } : {}), ...(design.mediaIds.length ? { customization_media_asset_ids: design.mediaIds } : {}) }, cartMutationKey())
      announceCartUpdate(cart)
      setStatus('added')
    } catch (error) {
      setStatus('error')
      setErrorMessage(error instanceof ApiError && error.body.error.code === 'insufficient_stock'
        ? t('personalization.stockError')
        : error instanceof ApiError && error.body.error.code === 'invalid_customization'
          ? t('personalization.validationError')
          : t('personalization.cartError'))
    }
  }

  function requestAddToCart() {
    if (!variant || soldOut || status === 'adding') return
    setPreviewOpen(false)
    if (design.missing.length) { setConfirmingIncomplete(true); return }
    void addToCart()
  }

  const addToCartLabel = soldOut ? t('personalization.soldOut') : status === 'adding' ? t('personalization.adding') : status === 'added' ? t('personalization.added') : t('personalization.addToCart')
  const creationTitle = t('personalization.creationTitle', { product: product.title })
  const extraMissing = design.missing.length > 3 ? t('personalization.moreOptions', { count: design.missing.length - 3 }) : ''

  return <>
    <StorefrontAnnouncement />
    <StorefrontHeader />
    <main className="personalization-page" id="main-content">
      <header className="personalization-page-header">
        <div className="personalization-page-toolbar">
          <a className="text-link" href={`/products/${product.slug}`}><ArrowLeft /> {t('personalization.backToProduct')}</a>
          <ol className="personalization-progress" aria-label={t('personalization.progress')}>
            <li className="completed"><span>1</span><b>{t('personalization.productStep')}</b></li>
            <li className="current" aria-current="step"><span>2</span><b>{t('personalization.personalizeStep')}</b></li>
            <li><span>3</span><b>{t('personalization.reviewStep')}</b></li>
          </ol>
          {product.variants.length > 1 ? <label>{t('personalization.option')}<select value={variant?.id} onChange={(event) => { setVariantId(event.target.value); setStatus('idle'); setErrorMessage('') }}>{product.variants.map((option) => <option key={option.id} value={option.id} disabled={variantStock(option).state === 'sold-out'}>{option.title}</option>)}</select></label> : <span aria-hidden="true" />}
        </div>
        <div className="personalization-page-intro"><p>{t('personalization.studio')}</p><h1>{creationTitle}</h1><span>{t('personalization.intro')}</span></div>
      </header>
      <ProductPersonalizer config={product.personalization} productMedia={product.media.map((media) => ({ id: media.id, url: mediaUrl(media.detail_url) }))} onChange={setDesign} previewOpen={previewOpen} onPreviewClose={() => setPreviewOpen(false)} onAddToCart={requestAddToCart} addToCartDisabled={!variant || soldOut || status === 'adding'} addToCartLabel={addToCartLabel} />
      <div className="personalization-checkout-bar">
        <span>{t(soldOut ? 'personalization.soldOutDetail' : design.ready ? 'personalization.ready' : 'personalization.optionalReady')}</span>
        <button className="button button--secondary personalization-preview-button" type="button" onClick={() => setPreviewOpen(true)}><Eye /> {t('personalization.previewResult')}</button>
        <button className="button button--primary" type="button" disabled={!variant || soldOut || status === 'adding'} onClick={requestAddToCart}><ShoppingBag />{addToCartLabel}</button>
        {status === 'added' && <a className="text-link" href="/cart">{t('personalization.viewCart')}</a>}
        {status === 'error' && <strong role="alert">{errorMessage}</strong>}
      </div>
      {confirmingIncomplete && <div className="personalization-confirmation-backdrop" role="presentation" onKeyDown={(event) => { if (event.key === 'Escape') setConfirmingIncomplete(false) }}><section className="personalization-confirmation" role="alertdialog" aria-modal="true" aria-labelledby="incomplete-personalization-title" aria-describedby="incomplete-personalization-description"><span>{t('personalization.confirmation')}</span><h2 id="incomplete-personalization-title">{t('personalization.incompleteTitle')}</h2><p id="incomplete-personalization-description">{t('personalization.incompleteDescription', { items: design.missing.slice(0, 3).join(', '), extra: extraMissing })}</p><div><button className="button button--secondary" type="button" autoFocus onClick={() => setConfirmingIncomplete(false)}>{t('personalization.continueEditing')}</button><button className="button button--primary" type="button" onClick={() => void addToCart()}>{t('personalization.confirmAdd')}</button></div></section></div>}
    </main>
  </>
}
