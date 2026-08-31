import { ApiError } from '@knitprint/api-client'
import { createFileRoute, notFound, redirect } from '@tanstack/react-router'
import { ArrowLeft, Eye, ShoppingBag } from 'lucide-react'
import { useState } from 'react'
import { announceCartUpdate, cartApi, cartMutationKey } from '../cart-api'
import { mediaUrl, preferredVariant, publishedProduct, variantStock } from '../catalog-api'
import { ProductPersonalizer, type CustomerCustomization } from '../components/product-personalizer'
import { StorefrontAnnouncement, StorefrontHeader } from '../components/storefront-shell'

export const Route = createFileRoute('/products/$slug_/personalize')({
  loader: async ({ params }) => {
    const product = await publishedProduct(params.slug)
    if (!product || product.personalization.mode === 'none') throw notFound()
    if (!product.variants.some(({ available_quantity }) => available_quantity > 0)) {
      throw redirect({ to: '/products/$slug', params: { slug: params.slug }, replace: true })
    }
    return product
  },
  head: ({ loaderData }) => ({ meta: [{ title: loaderData ? `Personalizar ${loaderData.title} — KnitnPrint` : 'KnitnPrint' }] }),
  component: PersonalizeProductPage,
})

function PersonalizeProductPage() {
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
        ? 'Este produto está esgotado. Atualiza o stock no administrador antes de o adicionares ao carrinho.'
        : error instanceof ApiError && error.body.error.code === 'invalid_customization'
          ? 'Não foi possível validar esta personalização. Revê os elementos e tenta novamente.'
          : 'Não foi possível adicionar o produto ao carrinho. Tenta novamente.')
    }
  }

  function requestAddToCart() {
    if (!variant || soldOut || status === 'adding') return
    setPreviewOpen(false)
    if (design.missing.length) { setConfirmingIncomplete(true); return }
    void addToCart()
  }

  const addToCartLabel = soldOut ? 'Produto esgotado' : status === 'adding' ? 'A adicionar…' : status === 'added' ? 'Adicionado ao carrinho' : 'Adicionar ao carrinho'

  return <>
    <StorefrontAnnouncement />
    <StorefrontHeader />
    <main className="personalization-page" id="main-content">
      <header className="personalization-page-header">
        <a className="text-link" href={`/products/${product.slug}`}><ArrowLeft /> Voltar ao produto</a>
        <div><p>Estúdio de personalização</p><h1>{product.title}</h1><span>Cria e confirma a tua composição antes de adicionares ao carrinho.</span></div>
        {product.variants.length > 1 && <label>Opção<select value={variant?.id} onChange={(event) => { setVariantId(event.target.value); setStatus('idle'); setErrorMessage('') }}>{product.variants.map((option) => <option key={option.id} value={option.id} disabled={variantStock(option).state === 'sold-out'}>{option.title}</option>)}</select></label>}
      </header>
      <ProductPersonalizer config={product.personalization} productMedia={product.media.map((media) => ({ id: media.id, url: mediaUrl(media.detail_url) }))} onChange={setDesign} previewOpen={previewOpen} onPreviewClose={() => setPreviewOpen(false)} onAddToCart={requestAddToCart} addToCartDisabled={!variant || soldOut || status === 'adding'} addToCartLabel={addToCartLabel} />
      <div className="personalization-checkout-bar">
        <span>{soldOut ? 'Este produto está esgotado. Podes personalizá-lo, mas não adicioná-lo ao carrinho enquanto não houver stock.' : design.ready ? 'A personalização está pronta.' : 'A personalização é opcional. Podes avançar sem preencher tudo.'}</span>
        <button className="button button--secondary personalization-preview-button" type="button" onClick={() => setPreviewOpen(true)}><Eye /> Pré-visualizar resultado</button>
        <button className="button button--primary" type="button" disabled={!variant || soldOut || status === 'adding'} onClick={requestAddToCart}><ShoppingBag />{addToCartLabel}</button>
        {status === 'added' && <a className="text-link" href="/cart">Ver carrinho</a>}
        {status === 'error' && <strong role="alert">{errorMessage}</strong>}
      </div>
      {confirmingIncomplete && <div className="personalization-confirmation-backdrop" role="presentation" onKeyDown={(event) => { if (event.key === 'Escape') setConfirmingIncomplete(false) }}><section className="personalization-confirmation" role="alertdialog" aria-modal="true" aria-labelledby="incomplete-personalization-title" aria-describedby="incomplete-personalization-description"><span>Confirmação</span><h2 id="incomplete-personalization-title">Queres avançar sem completar?</h2><p id="incomplete-personalization-description">Ainda falta {design.missing.slice(0, 3).join(', ')}{design.missing.length > 3 ? ` e mais ${design.missing.length - 3} opção(ões)` : ''}. O produto será colocado no carrinho apenas com as opções que preencheste.</p><div><button className="button button--secondary" type="button" autoFocus onClick={() => setConfirmingIncomplete(false)}>Continuar a editar</button><button className="button button--primary" type="button" onClick={() => void addToCart()}>Sim, adicionar</button></div></section></div>}
    </main>
  </>
}
