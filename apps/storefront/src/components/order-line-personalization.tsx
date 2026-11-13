import type { OrderLine } from '@knitnprint/api-client'
import { ImagePlus, Sparkles } from 'lucide-react'
import { useState } from 'react'
import { useI18n } from '../i18n'
import { personalizationSummary } from '../personalization-summary'

function record(value: unknown): Record<string, unknown> | undefined {
  return value && typeof value === 'object' && !Array.isArray(value) ? value as Record<string, unknown> : undefined
}

function label(value: unknown) {
  return typeof value === 'string' && value.trim() ? value : undefined
}

export function OrderLinePersonalization({ orderId, line }: Readonly<{ orderId: string; line: OrderLine }>) {
  const { t } = useI18n()
  const customization = record(line.customization)
  const mediaIds = [...new Set([...(line.customization_media_asset_ids ?? []), ...(line.customization_media_asset_id ? [line.customization_media_asset_id] : [])])]
  const modern = Array.isArray(customization?.areas)
  const rawAreas = modern ? customization!.areas as unknown[] : customization ? [customization] : []
  const areas = rawAreas.flatMap((raw, index) => {
    const area = record(raw)
    if (!area) return []
    const text = record(area.text)
    const content = label(text?.content)
    const photo = Boolean(record(area.photo)) || (!modern && mediaIds.length > 0)
    if (!content && !photo) return []
    return [{ index, view: label(area.view_label), area: label(area.area_label) ?? t('personalization.areaFallback', { number: index + 1 }), content, photo, font: label(text?.font), color: label(text?.color) }]
  })
  const savedPreviews = Array.isArray(line.personalization_previews) ? line.personalization_previews.flatMap((raw, index) => {
    const preview = record(raw)
    const id = label(preview?.media_id)
    return id ? [{ id, label: label(preview?.view_label) ?? t('personalization.viewFallback', { number: index + 1 }) }] : []
  }) : []
  const previews = savedPreviews.length ? savedPreviews : mediaIds.map((id, index) => ({ id, label: t('order.uploadedPhoto', { number: index + 1 }) }))
  if (!areas.length && !previews.length) return null
  const colors: Record<string, string> = {
    '#111111': t('personalization.colorBlack'), '#ffffff': t('personalization.colorWhite'),
    '#9c5263': t('personalization.colorDustyRose'), '#1f4f78': t('personalization.colorBlue'), '#b3232f': t('personalization.colorRed'),
  }

  return <section className="order-line-personalization" aria-label={t('order.personalizationDetails', { name: line.product_title })}>
    <p className="order-personalized-badge"><Sparkles size={14} aria-hidden="true" />{t('personalization.summary')}</p>
    {modern && areas.length > 0 && <p className="order-personalization-summary">{personalizationSummary(line.customization, mediaIds, t)}</p>}
    {areas.length > 0 && <dl className="order-personalization-areas">
      {areas.map(area => <div key={area.index}>
        <dt>{[area.view, area.area].filter(Boolean).join(' · ')}</dt>
        <dd>
          {area.photo && <p className="order-personalization-photo"><ImagePlus size={15} aria-hidden="true" />{t('order.photoAdded')}</p>}
          {area.content && <blockquote className="order-personalization-text">“{area.content}”</blockquote>}
          {area.content && (area.font || area.color) && <p className="order-personalization-style">
            {area.font && <span>{t('order.fontLabel')}: {area.font}</span>}
            {area.color && <span>{t('order.colorLabel')}: {colors[area.color.toLowerCase()] ?? area.color}</span>}
          </p>}
        </dd>
      </div>)}
    </dl>}
    {previews.length > 0 && <div className="order-personalization-previews">{previews.map((preview, index) => <PersonalizationImage key={`${preview.id}:${index}`} orderId={orderId} product={line.product_title} mediaId={preview.id} label={preview.label} />)}</div>}
  </section>
}

function PersonalizationImage({ orderId, mediaId, label: view, product }: Readonly<{ orderId: string; mediaId: string; label: string; product: string }>) {
  const { t } = useI18n()
  const [failed, setFailed] = useState(false)
  const url = `/api/orders/${encodeURIComponent(orderId)}/personalization/${encodeURIComponent(mediaId)}`
  return <figure className="order-personalization-preview">
    {failed ? <p className="order-personalization-unavailable">{t('order.previewUnavailable')}</p> : <a href={url} target="_blank" rel="noopener noreferrer" aria-label={t('order.openPreview', { view })}>
      <img src={url} alt={t('order.previewAlt', { name: product, view })} loading="lazy" onError={() => setFailed(true)} />
    </a>}
    <figcaption>{view}</figcaption>
  </figure>
}
