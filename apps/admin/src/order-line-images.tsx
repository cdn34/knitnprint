import type { OrderLine } from '@knitnprint/api-client'
import { Download, Eye, ImageOff } from 'lucide-react'
import { useState } from 'react'

export type SavedOrderImage = {
  mediaId: string
  label: string
  personalized: boolean
  url: string
  downloadUrl: string
}

function record(value: unknown): Record<string, unknown> | undefined {
  return value && typeof value === 'object' && !Array.isArray(value) ? value as Record<string, unknown> : undefined
}

function label(value: unknown, fallback: string): string {
  return typeof value === 'string' && value.trim() ? value : fallback
}

export function savedOrderImages(line: OrderLine): SavedOrderImage[] {
  const previews = Array.isArray(line.personalization_previews) ? line.personalization_previews : []
  const captured = previews.flatMap((value, index) => {
    const preview = record(value)
    if (typeof preview?.media_id !== 'string' || !preview.media_id.trim()) return []
    const mediaId = preview.media_id
    const path = `/api/admin/personalization/media/${encodeURIComponent(mediaId)}`
    return [{ mediaId, label: label(preview.view_label, `Vista ${index + 1}`), personalized: true, url: `${path}/detail`, downloadUrl: `${path}/original` }]
  })
  if (captured.length || line.customization) return captured

  const views = record(line.personalization_context)?.views
  if (!Array.isArray(views)) return []
  for (const value of views) {
    const view = record(value)
    if (typeof view?.media_id !== 'string' || !view.media_id.trim()) continue
    const mediaId = view.media_id
    const url = `/api/admin/order-product/media/${encodeURIComponent(mediaId)}/detail`
    return [{ mediaId, label: label(view.label, 'Fotografia do produto'), personalized: false, url, downloadUrl: url }]
  }
  return []
}

export function OrderLineImages({ images, product, orderNumber }: Readonly<{
  images: SavedOrderImage[]
  product: string
  orderNumber: string
}>) {
  return <section className="order-saved-images" aria-label={`Imagens da encomenda · ${product}`}>
    <header><strong>{images.some(image => image.personalized) ? 'Resultado personalizado' : 'Produto encomendado'}</strong><span>Imagens guardadas com a encomenda</span></header>
    {images.length ? <div className="order-saved-images-grid">{images.map((image, index) => <SavedImage key={`${image.mediaId}:${index}`} image={image} product={product} orderNumber={orderNumber} index={index} />)}</div>
      : <p className="order-saved-image-unavailable"><ImageOff aria-hidden="true" />Esta encomenda não tem uma fotografia do produto guardada. Os restantes detalhes continuam disponíveis.</p>}
  </section>
}

function SavedImage({ image, product, orderNumber, index }: Readonly<{
  image: SavedOrderImage
  product: string
  orderNumber: string
  index: number
}>) {
  const [failed, setFailed] = useState(false)
  const description = `${product} · ${image.label}`
  return <figure className="order-saved-image">
    {failed ? <div className="order-saved-image-unavailable" role="status">
      <ImageOff aria-hidden="true" />
      <p>Não foi possível carregar esta imagem. Os detalhes da encomenda continuam guardados.</p>
      <button type="button" onClick={() => setFailed(false)}>Tentar novamente</button>
    </div> : <a className="order-saved-image-preview" href={image.url} target="_blank" rel="noopener noreferrer" aria-label={`Abrir imagem · ${description}`}>
      <img src={image.url} alt={description} loading="lazy" onError={() => setFailed(true)} />
    </a>}
    <figcaption>
      <strong>{image.label}</strong>
      <div className="order-saved-image-actions">
        <a href={image.url} target="_blank" rel="noopener noreferrer" aria-label={`Ampliar imagem · ${description}`}><Eye aria-hidden="true" />Ampliar</a>
        <a href={image.downloadUrl} download={`${orderNumber}-imagem-${index + 1}-${image.mediaId}${image.personalized ? '.png' : '.webp'}`} aria-label={`Descarregar imagem · ${description}`}><Download aria-hidden="true" />Descarregar</a>
      </div>
    </figcaption>
  </figure>
}
