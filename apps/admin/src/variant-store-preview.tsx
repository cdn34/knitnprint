import { useState } from 'react'
import { ImagePlus } from 'lucide-react'
import type { Product, VariantConfig } from '@knitnprint/api-client'
import { isSizeOption } from './variant-design-controls'

type PreviewRow = { selections: Record<string, string>; price_minor: number; available_quantity: number; retired: boolean }
const money = (amount: number) => new Intl.NumberFormat('pt-PT', { style: 'currency', currency: 'EUR' }).format(amount / 100)

export function VariantStorePreview({ product, config, rows }: { product: Product; config: VariantConfig; rows: PreviewRow[] }) {
  const active = rows.filter(row => !row.retired)
  const [selections, setSelections] = useState(active[0]?.selections ?? {})
  const current = active.find(row => config.options.every(option => row.selections[option.code] === selections[option.code])) ?? active[0]
  const selected = current?.selections ?? {}
  const color = config.options.find(option => option.control === 'swatch')
  const photoIds = color?.values.find(value => value.code === selected[color.code])?.media_ids ?? []
  const photos = config.color_photos && photoIds.length ? product.media.filter(media => photoIds.includes(media.id)) : product.media
  return <div className="variant-shop-preview">
    <div className="variant-shop-art">{photos[0] ? <img src={`/api/admin/product-media/${encodeURIComponent(photos[0].id)}/detail`} alt={photos[0].alt_text || product.title} /> : <ImagePlus size={40} aria-label="Fotografia por adicionar" />}</div>
    <div className="variant-shop-copy">
      <h3>{product.title}</h3>
      <p className="variant-shop-price">{money(current?.price_minor ?? 0)}</p>
      {product.personalization.mode !== 'none' && <p className="variant-shop-personalization">Personalização disponível</p>}
      {config.options.map((option, index) => ({ option, index })).sort((a, b) => Number(b.option.control === 'swatch') - Number(a.option.control === 'swatch')).map(({ option, index }) => {
        const match = (code: string) => active.find(row => row.selections[option.code] === code && config.options.every(axis => axis.code === option.code || row.selections[axis.code] === selected[axis.code]))
          ?? active.find(row => row.selections[option.code] === code && config.options.slice(0, index).every(axis => row.selections[axis.code] === selected[axis.code]))
        return <fieldset key={option.code}><legend>{option.control === 'swatch' ? 'Cor' : isSizeOption(option) ? 'Tamanho' : option.label}</legend>
          <div className="variant-shop-choices">{option.values.map(value => <button type="button" className="variant-shop-choice" key={value.code} aria-pressed={selected[option.code] === value.code} disabled={!match(value.code)} onClick={() => setSelections(match(value.code)!.selections)}>
            {option.control === 'swatch' && <i className="variant-design-swatch" style={{ background: value.color ?? '#000000' }} aria-hidden="true" />}{value.label}
          </button>)}</div>
        </fieldset>
      })}
      {current && <p className={`variant-preview-stock${current.available_quantity <= 0 ? ' backorder' : ''}`} role="status">{current.available_quantity > 0 ? `Disponível · ${current.available_quantity} em stock` : 'Disponível por encomenda'}</p>}
      <p className="variant-shop-selection">{[...config.options].sort((a, b) => Number(b.control === 'swatch') - Number(a.control === 'swatch')).map(option => option.values.find(value => value.code === selected[option.code])?.label).filter(Boolean).join(' · ')}</p>
    </div>
  </div>
}
