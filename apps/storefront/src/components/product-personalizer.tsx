import type { PersonalizationConfig } from '@knitnprint/api-client'
import { ImagePlus, Move, ShoppingBag, Type } from 'lucide-react'
import { type KeyboardEvent, type PointerEvent as ReactPointerEvent, type ReactNode, useEffect, useMemo, useRef, useState } from 'react'
import { cartApi } from '../cart-api'
import { useI18n } from '../i18n'
import { configuredViews, type ElementFrame, type PrintView, type PrintArea } from '../personalization-views'
import { toBlob } from 'html-to-image'

const SUPPORTED_FONTS = ['Roboto', 'Montserrat', 'Playfair Display', 'Dancing Script', 'Pacifico'] as const
const DEFAULT_COLORS = ['#111111', '#ffffff', '#9c5263', '#1f4f78', '#b3232f']
type Interaction = { pointerX: number; pointerY: number; frame: ElementFrame; handle: 'move' | 'nw' | 'ne' | 'sw' | 'se' }
type ProductMediaForPersonalizer = { id: string; url: string }

function normalizedFrame(frame: ElementFrame): ElementFrame {
  const round = (value: number) => Math.round(value * 10_000) / 10_000
  const x = round(Math.max(0, Math.min(100, frame.x)))
  const y = round(Math.max(0, Math.min(100, frame.y)))
  return {
    x,
    y,
    width: round(Math.max(0.0001, Math.min(100 - x, frame.width))),
    height: round(Math.max(0.0001, Math.min(100 - y, frame.height))),
  }
}

const designKey = (viewId: string, areaId: string) => `${viewId}:${areaId}`

function articleReferenceSnapshot(view: PrintView, area: PrintArea) {
  const reference = view.articleReference
  if (!reference) return undefined
  const availableWidth = reference.physicalWidthCm * area.width / reference.width
  return {
    article_width_cm: reference.physicalWidthCm,
    article_height_cm: reference.physicalHeightCm,
    print_left_cm: Math.round((reference.physicalWidthCm * (area.x - reference.x) / reference.width + Math.max(0, availableWidth - area.physicalWidthCm) / 2) * 100) / 100,
    print_top_cm: Math.round(reference.physicalHeightCm * (area.y - reference.y) / reference.height * 100) / 100,
  }
}

export type CustomerCustomization = {
  version: 7
  previews?: Array<{ view_id: string; view_label: string; media_id: string }>
  areas: Array<{
    view_id: string
    view_label: string
    area_id: string
    area_label: string
    print_width_cm: number
    print_height_cm: number
    article_reference?: { article_width_cm: number; article_height_cm: number; print_left_cm: number; print_top_cm: number }
    text?: { content: string; font: string; color: string; size: number; x: number; y: number; width: number; height: number }
    photo?: { media_id: string; x: number; y: number; width: number; height: number; crop_x: number; crop_y: number; scale: number }
  }>
}

type AreaDesign = {
  text: string
  font: string
  color: string
  size: number
  textFrame: ElementFrame
  photoUrl?: string
  mediaId?: string
  photoFrame: ElementFrame
}

function DesignElement({ frame, kind, label, selected, onSelect, onChange, children }: Readonly<{
  frame: ElementFrame
  kind: 'photo' | 'text'
  label: string
  selected: boolean
  onSelect: () => void
  onChange: (frame: ElementFrame) => void
  children: ReactNode
}>) {
  const { t } = useI18n()
  const element = useRef<HTMLDivElement>(null)
  const interaction = useRef<Interaction | undefined>(undefined)

  function start(event: ReactPointerEvent<HTMLElement>, handle: Interaction['handle']) {
    event.preventDefault()
    event.stopPropagation()
    onSelect()
    event.currentTarget.setPointerCapture(event.pointerId)
    interaction.current = { pointerX: event.clientX, pointerY: event.clientY, frame, handle }
  }

  function move(event: ReactPointerEvent<HTMLElement>) {
    const active = interaction.current
    const bounds = element.current?.parentElement?.getBoundingClientRect()
    if (!active || !bounds) return
    const dx = (event.clientX - active.pointerX) / bounds.width * 100
    const dy = (event.clientY - active.pointerY) / bounds.height * 100
    let { x, y, width, height } = active.frame
    if (active.handle === 'move') {
      x = Math.max(0, Math.min(100 - width, x + dx))
      y = Math.max(0, Math.min(100 - height, y + dy))
    } else {
      if (active.handle.includes('w')) { const next = Math.max(0, Math.min(x + width - 10, x + dx)); width += x - next; x = next }
      if (active.handle.includes('e')) width = Math.max(10, Math.min(100 - x, width + dx))
      if (active.handle.includes('n')) { const next = Math.max(0, Math.min(y + height - 10, y + dy)); height += y - next; y = next }
      if (active.handle.includes('s')) height = Math.max(10, Math.min(100 - y, height + dy))
    }
    onChange({ x, y, width, height })
  }

  function moveWithKeyboard(event: KeyboardEvent<HTMLDivElement>) {
    const movement = event.shiftKey ? 5 : 2
    const offsets: Record<string, { x: number; y: number }> = { ArrowLeft: { x: -movement, y: 0 }, ArrowRight: { x: movement, y: 0 }, ArrowUp: { x: 0, y: -movement }, ArrowDown: { x: 0, y: movement } }
    const offset = offsets[event.key]
    if (!offset || event.target !== event.currentTarget) return
    event.preventDefault()
    onChange({ ...frame, x: Math.max(0, Math.min(100 - frame.width, frame.x + offset.x)), y: Math.max(0, Math.min(100 - frame.height, frame.y + offset.y)) })
  }

  function stop() { interaction.current = undefined }

  return <div
    ref={element}
    className={`personalizer-element personalizer-element--${kind}${selected ? ' personalizer-element--selected' : ''}`}
    role="group"
    tabIndex={0}
    aria-label={t('personalization.elementInstructions', { label })}
    style={{ left: `${frame.x}%`, top: `${frame.y}%`, width: `${frame.width}%`, height: `${frame.height}%` }}
    onPointerDown={(event) => start(event, 'move')}
    onPointerMove={move}
    onPointerUp={stop}
    onPointerCancel={stop}
    onKeyDown={moveWithKeyboard}
  >
    <div className="personalizer-element-content">{children}</div>
    {(['nw', 'ne', 'sw', 'se'] as const).map((handle) => <button
      key={handle}
      type="button"
      className={`personalizer-resize-handle personalizer-resize-handle--${handle}`}
      aria-label={t('personalization.resize', { label })}
      onPointerDown={(event) => start(event, handle)}
      onPointerMove={move}
      onPointerUp={stop}
      onPointerCancel={stop}
    >{handle === 'nw' ? '↖' : handle === 'ne' ? '↗' : handle === 'sw' ? '↙' : '↘'}</button>)}
  </div>
}

export function ProductPersonalizer({ config, variantId, variantLabel, productMedia, onChange, previewOpen, onPreviewClose, onAddToCart, addToCartDisabled, addToCartLabel, saving = false }: Readonly<{
  config: PersonalizationConfig
  variantId: string
  variantLabel?: string
  productMedia: ProductMediaForPersonalizer[]
  onChange: (value: { customization: CustomerCustomization | null; mediaIds: string[]; ready: boolean; missing: string[]; savePreviews?: () => Promise<NonNullable<CustomerCustomization['previews']>> }) => void
  previewOpen: boolean
  onPreviewClose: () => void
  onAddToCart: () => void
  addToCartDisabled: boolean
  addToCartLabel: string
  saving?: boolean
}>) {
  const { t } = useI18n()
  const fonts = useMemo(() => { const valid = Array.isArray(config.allowed_fonts) ? config.allowed_fonts.filter((value): value is typeof SUPPORTED_FONTS[number] => typeof value === 'string' && SUPPORTED_FONTS.includes(value as typeof SUPPORTED_FONTS[number])) : []; return valid.length ? valid : [...SUPPORTED_FONTS] }, [config.allowed_fonts])
  const colors = useMemo(() => { const valid = Array.isArray(config.allowed_colors) ? config.allowed_colors.filter((value): value is string => typeof value === 'string' && /^#[0-9a-f]{6}$/i.test(value)) : []; return valid.length ? valid : DEFAULT_COLORS }, [config.allowed_colors])
  const colorName = (value: string) => ({
    '#111111': t('personalization.colorBlack'),
    '#ffffff': t('personalization.colorWhite'),
    '#9c5263': t('personalization.colorDustyRose'),
    '#1f4f78': t('personalization.colorBlue'),
    '#b3232f': t('personalization.colorRed'),
  }[value.toLowerCase()] ?? value)
  const views = useMemo(() => configuredViews(
    config,
    variantId,
    t('personalization.front'),
    (number) => t('personalization.viewFallback', { number }),
    (number) => t('personalization.areaFallback', { number }),
    productMedia,
  ), [config, variantId, t, productMedia])
  const newDesign = (mode = views[0].mode): AreaDesign => ({
    text: '',
    font: fonts[0] ?? 'Arial',
    color: colors[0] ?? '#111111',
    size: config.text_min_size,
    textFrame: mode === 'photo_text' ? { x: 10, y: 58, width: 80, height: 40 } : { x: 15, y: 35, width: 70, height: 30 },
    photoFrame: mode === 'photo_text' ? { x: 5, y: 5, width: 90, height: 50 } : { x: 15, y: 15, width: 70, height: 70 },
  })
  const [designs, setDesigns] = useState<Record<string, AreaDesign>>(() => Object.fromEntries(views.flatMap((view) => view.printAreas.map((area) => [designKey(view.id, area.id), newDesign(view.mode)]))))
  const [activeViewId, setActiveViewId] = useState(views[0].id)
  const activeView = views.find(({ id }) => id === activeViewId) ?? views[0]
  const wantsPhoto = activeView.mode === 'photo' || activeView.mode === 'photo_text'
  const wantsText = activeView.mode === 'text' || activeView.mode === 'photo_text'
  const printAreas = activeView.printAreas
  const [activeAreaId, setActiveAreaId] = useState(printAreas[0].id)
  const [selected, setSelected] = useState<'photo' | 'text'>(wantsPhoto ? 'photo' : 'text')
  const [uploadingAreas, setUploadingAreas] = useState<Record<string, boolean>>({})
  const [previewViewId, setPreviewViewId] = useState(views[0].id)
  const objectUrls = useRef(new Set<string>())
  const confirmationCanvases = useRef(new Map<string, HTMLDivElement>())
  const activeDesignKey = designKey(activeView.id, activeAreaId)
  const activeDesign = designs[activeDesignKey] ?? newDesign()
  const activePrintArea = printAreas.find(({ id }) => id === activeAreaId) ?? printAreas[0]
  const activeProductImage = productMedia.find(({ id }) => id === activeView.mediaId)?.url ?? (views.length === 1 ? productMedia[0]?.url : undefined)
  const previewView = views.find(({ id }) => id === previewViewId) ?? views[0]
  const previewProductImage = productMedia.find(({ id }) => id === previewView.mediaId)?.url ?? (views.length === 1 ? productMedia[0]?.url : undefined)
  const previewHasDesign = previewView.printAreas.some((area) => {
    const design = designs[designKey(previewView.id, area.id)]
    return Boolean(design?.photoUrl || design?.text.trim())
  })
  const previewIsPrintCapped = previewView.printAreas.some((area) => {
    const design = designs[designKey(previewView.id, area.id)]
    const frame = area.previewFrame
    return Boolean((design?.photoUrl || design?.text.trim()) && frame && (frame.width < area.width - 0.01 || frame.height < area.height - 0.01))
  })

  function updateDesign(key: string, update: Partial<AreaDesign> | ((current: AreaDesign) => AreaDesign)) {
    setDesigns((current) => {
      const areaDesign = current[key] ?? newDesign()
      return { ...current, [key]: typeof update === 'function' ? update(areaDesign) : { ...areaDesign, ...update } }
    })
  }

  function selectView(view: PrintView) {
    setActiveViewId(view.id)
    setActiveAreaId(view.printAreas[0].id)
    setSelected(view.mode === 'photo' || view.mode === 'photo_text' ? 'photo' : 'text')
  }

  const customization = useMemo<CustomerCustomization>(() => ({
    version: 7,
    areas: views.flatMap((view) => view.printAreas.flatMap((area) => {
      const design = designs[designKey(view.id, area.id)]
      if (!design) return []
      const photo = (view.mode === 'photo' || view.mode === 'photo_text') && design.mediaId ? { media_id: design.mediaId, ...normalizedFrame(design.photoFrame), crop_x: 50, crop_y: 50, scale: 1 } : undefined
      const text = (view.mode === 'text' || view.mode === 'photo_text') && design.text.trim() ? { content: design.text.trim(), font: design.font, color: design.color, size: design.size, ...normalizedFrame(design.textFrame) } : undefined
      const reference = articleReferenceSnapshot(view, area)
      return photo || text ? [{ view_id: view.id, view_label: view.label, area_id: area.id, area_label: area.label, print_width_cm: area.physicalWidthCm, print_height_cm: area.physicalHeightCm, ...(reference ? { article_reference: reference } : {}), ...(photo ? { photo } : {}), ...(text ? { text } : {}) }] : []
    })),
  }), [designs, views])
  const mediaIds = useMemo(() => customization.areas.flatMap((area) => area.photo ? [area.photo.media_id] : []), [customization])
  const missing = useMemo(() => views.flatMap((view) => view.printAreas.flatMap((area) => {
    const design = designs[designKey(view.id, area.id)]
    return [
      (view.mode === 'photo' || view.mode === 'photo_text') && !design?.mediaId ? t('personalization.missingPhoto', { view: view.label, area: area.label }) : '',
      (view.mode === 'text' || view.mode === 'photo_text') && !design?.text.trim() ? t('personalization.missingText', { view: view.label, area: area.label }) : '',
    ].filter(Boolean)
  })), [designs, t, views])
  const ready = missing.length === 0

  useEffect(() => onChange({ customization: customization.areas.length ? customization : null, mediaIds, ready, missing,
    savePreviews: async () => {
      if (!customization.areas.length) return []
      await document.fonts.ready
      const result: NonNullable<CustomerCustomization['previews']> = []
      for (const view of views.filter((view) => customization.areas.some((area) => area.view_id === view.id))) {
        const node = confirmationCanvases.current.get(view.id)
        if (!node) throw new Error('Personalized preview unavailable')
        await Promise.all(Array.from(node.querySelectorAll('img')).map((image) => image.decode()))
        const blob = await toBlob(node, { pixelRatio: 1, backgroundColor: '#f5f1ee', preferredFontFormat: 'woff2' })
        if (!blob || blob.size === 0) throw new Error('Could not save personalized preview')
        const filename = 'personalized-preview.png'
        const upload = await cartApi.initiatePersonalizationUpload({ filename, content_type: 'image/png', byte_size: blob.size })
        await cartApi.uploadMediaObject(upload.upload_url, new File([blob], filename, { type: 'image/png' }), 'image/png')
        await cartApi.completePersonalizationUpload(upload.id)
        result.push({ view_id: view.id, view_label: view.label, media_id: upload.id })
      }
      return result
    },
  }), [customization, mediaIds, missing, onChange, ready])
  useEffect(() => () => { objectUrls.current.forEach((url) => URL.revokeObjectURL(url)); objectUrls.current.clear() }, [])
  useEffect(() => {
    setDesigns((current) => Object.fromEntries(views.flatMap((view) => view.printAreas.map((area) => { const key = designKey(view.id, area.id); return [key, current[key] ?? newDesign(view.mode)] }))))
    if (!views.some(({ id }) => id === activeViewId)) setActiveViewId(views[0].id)
    if (!printAreas.some(({ id }) => id === activeAreaId)) setActiveAreaId(printAreas[0].id)
  }, [views, activeViewId, printAreas, activeAreaId])
  useEffect(() => {
    if (!previewOpen) return
    setPreviewViewId(activeView.id)
    const previousOverflow = document.body.style.overflow
    const closeOnEscape = (event: globalThis.KeyboardEvent) => { if (event.key === 'Escape') onPreviewClose() }
    document.body.style.overflow = 'hidden'
    window.addEventListener('keydown', closeOnEscape)
    return () => {
      document.body.style.overflow = previousOverflow
      window.removeEventListener('keydown', closeOnEscape)
    }
  }, [previewOpen, activeView.id, onPreviewClose])

  async function upload(key: string, file?: File) {
    if (!file) return
    const currentUrl = designs[key]?.photoUrl
    if (currentUrl) { URL.revokeObjectURL(currentUrl); objectUrls.current.delete(currentUrl) }
    const photoUrl = URL.createObjectURL(file)
    objectUrls.current.add(photoUrl)
    updateDesign(key, { photoUrl, mediaId: undefined })
    setUploadingAreas((current) => ({ ...current, [key]: true }))
    setSelected('photo')
    try {
      const uploadRequest = await cartApi.initiatePersonalizationUpload({ filename: file.name, content_type: file.type, byte_size: file.size })
      await cartApi.uploadMediaObject(uploadRequest.upload_url, file, file.type)
      const complete = await cartApi.completePersonalizationUpload(uploadRequest.id)
      updateDesign(key, { mediaId: complete.id })
    } finally {
      setUploadingAreas((current) => ({ ...current, [key]: false }))
    }
  }

  function renderFinalPreview(view: PrintView, imageUrl: string, capture = false) {
    return <div className="personalization-final-preview-canvas" ref={capture ? (node) => { if (node) confirmationCanvases.current.set(view.id, node); else confirmationCanvases.current.delete(view.id) } : undefined}>
            <img src={imageUrl} alt={t('personalization.customizedResultAlt', { view: view.label })} />
            {view.printAreas.map((area) => {
              const design = designs[designKey(view.id, area.id)] ?? newDesign()
              if (!design.photoUrl && !design.text.trim()) return null
              const frame = area.previewFrame ?? area
              return <div key={area.id} className="personalization-final-area" style={{ left: `${frame.x}%`, top: `${frame.y}%`, width: `${frame.width}%`, height: `${frame.height}%` }}>
                {design.photoUrl && <div className="personalization-final-element" style={{ left: `${design.photoFrame.x}%`, top: `${design.photoFrame.y}%`, width: `${design.photoFrame.width}%`, height: `${design.photoFrame.height}%` }}><img className="personalization-final-photo" src={design.photoUrl} alt={t('personalization.customPhotoAlt')} /></div>}
                {design.text.trim() && <div className="personalization-final-element personalization-final-element--text" style={{ left: `${design.textFrame.x}%`, top: `${design.textFrame.y}%`, width: `${design.textFrame.width}%`, height: `${design.textFrame.height}%` }}><span style={{ color: design.color, fontFamily: design.font, fontSize: `${design.size}px` }}>{design.text}</span></div>}
              </div>
            })}
          </div>
  }

  return <section className="personalizer" aria-labelledby="personalizer-title" inert={saving} aria-busy={saving}>
    <h2 className="sr-only" id="personalizer-title">{t('personalization.options')}</h2>
    <div className="personalizer-layout">
      <div className="personalizer-tools">
        <section className="personalizer-location-card">
          <header className="personalizer-sidebar-heading"><span className="personalizer-step">1</span><div><strong>{t('personalization.chooseWhere')}</strong><small>{activeView.label} · {activePrintArea.label}</small></div></header>
          <div className="personalizer-sidebar-label"><span>{t('personalization.productSide')}</span><small>{views.length} {t(views.length === 1 ? 'personalization.availableSingular' : 'personalization.availablePlural')}</small></div>
          {views.length > 1 ? <div className="personalizer-view-switcher" role="tablist" aria-label={t('personalization.chooseProductSide')}>{views.map((view, index) => <button key={view.id} type="button" role="tab" aria-selected={view.id === activeView.id} className={view.id === activeView.id ? 'selected' : ''} onClick={() => selectView(view)}><b>{index + 1}</b><span>{view.label}</span></button>)}</div> : <div className="personalizer-single-choice"><b>1</b><span>{activeView.label}</span></div>}
          <div className="personalizer-area-switcher" role="group" aria-label={t('personalization.choosePrintArea')}><div className="personalizer-sidebar-label"><span>{t('personalization.printArea')}</span></div><div>{printAreas.map((area, index) => <button key={area.id} type="button" className={activeAreaId === area.id ? 'selected' : ''} aria-pressed={activeAreaId === area.id} onClick={() => setActiveAreaId(area.id)}><b>{index + 1}</b>{area.label}</button>)}</div></div>
        </section>
        {wantsPhoto && <div className={`personalizer-tool${selected === 'photo' ? ' personalizer-tool--selected' : ''}`} onClick={() => setSelected('photo')}>
          <header className="personalizer-sidebar-heading"><span className="personalizer-step">2</span><div><strong><ImagePlus /> {t('personalization.photo')} · {activePrintArea.label}</strong><small>{t('personalization.optional')}</small></div></header>
          <label className="personalizer-upload">{t(uploadingAreas[activeDesignKey] ? 'personalization.preparingPhoto' : activeDesign.photoUrl ? 'personalization.replacePhoto' : 'personalization.uploadPhoto')}<input type="file" accept="image/jpeg,image/png,image/webp" disabled={uploadingAreas[activeDesignKey]} onChange={(event) => void upload(activeDesignKey, event.currentTarget.files?.[0])} /></label>
          <span className="personalizer-drag-hint"><Move /> {t('personalization.photoDragHint')}</span>
        </div>}
        {wantsText && <div className={`personalizer-tool${selected === 'text' ? ' personalizer-tool--selected' : ''}`} onClick={() => setSelected('text')}>
          <header className="personalizer-sidebar-heading"><span className="personalizer-step">{wantsPhoto ? 3 : 2}</span><div><strong><Type /> {t('personalization.text')} · {activePrintArea.label}</strong><small>{t('personalization.optional')}</small></div></header>
          <label>{t('personalization.yourText')}<textarea rows={2} maxLength={config.text_max_characters} value={activeDesign.text} onChange={(event) => { updateDesign(activeDesignKey, { text: event.target.value }); setSelected('text') }} placeholder={t('personalization.textPlaceholder')} /></label>
          <small>{activeDesign.text.length} / {config.text_max_characters}</small>
          <span className="personalizer-drag-hint"><Move /> {t('personalization.textDragHint')}</span>
          <span className="personalizer-control-label">{t('personalization.font')}</span>
          <div className="font-choice-grid">{fonts.map((value) => <button key={value} type="button" className={activeDesign.font === value ? 'selected' : ''} aria-pressed={activeDesign.font === value} onClick={() => updateDesign(activeDesignKey, { font: value })}><b style={{ fontFamily: value }}>Ag</b><small>{value}</small></button>)}</div>
          <label>{t('personalization.color')}<select value={activeDesign.color} onChange={(event) => updateDesign(activeDesignKey, { color: event.target.value })}>{colors.map((value) => <option key={value} value={value}>{colorName(value)} · {value}</option>)}</select></label>
          <span className="selected-color"><i style={{ background: activeDesign.color }} />{colorName(activeDesign.color)}</span>
          <label><span className="personalizer-range-heading">{t('personalization.fontSize')} <output>{activeDesign.size}</output></span><input aria-label={t('personalization.fontSize')} type="range" min={config.text_min_size} max={config.text_max_size} value={activeDesign.size} onChange={(event) => updateDesign(activeDesignKey, { size: Number(event.target.value) })} /></label>
        </div>}
      </div>
      <div className="personalizer-stage">
        {activeProductImage ? <div className="personalizer-canvas"><img className="personalizer-product" src={activeProductImage} alt={t('personalization.productPreviewAlt', { view: activeView.label })} />
          {printAreas.map((area) => {
            const key = designKey(activeView.id, area.id)
            const design = designs[key] ?? newDesign()
            const active = activeAreaId === area.id
            const frame = area.previewFrame ?? area
            return <div key={area.id} className={`personalizer-print-area${active ? ' personalizer-print-area--active' : ''}`} aria-label={area.label} style={{ left: `${frame.x}%`, top: `${frame.y}%`, width: `${frame.width}%`, height: `${frame.height}%` }} onClick={() => setActiveAreaId(area.id)}>
              <span className="personalizer-print-area-label">{area.label}</span>
              {wantsPhoto && (active || design.mediaId) && <DesignElement frame={design.photoFrame} kind="photo" label={t('personalization.photoElement', { view: activeView.label, area: area.label })} selected={active && selected === 'photo'} onSelect={() => { setActiveAreaId(area.id); setSelected('photo') }} onChange={(photoFrame) => updateDesign(key, { photoFrame })}>{design.photoUrl ? <img className="personalizer-photo" src={design.photoUrl} alt={t('personalization.uploadedPhotoAlt')} draggable={false} /> : <span className="personalizer-placeholder"><ImagePlus /> {t('personalization.photo')}</span>}</DesignElement>}
              {wantsText && (active || design.text.trim()) && <DesignElement frame={design.textFrame} kind="text" label={t('personalization.textElement', { view: activeView.label, area: area.label })} selected={active && selected === 'text'} onSelect={() => { setActiveAreaId(area.id); setSelected('text') }} onChange={(textFrame) => updateDesign(key, { textFrame })}>{design.text.trim() ? <span className="personalizer-text" style={{ color: design.color, fontFamily: design.font, fontSize: `${design.size}px` }}>{design.text}</span> : <span className="personalizer-placeholder"><Type /> {t('personalization.text')}</span>}</DesignElement>}
            </div>
          })}
        </div> : <div className="personalizer-product-empty">{t('personalization.emptyView')}</div>}
      </div>
    </div>
    <div aria-hidden="true" style={{ position: 'fixed', left: '-10000px', top: 0, width: 'min(680px, calc(100vw - 32px))', pointerEvents: 'none' }}>
      {views.filter((view) => customization.areas.some((area) => area.view_id === view.id)).map((view) => {
        const imageUrl = productMedia.find(({ id }) => id === view.mediaId)?.url ?? (views.length === 1 ? productMedia[0]?.url : undefined)
        return imageUrl ? <div key={view.id}>{renderFinalPreview(view, imageUrl, true)}</div> : null
      })}
    </div>
    {previewOpen && <div className="personalization-final-preview-backdrop" role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget) onPreviewClose() }}>
      <section className="personalization-final-preview" role="dialog" aria-modal="true" aria-labelledby="final-preview-title" aria-describedby="final-preview-description">
        <header>
          <div><span>{t('personalization.preview')}</span><h2 id="final-preview-title">{t('personalization.finalResult')}</h2><p id="final-preview-description">{t('personalization.previewDescription')}</p>{variantLabel && <p className="personalization-final-preview-option" aria-live="polite">{t('personalization.previewForOption', { option: variantLabel })}</p>}</div>
          {views.length > 1 && <div className="personalization-final-preview-tabs" role="tablist" aria-label={t('personalization.choosePreviewSide')}>{views.map((view, index) => <button key={view.id} type="button" role="tab" aria-selected={view.id === previewView.id} className={view.id === previewView.id ? 'selected' : ''} onClick={() => setPreviewViewId(view.id)}><b>{index + 1}</b>{view.label}</button>)}</div>}
        </header>
        <div className="personalization-final-preview-stage">
          {previewProductImage ? renderFinalPreview(previewView, previewProductImage) : <p>{t('personalization.emptyView')}</p>}
          {previewProductImage && previewHasDesign && <p className="personalization-final-preview-note" role="status">{t(previewIsPrintCapped ? 'personalization.previewPrintCapped' : 'personalization.previewScaleNote')}</p>}
        </div>
        <footer><span>{t('personalization.previewFooter')}</span><div className="personalization-final-preview-actions"><button className="button button--secondary" type="button" autoFocus onClick={onPreviewClose}>{t('personalization.continuePersonalizing')}</button><button className="button button--primary" type="button" disabled={addToCartDisabled} onClick={onAddToCart}><ShoppingBag />{addToCartLabel}</button></div></footer>
      </section>
    </div>}
  </section>
}
