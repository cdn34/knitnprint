import type { PersonalizationConfig } from '@knitprint/api-client'
import { ImagePlus, Move, Type } from 'lucide-react'
import { type KeyboardEvent, type PointerEvent as ReactPointerEvent, type ReactNode, useEffect, useMemo, useRef, useState } from 'react'
import { cartApi } from '../cart-api'

const SUPPORTED_FONTS = ['Roboto', 'Montserrat', 'Playfair Display', 'Dancing Script', 'Pacifico'] as const
const DEFAULT_COLORS = ['#111111', '#ffffff', '#9c5263', '#1f4f78', '#b3232f']
const safeBasisPoints = (value: unknown, fallback: number) => typeof value === 'number' && Number.isFinite(value) ? value : fallback

type ElementFrame = { x: number; y: number; width: number; height: number }
type Interaction = { pointerX: number; pointerY: number; frame: ElementFrame; handle: 'move' | 'nw' | 'ne' | 'sw' | 'se' }
type PrintArea = ElementFrame & { id: string; label: string }

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

function configuredPrintAreas(config: PersonalizationConfig): PrintArea[] {
  const raw = config.print_areas
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
      }]
    })
    if (areas.length) return areas
  }
  return [{ id: 'area-1', label: 'Área 1', x: safeBasisPoints(config.area_x, 2500) / 100, y: safeBasisPoints(config.area_y, 2500) / 100, width: safeBasisPoints(config.area_width, 5000) / 100, height: safeBasisPoints(config.area_height, 5000) / 100 }]
}

export type CustomerCustomization = {
  version: 4
  areas: Array<{
    area_id: string
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
  photoCrop: { x: number; y: number; scale: number }
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
    aria-label={`${label}. Arrasta para mover e usa as setas nos cantos para redimensionar.`}
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
      aria-label={`Redimensionar ${label}`}
      onPointerDown={(event) => start(event, handle)}
      onPointerMove={move}
      onPointerUp={stop}
      onPointerCancel={stop}
    >{handle === 'nw' ? '↖' : handle === 'ne' ? '↗' : handle === 'sw' ? '↙' : '↘'}</button>)}
  </div>
}

export function ProductPersonalizer({ config, productImage, onChange }: Readonly<{
  config: PersonalizationConfig
  productImage?: string
  onChange: (value: { customization: CustomerCustomization | null; mediaIds: string[]; ready: boolean; missing: string[] }) => void
}>) {
  const fonts = useMemo(() => { const valid = Array.isArray(config.allowed_fonts) ? config.allowed_fonts.filter((value): value is typeof SUPPORTED_FONTS[number] => typeof value === 'string' && SUPPORTED_FONTS.includes(value as typeof SUPPORTED_FONTS[number])) : []; return valid.length ? valid : [...SUPPORTED_FONTS] }, [config.allowed_fonts])
  const colors = useMemo(() => { const valid = Array.isArray(config.allowed_colors) ? config.allowed_colors.filter((value): value is string => typeof value === 'string' && /^#[0-9a-f]{6}$/i.test(value)) : []; return valid.length ? valid : DEFAULT_COLORS }, [config.allowed_colors])
  const colorName = (value: string) => ({ '#111111': 'Preto', '#ffffff': 'Branco', '#9c5263': 'Rosa antigo', '#1f4f78': 'Azul', '#b3232f': 'Vermelho' }[value.toLowerCase()] ?? value)
  const wantsPhoto = config.mode === 'photo' || config.mode === 'photo_text'
  const wantsText = config.mode === 'text' || config.mode === 'photo_text'
  const combined = wantsPhoto && wantsText
  const printAreas = useMemo(() => configuredPrintAreas(config), [config.print_areas, config.area_x, config.area_y, config.area_width, config.area_height])
  const newDesign = (): AreaDesign => ({
    text: '',
    font: fonts[0] ?? 'Arial',
    color: colors[0] ?? '#111111',
    size: config.text_min_size,
    textFrame: combined ? { x: 10, y: 58, width: 80, height: 40 } : { x: 15, y: 35, width: 70, height: 30 },
    photoCrop: { x: 50, y: 50, scale: 1 },
    photoFrame: combined ? { x: 5, y: 5, width: 90, height: 50 } : { x: 15, y: 15, width: 70, height: 70 },
  })
  const [designs, setDesigns] = useState<Record<string, AreaDesign>>(() => Object.fromEntries(printAreas.map((area) => [area.id, newDesign()])))
  const [activeAreaId, setActiveAreaId] = useState(printAreas[0].id)
  const [selected, setSelected] = useState<'photo' | 'text'>(wantsPhoto ? 'photo' : 'text')
  const [uploadingAreas, setUploadingAreas] = useState<Record<string, boolean>>({})
  const objectUrls = useRef(new Set<string>())
  const activeDesign = designs[activeAreaId] ?? newDesign()

  function updateDesign(areaId: string, update: Partial<AreaDesign> | ((current: AreaDesign) => AreaDesign)) {
    setDesigns((current) => {
      const areaDesign = current[areaId] ?? newDesign()
      return { ...current, [areaId]: typeof update === 'function' ? update(areaDesign) : { ...areaDesign, ...update } }
    })
  }

  const customization = useMemo<CustomerCustomization>(() => ({
    version: 4,
    areas: printAreas.flatMap((area) => {
      const design = designs[area.id]
      if (!design) return []
      const photo = wantsPhoto && design.mediaId ? { media_id: design.mediaId, ...normalizedFrame(design.photoFrame), crop_x: design.photoCrop.x, crop_y: design.photoCrop.y, scale: design.photoCrop.scale } : undefined
      const text = wantsText && design.text.trim() ? { content: design.text.trim(), font: design.font, color: design.color, size: design.size, ...normalizedFrame(design.textFrame) } : undefined
      return photo || text ? [{ area_id: area.id, ...(photo ? { photo } : {}), ...(text ? { text } : {}) }] : []
    }),
  }), [designs, printAreas, wantsPhoto, wantsText])
  const mediaIds = useMemo(() => customization.areas.flatMap((area) => area.photo ? [area.photo.media_id] : []), [customization])
  const missing = useMemo(() => printAreas.flatMap((area) => {
    const design = designs[area.id]
    return [wantsPhoto && !design?.mediaId ? `fotografia em ${area.label}` : '', wantsText && !design?.text.trim() ? `texto em ${area.label}` : ''].filter(Boolean)
  }), [designs, printAreas, wantsPhoto, wantsText])
  const ready = missing.length === 0

  useEffect(() => onChange({ customization: customization.areas.length ? customization : null, mediaIds, ready, missing }), [customization, mediaIds, missing, onChange, ready])
  useEffect(() => () => { objectUrls.current.forEach((url) => URL.revokeObjectURL(url)); objectUrls.current.clear() }, [])
  useEffect(() => {
    setDesigns((current) => Object.fromEntries(printAreas.map((area) => [area.id, current[area.id] ?? newDesign()])))
    if (!printAreas.some(({ id }) => id === activeAreaId)) setActiveAreaId(printAreas[0].id)
  }, [printAreas, activeAreaId])

  async function upload(areaId: string, file?: File) {
    if (!file) return
    const currentUrl = designs[areaId]?.photoUrl
    if (currentUrl) { URL.revokeObjectURL(currentUrl); objectUrls.current.delete(currentUrl) }
    const photoUrl = URL.createObjectURL(file)
    objectUrls.current.add(photoUrl)
    updateDesign(areaId, { photoUrl, mediaId: undefined })
    setUploadingAreas((current) => ({ ...current, [areaId]: true }))
    setActiveAreaId(areaId)
    setSelected('photo')
    try {
      const uploadRequest = await cartApi.initiatePersonalizationUpload({ filename: file.name, content_type: file.type, byte_size: file.size })
      await cartApi.uploadMediaObject(uploadRequest.upload_url, file, file.type)
      const complete = await cartApi.completePersonalizationUpload(uploadRequest.id)
      updateDesign(areaId, { mediaId: complete.id })
    } finally {
      setUploadingAreas((current) => ({ ...current, [areaId]: false }))
    }
  }

  return <section className="personalizer" aria-labelledby="personalizer-title">
    <div className="personalizer-heading"><p>Cria a tua peça</p><h2 id="personalizer-title">Personaliza antes de adicionar</h2><span>Escolhe uma área e adiciona-lhe fotografia, texto ou ambos. Cada zona pode ter uma composição diferente.</span></div>
    <div className="personalizer-layout">
      <div className="personalizer-tools">
        {printAreas.length > 1 && <div className="personalizer-area-switcher" role="group" aria-label="Escolher área de impressão"><span>Área a editar</span><div>{printAreas.map((area, index) => <button key={area.id} type="button" className={activeAreaId === area.id ? 'selected' : ''} aria-pressed={activeAreaId === area.id} onClick={() => setActiveAreaId(area.id)}><b>{index + 1}</b>{area.label}</button>)}</div></div>}
        {wantsPhoto && <div className={`personalizer-tool${selected === 'photo' ? ' personalizer-tool--selected' : ''}`} onClick={() => setSelected('photo')}><strong><ImagePlus /> Fotografia · {printAreas.find(({ id }) => id === activeAreaId)?.label}</strong><label className="personalizer-upload">{uploadingAreas[activeAreaId] ? 'A preparar fotografia…' : activeDesign.photoUrl ? 'Trocar fotografia' : 'Carregar fotografia'}<input type="file" accept="image/jpeg,image/png,image/webp" disabled={uploadingAreas[activeAreaId]} onChange={(event) => void upload(activeAreaId, event.currentTarget.files?.[0])} /></label>{activeDesign.photoUrl && <label>Zoom<input type="range" min="1" max="3" step="0.05" value={activeDesign.photoCrop.scale} onChange={(event) => updateDesign(activeAreaId, (current) => ({ ...current, photoCrop: { ...current.photoCrop, scale: Number(event.target.value) } }))} /></label>}<span className="personalizer-drag-hint"><Move /> Arrasta e dimensiona a fotografia dentro desta área.</span></div>}
        {wantsText && <div className={`personalizer-tool${selected === 'text' ? ' personalizer-tool--selected' : ''}`} onClick={() => setSelected('text')}><strong><Type /> Texto · {printAreas.find(({ id }) => id === activeAreaId)?.label}</strong><label>O teu texto<textarea rows={2} maxLength={config.text_max_characters} value={activeDesign.text} onChange={(event) => { updateDesign(activeAreaId, { text: event.target.value }); setSelected('text') }} placeholder="Escreve aqui" /></label><small>{activeDesign.text.length} / {config.text_max_characters}</small><span className="personalizer-drag-hint"><Move /> Arrasta e dimensiona o texto dentro desta área.</span><span className="personalizer-control-label">Tipo de letra</span><div className="font-choice-grid">{fonts.map((value) => <button key={value} type="button" className={activeDesign.font === value ? 'selected' : ''} aria-pressed={activeDesign.font === value} onClick={() => updateDesign(activeAreaId, { font: value })}><b style={{ fontFamily: value }}>Ag</b><small>{value}</small></button>)}</div><label>Cor<select value={activeDesign.color} onChange={(event) => updateDesign(activeAreaId, { color: event.target.value })}>{colors.map((value) => <option key={value} value={value}>{colorName(value)} · {value}</option>)}</select></label><span className="selected-color"><i style={{ background: activeDesign.color }} />{colorName(activeDesign.color)}</span><label>Tamanho<input type="range" min={config.text_min_size} max={config.text_max_size} value={activeDesign.size} onChange={(event) => updateDesign(activeAreaId, { size: Number(event.target.value) })} /></label></div>}
      </div>
      <div className="personalizer-stage">
        {productImage ? <div className="personalizer-canvas"><img className="personalizer-product" src={productImage} alt="Pré-visualização do produto" />
          {printAreas.map((area) => {
            const design = designs[area.id] ?? newDesign()
            const active = activeAreaId === area.id
            return <div key={area.id} className={`personalizer-print-area${active ? ' personalizer-print-area--active' : ''}`} aria-label={`Área de impressão: ${area.label}`} style={{ left: `${area.x}%`, top: `${area.y}%`, width: `${area.width}%`, height: `${area.height}%` }} onClick={() => setActiveAreaId(area.id)}>
              <span className="personalizer-print-area-label">{area.label}</span>
              {wantsPhoto && (active || design.mediaId) && <DesignElement frame={design.photoFrame} kind="photo" label={`Fotografia em ${area.label}`} selected={active && selected === 'photo'} onSelect={() => { setActiveAreaId(area.id); setSelected('photo') }} onChange={(photoFrame) => updateDesign(area.id, { photoFrame })}>{design.photoUrl ? <img className="personalizer-photo" src={design.photoUrl} alt="Fotografia carregada" draggable={false} style={{ left: `${design.photoCrop.x}%`, top: `${design.photoCrop.y}%`, transform: `translate(-50%, -50%) scale(${design.photoCrop.scale})` }} /> : <span className="personalizer-placeholder"><ImagePlus /> Fotografia</span>}</DesignElement>}
              {wantsText && (active || design.text.trim()) && <DesignElement frame={design.textFrame} kind="text" label={`Texto em ${area.label}`} selected={active && selected === 'text'} onSelect={() => { setActiveAreaId(area.id); setSelected('text') }} onChange={(textFrame) => updateDesign(area.id, { textFrame })}>{design.text.trim() ? <span className="personalizer-text" style={{ color: design.color, fontFamily: design.font, fontSize: `${design.size}px` }}>{design.text}</span> : <span className="personalizer-placeholder"><Type /> Texto</span>}</DesignElement>}
            </div>
          })}
        </div> : <div className="personalizer-product-empty">Pré-visualização do produto</div>}
      </div>
    </div>
  </section>
}
