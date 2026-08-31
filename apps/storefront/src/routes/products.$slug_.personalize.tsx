import { createFileRoute, notFound } from '@tanstack/react-router'
import { ArrowLeft, ShoppingBag } from 'lucide-react'
import { useState } from 'react'
import { cartApi, cartMutationKey } from '../cart-api'
import { mediaUrl, preferredVariant, publishedProduct, variantStock } from '../catalog-api'
import { ProductPersonalizer, type CustomerCustomization } from '../components/product-personalizer'
import { StorefrontAnnouncement, StorefrontHeader } from '../components/storefront-shell'

export const Route = createFileRoute('/products/$slug_/personalize')({
  loader: async ({ params }) => {
    const product = await publishedProduct(params.slug)
    if (!product || product.personalization.mode === 'none') throw notFound()
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
  const [design, setDesign] = useState<{ customization: CustomerCustomization | null; mediaId?: string; ready: boolean }>({ customization: null, ready: false })
  const [status, setStatus] = useState<'idle' | 'adding' | 'added' | 'error'>('idle')
  const selectedMedia = product.media.find(({ id }) => id === product.personalization.preview_media_id) ?? product.media[0]

  async function addToCart() {
    if (!variant || !design.ready || !design.customization) return
    setStatus('adding')
    try {
      await cartApi.addCartItem({ variant_id: variant.id, quantity: 1, customization: design.customization, customization_media_asset_id: design.mediaId }, cartMutationKey())
      setStatus('added')
    } catch { setStatus('error') }
  }

  return <>
    <StorefrontAnnouncement />
    <StorefrontHeader />
    <main className="personalization-page" id="main-content">
      <header className="personalization-page-header">
        <a className="text-link" href={`/products/${product.slug}`}><ArrowLeft /> Voltar ao produto</a>
        <div><p>Estúdio de personalização</p><h1>{product.title}</h1><span>Cria e confirma a tua composição antes de adicionares ao carrinho.</span></div>
        {product.variants.length > 1 && <label>Opção<select value={variant?.id} onChange={(event) => setVariantId(event.target.value)}>{product.variants.map((option) => <option key={option.id} value={option.id} disabled={variantStock(option).state === 'sold-out'}>{option.title}</option>)}</select></label>}
      </header>
      <ProductPersonalizer config={product.personalization} productImage={selectedMedia ? mediaUrl(selectedMedia.detail_url) : undefined} onChange={setDesign} />
      <div className="personalization-checkout-bar">
        <span>{design.ready ? 'A personalização está pronta.' : 'Completa as opções de personalização para continuar.'}</span>
        <button className="button button--primary" type="button" disabled={!design.ready || !variant || status === 'adding'} onClick={addToCart}><ShoppingBag />{status === 'adding' ? 'A adicionar…' : status === 'added' ? 'Adicionado ao carrinho' : 'Adicionar ao carrinho'}</button>
        {status === 'added' && <a className="text-link" href="/cart">Ver carrinho</a>}
        {status === 'error' && <strong role="alert">Não foi possível guardar a personalização. Tenta novamente.</strong>}
      </div>
    </main>
  </>
}
