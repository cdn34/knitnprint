import type { PersonalizationConfig } from '@knitprint/api-client'
import { ImagePlus, Move, Ruler, ShoppingBag, Type } from 'lucide-react'
import { type KeyboardEvent, type PointerEvent as ReactPointerEvent, type ReactNode, useEffect, useMemo, useRef, useState } from 'react'
import { cartApi } from '../cart-api'

const SUPPORTED_FONTS = ['Roboto', 'Montserrat', 'Playfair Display', 'Dancing Script', 'Pacifico'] as const
const DEFAULT_COLORS = ['#111111', '#ffffff', '#9c5263', '#1f4f78', '#b3232f']
const safeBasisPoints = (value: unknown, fallback: number) => typeof value === 'number' && Number.isFinite(value) ? value : fallback

type ElementFrame = { x: number; y: number; width: number; height: number }
type Interaction = { pointerX: number; pointerY: number; frame: ElementFrame; handle: 'move' | 'nw' | 'ne' | 'sw' | 'se' }
type PrintArea = ElementFrame & { id: string; label: string; physicalWidthCm: number; physicalHeightCm: number }
type ArticleReference = ElementFrame & { physicalWidthCm: number; physicalHeightCm: number }
type PrintView = { id: string; label: string; mediaId?: string; articleReference?: ArticleReference; printAreas: PrintArea[] }
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

function configuredPrintAreas(raw: unknown, config: PersonalizationConfig): PrintArea[] {
  if (Array.isArray(raw)) {
    const areas = raw.flatMap((item, index) => {
      if (!item || typeof item !== 'object') return []
      const area = item as Record<string, unknown>
      const coordinates = ['x', 'y', 'width', 'height'].map((key) => area[key])
      if (coordinates.some((value) => typeof value !== 'number' || !Number.isFinite(value))) return []
      return [{
        id: typeof area.id === 'string' && area.id ? area.id : `area-${index + 1}`,
        label: typeof area.label === 'string' && area.label ? area.label : `Área ${index + 1}`,
        x: Number(area.x) / 100,
        y: Number(area.y) / 100,
        width: Number(area.width) / 100,
        height: Number(area.height) / 100,
        physicalWidthCm: typeof area.physical_width_cm === 'number' && Number.isFinite(area.physical_width_cm) ? area.physical_width_cm : 20,
        physicalHeightCm: typeof area.physical_height_cm === 'number' && Number.isFinite(area.physical_height_cm) ? area.physical_height_cm : 20,
      }]
    })
    if (areas.length) return areas
  }
  return [{ id: 'area-1', label: 'Área 1', x: safeBasisPoints(config.area_x, 2500) / 100, y: safeBasisPoints(config.area_y, 2500) / 100, width: safeBasisPoints(config.area_width, 5000) / 100, height: safeBasisPoints(config.area_height, 5000) / 100, physicalWidthCm: 20, physicalHeightCm: 20 }]
}

function configuredViews(config: PersonalizationConfig): PrintView[] {
  if (Array.isArray(config.views)) {
    const views = config.views.flatMap((item, index) => {
      if (!item || typeof item !== 'object') return []
      const view = item as Record<string, unknown>
      const printAreas = configuredPrintAreas(view.print_areas, config)
      const rawReference = view.article_reference && typeof view.article_reference === 'object' ? view.article_reference as Record<string, unknown> : undefined
      const referenceCoordinates = rawReference ? ['x', 'y', 'width', 'height'].map((key) => rawReference[key]) : []
      const articleReference = rawReference?.configured === true
        && referenceCoordinates.every((value) => typeof value === 'number' && Number.isFinite(value))
        && typeof rawReference.physical_width_cm === 'number' && Number.isFinite(rawReference.physical_width_cm)
        && typeof rawReference.physical_height_cm === 'number' && Number.isFinite(rawReference.physical_height_cm)
        ? { x: Number(rawReference.x) / 100, y: Number(rawReference.y) / 100, width: Number(rawReference.width) / 100, height: Number(rawReference.height) / 100, physicalWidthCm: rawReference.physical_width_cm, physicalHeightCm: rawReference.physical_height_cm }
        : undefined
      return [{
        id: typeof view.id === 'string' && view.id ? view.id : `view-${index + 1}`,
        label: typeof view.label === 'string' && view.label ? view.label : index === 0 ? 'Frente' : `Vista ${index + 1}`,
        mediaId: typeof view.media_id === 'string' ? view.media_id : undefined,
        articleReference,
        printAreas,
      }]
    })
    if (views.length) return views
  }
  return [{ id: 'view-front', label: 'Frente', mediaId: config.preview_media_id ?? undefined, printAreas: configuredPrintAreas(config.print_areas, config) }]
}

const designKey = (viewId: string, areaId: string) => `${viewId}:${areaId}`
const formatCm = (value: number) => new Intl.NumberFormat('pt-PT', { maximumFractionDigits: 1 }).format(Math.round(value * 10) / 10)
const physicalFrameSize = (area: PrintArea, frame: ElementFrame) => `${formatCm(area.physicalWidthCm * frame.width / 100)} × ${formatCm(area.physicalHeightCm * frame.height / 100)} cm`

type ElementPlacement = { left: number; right: number; top: number; bottom: number; absoluteFrame: ElementFrame }

function elementPlacement(view: PrintView, area: PrintArea, frame: ElementFrame): ElementPlacement | undefined {
  const reference = view.articleReference
  if (!reference) return undefined
  const absoluteFrame = {
    x: area.x + area.width * frame.x / 100,
    y: area.y + area.height * frame.y / 100,
    width: area.width * frame.width / 100,
    height: area.height * frame.height / 100,
  }
  return {
    left: reference.physicalWidthCm * (absoluteFrame.x - reference.x) / reference.width,
    right: reference.physicalWidthCm * (reference.x + reference.width - absoluteFrame.x - absoluteFrame.width) / reference.width,
    top: reference.physicalHeightCm * (absoluteFrame.y - reference.y) / reference.height,
    bottom: reference.physicalHeightCm * (reference.y + reference.height - absoluteFrame.y - absoluteFrame.height) / reference.height,
    absoluteFrame,
  }
}

function articleReferenceSnapshot(view: PrintView, area: PrintArea) {
  const reference = view.articleReference
  if (!reference) return undefined
  return {
    article_width_cm: reference.physicalWidthCm,
    article_height_cm: reference.physicalHeightCm,
    print_left_cm: Math.round(reference.physicalWidthCm * (area.x - reference.x) / reference.width * 100) / 100,
    print_top_cm: Math.round(reference.physicalHeightCm * (area.y - reference.y) / reference.height * 100) / 100,
  }
}

export type CustomerCustomization = {
  version: 7
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

function DesignElement({ frame, kind, label, measurement, selected, onSelect, onChange, children }: Readonly<{
  frame: ElementFrame
  kind: 'photo' | 'text'
  label: string
  measurement: string
  selected: boolean
  onSelect: () => void
  onChange: (frame: ElementFrame) => void
  children: ReactNode
}>) {
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
    aria-label={`${label}, ${measurement}. Arrasta para mover e usa as setas nos cantos para redimensionar.`}
    style={{ left: `${frame.x}%`, top: `${frame.y}%`, width: `${frame.width}%`, height: `${frame.height}%` }}
    onPointerDown={(event) => start(event, 'move')}
    onPointerMove={move}
    onPointerUp={stop}
    onPointerCancel={stop}
    onKeyDown={moveWithKeyboard}
  >
    <div className="personalizer-element-content">{children}</div>
    <span className="personalizer-element-measure">{measurement}</span>
    {(['nw', 'ne', 'sw', 'se'] as const).map((handle) => <button
      key={handle}
      type="button"
      className={`personalizer-resize-handle personalizer-resize-handle--${handle}`}
      aria-label={`Redimensionar ${label}`}
      onPointerDown={(event) => start(event, handle)}
      onPointerMove={move}
      onPointerUp={stop}
      onPointerCancel={stop}
    >{handle === 'nw' ? '↖' : handle === 'ne' ? '↗' : handle === 'sw' ? '↙' : '↘'}</button>)}
  </div>
}

function PlacementSummary({ placement }: Readonly<{ placement?: ElementPlacement }>) {
  if (!placement) return <p className="personalizer-placement-unavailable"><Ruler />As distâncias ficam disponíveis depois de o artigo ser calibrado no administrador.</p>
  return <div className="personalizer-placement-summary" aria-label="Distâncias da personalização aos limites do artigo">
    <span><small>Topo</small><b>{formatCm(placement.top)} cm</b></span>
    <span><small>Esquerda</small><b>{formatCm(placement.left)} cm</b></span>
    <span><small>Direita</small><b>{formatCm(placement.right)} cm</b></span>
    <span><small>Fundo</small><b>{formatCm(placement.bottom)} cm</b></span>
  </div>
}

function MeasurementGuides({ reference, placement }: Readonly<{ reference?: ArticleReference; placement?: ElementPlacement }>) {
  if (!reference || !placement) return null
  const frame = placement.absoluteFrame
  const right = frame.x + frame.width
  const bottom = frame.y + frame.height
  const referenceRight = reference.x + reference.width
  const referenceBottom = reference.y + reference.height
  return <div className="personalizer-measurement-layer" aria-hidden="true">
    <div className="personalizer-article-reference" style={{ left: `${reference.x}%`, top: `${reference.y}%`, width: `${reference.width}%`, height: `${reference.height}%` }}><span>Limites do artigo</span></div>
    <span className="personalizer-distance-guide personalizer-distance-guide--horizontal" style={{ left: `${reference.x}%`, top: `${frame.y + frame.height / 2}%`, width: `${Math.max(0, frame.x - reference.x)}%` }}><b>{formatCm(placement.left)} cm</b></span>
    <span className="personalizer-distance-guide personalizer-distance-guide--horizontal" style={{ left: `${right}%`, top: `${frame.y + frame.height / 2}%`, width: `${Math.max(0, referenceRight - right)}%` }}><b>{formatCm(placement.right)} cm</b></span>
    <span className="personalizer-distance-guide personalizer-distance-guide--vertical" style={{ left: `${frame.x + frame.width / 2}%`, top: `${reference.y}%`, height: `${Math.max(0, frame.y - reference.y)}%` }}><b>{formatCm(placement.top)} cm</b></span>
    <span className="personalizer-distance-guide personalizer-distance-guide--vertical" style={{ left: `${frame.x + frame.width / 2}%`, top: `${bottom}%`, height: `${Math.max(0, referenceBottom - bottom)}%` }}><b>{formatCm(placement.bottom)} cm</b></span>
  </div>
}

export function ProductPersonalizer({ config, productMedia, onChange, previewOpen, onPreviewClose, onAddToCart, addToCartDisabled, addToCartLabel }: Readonly<{
  config: PersonalizationConfig
  productMedia: ProductMediaForPersonalizer[]
  onChange: (value: { customization: CustomerCustomization | null; mediaIds: string[]; ready: boolean; missing: string[] }) => void
  previewOpen: boolean
  onPreviewClose: () => void
  onAddToCart: () => void
  addToCartDisabled: boolean
  addToCartLabel: string
}>) {
  const fonts = useMemo(() => { const valid = Array.isArray(config.allowed_fonts) ? config.allowed_fonts.filter((value): value is typeof SUPPORTED_FONTS[number] => typeof value === 'string' && SUPPORTED_FONTS.includes(value as typeof SUPPORTED_FONTS[number])) : []; return valid.length ? valid : [...SUPPORTED_FONTS] }, [config.allowed_fonts])
  const colors = useMemo(() => { const valid = Array.isArray(config.allowed_colors) ? config.allowed_colors.filter((value): value is string => typeof value === 'string' && /^#[0-9a-f]{6}$/i.test(value)) : []; return valid.length ? valid : DEFAULT_COLORS }, [config.allowed_colors])
  const colorName = (value: string) => ({ '#111111': 'Preto', '#ffffff': 'Branco', '#9c5263': 'Rosa antigo', '#1f4f78': 'Azul', '#b3232f': 'Vermelho' }[value.toLowerCase()] ?? value)
  const wantsPhoto = config.mode === 'photo' || config.mode === 'photo_text'
  const wantsText = config.mode === 'text' || config.mode === 'photo_text'
  const combined = wantsPhoto && wantsText
  const views = useMemo(() => configuredViews(config), [config.views, config.preview_media_id, config.print_areas, config.area_x, config.area_y, config.area_width, config.area_height])
  const newDesign = (): AreaDesign => ({
    text: '',
    font: fonts[0] ?? 'Arial',
    color: colors[0] ?? '#111111',
    size: config.text_min_size,
    textFrame: combined ? { x: 10, y: 58, width: 80, height: 40 } : { x: 15, y: 35, width: 70, height: 30 },
    photoFrame: combined ? { x: 5, y: 5, width: 90, height: 50 } : { x: 15, y: 15, width: 70, height: 70 },
  })
  const [designs, setDesigns] = useState<Record<string, AreaDesign>>(() => Object.fromEntries(views.flatMap((view) => view.printAreas.map((area) => [designKey(view.id, area.id), newDesign()]))))
  const [activeViewId, setActiveViewId] = useState(views[0].id)
  const activeView = views.find(({ id }) => id === activeViewId) ?? views[0]
  const printAreas = activeView.printAreas
  const [activeAreaId, setActiveAreaId] = useState(printAreas[0].id)
  const [selected, setSelected] = useState<'photo' | 'text'>(wantsPhoto ? 'photo' : 'text')
  const [uploadingAreas, setUploadingAreas] = useState<Record<string, boolean>>({})
  const [previewViewId, setPreviewViewId] = useState(views[0].id)
  const objectUrls = useRef(new Set<string>())
  const activeDesignKey = designKey(activeView.id, activeAreaId)
  const activeDesign = designs[activeDesignKey] ?? newDesign()
  const activePrintArea = printAreas.find(({ id }) => id === activeAreaId) ?? printAreas[0]
  const activePhotoMeasurement = physicalFrameSize(activePrintArea, activeDesign.photoFrame)
  const activeTextMeasurement = physicalFrameSize(activePrintArea, activeDesign.textFrame)
  const activePhotoPlacement = elementPlacement(activeView, activePrintArea, activeDesign.photoFrame)
  const activeTextPlacement = elementPlacement(activeView, activePrintArea, activeDesign.textFrame)
  const selectedPlacement = selected === 'photo' ? activePhotoPlacement : activeTextPlacement
  const activeProductImage = productMedia.find(({ id }) => id === activeView.mediaId)?.url ?? (views.length === 1 ? productMedia[0]?.url : undefined)
  const previewView = views.find(({ id }) => id === previewViewId) ?? views[0]
  const previewProductImage = productMedia.find(({ id }) => id === previewView.mediaId)?.url ?? (views.length === 1 ? productMedia[0]?.url : undefined)

  function updateDesign(key: string, update: Partial<AreaDesign> | ((current: AreaDesign) => AreaDesign)) {
    setDesigns((current) => {
      const areaDesign = current[key] ?? newDesign()
      return { ...current, [key]: typeof update === 'function' ? update(areaDesign) : { ...areaDesign, ...update } }
    })
  }

  function selectView(view: PrintView) {
    setActiveViewId(view.id)
    setActiveAreaId(view.printAreas[0].id)
  }

  const customization = useMemo<CustomerCustomization>(() => ({
    version: 7,
    areas: views.flatMap((view) => view.printAreas.flatMap((area) => {
      const design = designs[designKey(view.id, area.id)]
      if (!design) return []
      const photo = wantsPhoto && design.mediaId ? { media_id: design.mediaId, ...normalizedFrame(design.photoFrame), crop_x: 50, crop_y: 50, scale: 1 } : undefined
      const text = wantsText && design.text.trim() ? { content: design.text.trim(), font: design.font, color: design.color, size: design.size, ...normalizedFrame(design.textFrame) } : undefined
      const reference = articleReferenceSnapshot(view, area)
      return photo || text ? [{ view_id: view.id, view_label: view.label, area_id: area.id, area_label: area.label, print_width_cm: area.physicalWidthCm, print_height_cm: area.physicalHeightCm, ...(reference ? { article_reference: reference } : {}), ...(photo ? { photo } : {}), ...(text ? { text } : {}) }] : []
    })),
  }), [designs, views, wantsPhoto, wantsText])
  const mediaIds = useMemo(() => customization.areas.flatMap((area) => area.photo ? [area.photo.media_id] : []), [customization])
  const missing = useMemo(() => views.flatMap((view) => view.printAreas.flatMap((area) => {
    const design = designs[designKey(view.id, area.id)]
    return [wantsPhoto && !design?.mediaId ? `fotografia em ${view.label} · ${area.label}` : '', wantsText && !design?.text.trim() ? `texto em ${view.label} · ${area.label}` : ''].filter(Boolean)
  })), [designs, views, wantsPhoto, wantsText])
  const ready = missing.length === 0

  useEffect(() => onChange({ customization: customization.areas.length ? customization : null, mediaIds, ready, missing }), [customization, mediaIds, missing, onChange, ready])
  useEffect(() => () => { objectUrls.current.forEach((url) => URL.revokeObjectURL(url)); objectUrls.current.clear() }, [])
  useEffect(() => {
    setDesigns((current) => Object.fromEntries(views.flatMap((view) => view.printAreas.map((area) => { const key = designKey(view.id, area.id); return [key, current[key] ?? newDesign()] }))))
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

  return <section className="personalizer" aria-labelledby="personalizer-title">
    <h2 className="sr-only" id="personalizer-title">Opções de personalização</h2>
    <div className="personalizer-layout">
      <div className="personalizer-tools">
        <section className="personalizer-location-card">
          <header className="personalizer-sidebar-heading"><span className="personalizer-step">1</span><div><strong>Escolhe onde</strong><small>{activeView.label} · {activePrintArea.label}</small></div></header>
          <div className="personalizer-sidebar-label"><span>Lado do produto</span><small>{views.length} {views.length === 1 ? 'disponível' : 'disponíveis'}</small></div>
          {views.length > 1 ? <div className="personalizer-view-switcher" role="tablist" aria-label="Escolher lado do produto">{views.map((view, index) => <button key={view.id} type="button" role="tab" aria-selected={view.id === activeView.id} className={view.id === activeView.id ? 'selected' : ''} onClick={() => selectView(view)}><b>{index + 1}</b><span>{view.label}</span></button>)}</div> : <div className="personalizer-single-choice"><b>1</b><span>{activeView.label}</span></div>}
          <div className="personalizer-area-switcher" role="group" aria-label="Escolher área de impressão"><div className="personalizer-sidebar-label"><span>Área de impressão</span><small>Máx. {formatCm(activePrintArea.physicalWidthCm)} × {formatCm(activePrintArea.physicalHeightCm)} cm</small></div><div>{printAreas.map((area, index) => <button key={area.id} type="button" className={activeAreaId === area.id ? 'selected' : ''} aria-pressed={activeAreaId === area.id} onClick={() => setActiveAreaId(area.id)}><b>{index + 1}</b>{area.label}</button>)}</div></div>
        </section>
        {wantsPhoto && <div className={`personalizer-tool${selected === 'photo' ? ' personalizer-tool--selected' : ''}`} onClick={() => setSelected('photo')}>
          <header className="personalizer-sidebar-heading"><span className="personalizer-step">2</span><div><strong><ImagePlus /> Fotografia · {activePrintArea.label}</strong><small>Opcional</small></div></header>
          <span className="personalizer-measure"><small>Tamanho final</small><b>{activePhotoMeasurement}</b></span>
          <PlacementSummary placement={activePhotoPlacement} />
          <label className="personalizer-upload">{uploadingAreas[activeDesignKey] ? 'A preparar fotografia…' : activeDesign.photoUrl ? 'Trocar fotografia' : 'Carregar fotografia'}<input type="file" accept="image/jpeg,image/png,image/webp" disabled={uploadingAreas[activeDesignKey]} onChange={(event) => void upload(activeDesignKey, event.currentTarget.files?.[0])} /></label>
          <span className="personalizer-drag-hint"><Move /> Arrasta e dimensiona a fotografia dentro desta área. A imagem não tem zoom.</span>
        </div>}
        {wantsText && <div className={`personalizer-tool${selected === 'text' ? ' personalizer-tool--selected' : ''}`} onClick={() => setSelected('text')}>
          <header className="personalizer-sidebar-heading"><span className="personalizer-step">{wantsPhoto ? 3 : 2}</span><div><strong><Type /> Texto · {activePrintArea.label}</strong><small>Opcional</small></div></header>
          <span className="personalizer-measure"><small>Caixa de texto</small><b>{activeTextMeasurement}</b></span>
          <PlacementSummary placement={activeTextPlacement} />
          <label>O teu texto<textarea rows={2} maxLength={config.text_max_characters} value={activeDesign.text} onChange={(event) => { updateDesign(activeDesignKey, { text: event.target.value }); setSelected('text') }} placeholder="Escreve aqui" /></label>
          <small>{activeDesign.text.length} / {config.text_max_characters}</small>
          <span className="personalizer-drag-hint"><Move /> Arrasta e dimensiona o texto dentro desta área.</span>
          <span className="personalizer-control-label">Tipo de letra</span>
          <div className="font-choice-grid">{fonts.map((value) => <button key={value} type="button" className={activeDesign.font === value ? 'selected' : ''} aria-pressed={activeDesign.font === value} onClick={() => updateDesign(activeDesignKey, { font: value })}><b style={{ fontFamily: value }}>Ag</b><small>{value}</small></button>)}</div>
          <label>Cor<select value={activeDesign.color} onChange={(event) => updateDesign(activeDesignKey, { color: event.target.value })}>{colors.map((value) => <option key={value} value={value}>{colorName(value)} · {value}</option>)}</select></label>
          <span className="selected-color"><i style={{ background: activeDesign.color }} />{colorName(activeDesign.color)}</span>
          <label><span className="personalizer-range-heading">Tamanho da letra <output>{activeDesign.size}</output></span><input aria-label="Tamanho da letra" type="range" min={config.text_min_size} max={config.text_max_size} value={activeDesign.size} onChange={(event) => updateDesign(activeDesignKey, { size: Number(event.target.value) })} /></label>
        </div>}
      </div>
      <div className="personalizer-stage">
        {activeProductImage ? <div className="personalizer-canvas"><img className="personalizer-product" src={activeProductImage} alt={`Pré-visualização do produto · ${activeView.label}`} />
          <MeasurementGuides reference={activeView.articleReference} placement={selectedPlacement} />
          {printAreas.map((area) => {
            const key = designKey(activeView.id, area.id)
            const design = designs[key] ?? newDesign()
            const active = activeAreaId === area.id
            return <div key={area.id} className={`personalizer-print-area${active ? ' personalizer-print-area--active' : ''}`} aria-label={`Área de impressão: ${area.label}, ${formatCm(area.physicalWidthCm)} × ${formatCm(area.physicalHeightCm)} cm`} style={{ left: `${area.x}%`, top: `${area.y}%`, width: `${area.width}%`, height: `${area.height}%` }} onClick={() => setActiveAreaId(area.id)}>
              <span className="personalizer-print-area-label">{area.label} · {formatCm(area.physicalWidthCm)} × {formatCm(area.physicalHeightCm)} cm</span>
              {wantsPhoto && (active || design.mediaId) && <DesignElement frame={design.photoFrame} kind="photo" label={`Fotografia em ${activeView.label} · ${area.label}`} measurement={physicalFrameSize(area, design.photoFrame)} selected={active && selected === 'photo'} onSelect={() => { setActiveAreaId(area.id); setSelected('photo') }} onChange={(photoFrame) => updateDesign(key, { photoFrame })}>{design.photoUrl ? <img className="personalizer-photo" src={design.photoUrl} alt="Fotografia carregada" draggable={false} /> : <span className="personalizer-placeholder"><ImagePlus /> Fotografia</span>}</DesignElement>}
              {wantsText && (active || design.text.trim()) && <DesignElement frame={design.textFrame} kind="text" label={`Texto em ${activeView.label} · ${area.label}`} measurement={physicalFrameSize(area, design.textFrame)} selected={active && selected === 'text'} onSelect={() => { setActiveAreaId(area.id); setSelected('text') }} onChange={(textFrame) => updateDesign(key, { textFrame })}>{design.text.trim() ? <span className="personalizer-text" style={{ color: design.color, fontFamily: design.font, fontSize: `${design.size}px` }}>{design.text}</span> : <span className="personalizer-placeholder"><Type /> Texto</span>}</DesignElement>}
            </div>
          })}
        </div> : <div className="personalizer-product-empty">Esta vista ainda não tem uma fotografia associada.</div>}
      </div>
    </div>
    {previewOpen && <div className="personalization-final-preview-backdrop" role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget) onPreviewClose() }}>
      <section className="personalization-final-preview" role="dialog" aria-modal="true" aria-labelledby="final-preview-title" aria-describedby="final-preview-description">
        <header>
          <div><span>Pré-visualização</span><h2 id="final-preview-title">O teu resultado final</h2><p id="final-preview-description">Vê a composição sem guias ou caixas de edição antes de a adicionares ao carrinho.</p></div>
          {views.length > 1 && <div className="personalization-final-preview-tabs" role="tablist" aria-label="Escolher lado para pré-visualizar">{views.map((view, index) => <button key={view.id} type="button" role="tab" aria-selected={view.id === previewView.id} className={view.id === previewView.id ? 'selected' : ''} onClick={() => setPreviewViewId(view.id)}><b>{index + 1}</b>{view.label}</button>)}</div>}
        </header>
        <div className="personalization-final-preview-stage">
          {previewProductImage ? <div className="personalization-final-preview-canvas">
            <img src={previewProductImage} alt={`Resultado personalizado · ${previewView.label}`} />
            {previewView.printAreas.map((area) => {
              const design = designs[designKey(previewView.id, area.id)] ?? newDesign()
              if (!design.photoUrl && !design.text.trim()) return null
              return <div key={area.id} className="personalization-final-area" style={{ left: `${area.x}%`, top: `${area.y}%`, width: `${area.width}%`, height: `${area.height}%` }}>
                {design.photoUrl && <div className="personalization-final-element" style={{ left: `${design.photoFrame.x}%`, top: `${design.photoFrame.y}%`, width: `${design.photoFrame.width}%`, height: `${design.photoFrame.height}%` }}><img className="personalization-final-photo" src={design.photoUrl} alt="Fotografia da personalização" /></div>}
                {design.text.trim() && <div className="personalization-final-element personalization-final-element--text" style={{ left: `${design.textFrame.x}%`, top: `${design.textFrame.y}%`, width: `${design.textFrame.width}%`, height: `${design.textFrame.height}%` }}><span style={{ color: design.color, fontFamily: design.font, fontSize: `${design.size}px` }}>{design.text}</span></div>}
              </div>
            })}
          </div> : <p>Esta vista ainda não tem uma fotografia associada.</p>}
        </div>
        <footer><span>Ainda podes alterar qualquer fotografia, texto, tamanho ou posição.</span><div className="personalization-final-preview-actions"><button className="button button--secondary" type="button" autoFocus onClick={onPreviewClose}>Continuar a personalizar</button><button className="button button--primary" type="button" disabled={addToCartDisabled} onClick={onAddToCart}><ShoppingBag />{addToCartLabel}</button></div></footer>
      </section>
    </div>}
  </section>
}
