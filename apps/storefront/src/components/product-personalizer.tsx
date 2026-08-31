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
  version: 3
  text?: { content: string; font: string; color: string; size: number; area_id: string; x: number; y: number; width: number; height: number }
  photo?: { area_id: string; x: number; y: number; width: number; height: number; crop_x: number; crop_y: number; scale: number }
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
  onChange: (value: { customization: CustomerCustomization | null; mediaId?: string; ready: boolean }) => void
}>) {
  const fonts = useMemo(() => { const valid = Array.isArray(config.allowed_fonts) ? config.allowed_fonts.filter((value): value is typeof SUPPORTED_FONTS[number] => typeof value === 'string' && SUPPORTED_FONTS.includes(value as typeof SUPPORTED_FONTS[number])) : []; return valid.length ? valid : [...SUPPORTED_FONTS] }, [config.allowed_fonts])
  const colors = useMemo(() => { const valid = Array.isArray(config.allowed_colors) ? config.allowed_colors.filter((value): value is string => typeof value === 'string' && /^#[0-9a-f]{6}$/i.test(value)) : []; return valid.length ? valid : DEFAULT_COLORS }, [config.allowed_colors])
  const colorName = (value: string) => ({ '#111111': 'Preto', '#ffffff': 'Branco', '#9c5263': 'Rosa antigo', '#1f4f78': 'Azul', '#b3232f': 'Vermelho' }[value.toLowerCase()] ?? value)
  const wantsPhoto = config.mode === 'photo' || config.mode === 'photo_text'
  const wantsText = config.mode === 'text' || config.mode === 'photo_text'
  const combined = wantsPhoto && wantsText
  const printAreas = useMemo(() => configuredPrintAreas(config), [config.print_areas, config.area_x, config.area_y, config.area_width, config.area_height])
  const [text, setText] = useState('')
  const [font, setFont] = useState(fonts[0] ?? 'Arial')
  const [color, setColor] = useState(colors[0] ?? '#111111')
  const [size, setSize] = useState(config.text_min_size)
  const [photoUrl, setPhotoUrl] = useState<string>()
  const [mediaId, setMediaId] = useState<string>()
  const [photoCrop, setPhotoCrop] = useState({ x: 50, y: 50, scale: 1 })
  const [photoFrame, setPhotoFrame] = useState<ElementFrame>(combined ? { x: 5, y: 5, width: 90, height: 50 } : { x: 15, y: 15, width: 70, height: 70 })
  const [textFrame, setTextFrame] = useState<ElementFrame>(combined ? { x: 10, y: 58, width: 80, height: 40 } : { x: 15, y: 35, width: 70, height: 30 })
  const [photoAreaId, setPhotoAreaId] = useState(printAreas[0].id)
  const [textAreaId, setTextAreaId] = useState(printAreas[0].id)
  const [selected, setSelected] = useState<'photo' | 'text'>(wantsPhoto ? 'photo' : 'text')
  const [uploading, setUploading] = useState(false)
  const customization: CustomerCustomization = {
    version: 3,
    ...(wantsPhoto && mediaId ? { photo: { area_id: photoAreaId, ...photoFrame, crop_x: photoCrop.x, crop_y: photoCrop.y, scale: photoCrop.scale } } : {}),
    ...(wantsText && text.trim() ? { text: { content: text.trim(), font, color, size, area_id: textAreaId, ...textFrame } } : {}),
  }
  const hasCustomization = Boolean(customization.photo || customization.text)
  const ready = (!wantsPhoto || Boolean(mediaId)) && (!wantsText || Boolean(text.trim()))

  useEffect(() => onChange({ customization: hasCustomization ? customization : null, mediaId, ready }), [text, font, color, size, textAreaId, textFrame.x, textFrame.y, textFrame.width, textFrame.height, photoAreaId, photoFrame.x, photoFrame.y, photoFrame.width, photoFrame.height, photoCrop.x, photoCrop.y, photoCrop.scale, mediaId, ready])
  useEffect(() => () => { if (photoUrl) URL.revokeObjectURL(photoUrl) }, [photoUrl])
  useEffect(() => {
    if (!printAreas.some(({ id }) => id === photoAreaId)) setPhotoAreaId(printAreas[0].id)
    if (!printAreas.some(({ id }) => id === textAreaId)) setTextAreaId(printAreas[0].id)
  }, [printAreas, photoAreaId, textAreaId])

  async function upload(file?: File) {
    if (!file) return
    if (photoUrl) URL.revokeObjectURL(photoUrl)
    setPhotoUrl(URL.createObjectURL(file)); setMediaId(undefined); setUploading(true); setSelected('photo')
    try {
      const uploadRequest = await cartApi.initiatePersonalizationUpload({ filename: file.name, content_type: file.type, byte_size: file.size })
      await cartApi.uploadMediaObject(uploadRequest.upload_url, file, file.type)
      const complete = await cartApi.completePersonalizationUpload(uploadRequest.id)
      setMediaId(complete.id)
    } finally { setUploading(false) }
  }

  return <section className="personalizer" aria-labelledby="personalizer-title">
    <div className="personalizer-heading"><p>Cria a tua peça</p><h2 id="personalizer-title">Personaliza antes de adicionar</h2><span>As linhas tracejadas delimitam as áreas de impressão. Escolhe a zona e organiza cada elemento dentro dela.</span></div>
    <div className="personalizer-layout">
      <div className="personalizer-tools">
        {wantsPhoto && <div className={`personalizer-tool${selected === 'photo' ? ' personalizer-tool--selected' : ''}`} onClick={() => setSelected('photo')}><strong><ImagePlus /> Fotografia</strong>{printAreas.length > 1 && <label>Área de impressão<select value={photoAreaId} onChange={(event) => { setPhotoAreaId(event.target.value); setSelected('photo') }}>{printAreas.map((area) => <option key={area.id} value={area.id}>{area.label}</option>)}</select></label>}<label className="personalizer-upload">{uploading ? 'A preparar fotografia…' : photoUrl ? 'Trocar fotografia' : 'Carregar fotografia'}<input type="file" accept="image/jpeg,image/png,image/webp" disabled={uploading} onChange={(event) => void upload(event.currentTarget.files?.[0])} /></label>{photoUrl && <label>Zoom<input type="range" min="1" max="3" step="0.05" value={photoCrop.scale} onChange={(event) => setPhotoCrop((current) => ({ ...current, scale: Number(event.target.value) }))} /></label>}<span className="personalizer-drag-hint"><Move /> Arrasta a caixa na área escolhida e usa as setas dos cantos para a dimensionar.</span></div>}
        {wantsText && <div className={`personalizer-tool${selected === 'text' ? ' personalizer-tool--selected' : ''}`} onClick={() => setSelected('text')}><strong><Type /> Texto</strong>{printAreas.length > 1 && <label>Área de impressão<select value={textAreaId} onChange={(event) => { setTextAreaId(event.target.value); setSelected('text') }}>{printAreas.map((area) => <option key={area.id} value={area.id}>{area.label}</option>)}</select></label>}<label>O teu texto<textarea rows={2} maxLength={config.text_max_characters} value={text} onChange={(event) => { setText(event.target.value); setSelected('text') }} placeholder="Escreve aqui" /></label><small>{text.length} / {config.text_max_characters}</small><span className="personalizer-drag-hint"><Move /> Arrasta a caixa na área escolhida e usa as setas dos cantos para a dimensionar.</span><span className="personalizer-control-label">Tipo de letra</span><div className="font-choice-grid">{fonts.map((value) => <button key={value} type="button" className={font === value ? 'selected' : ''} aria-pressed={font === value} onClick={() => setFont(value)}><b style={{ fontFamily: value }}>Ag</b><small>{value}</small></button>)}</div><label>Cor<select value={color} onChange={(event) => setColor(event.target.value)}>{colors.map((value) => <option key={value} value={value}>{colorName(value)} · {value}</option>)}</select></label><span className="selected-color"><i style={{ background: color }} />{colorName(color)}</span><label>Tamanho<input type="range" min={config.text_min_size} max={config.text_max_size} value={size} onChange={(event) => setSize(Number(event.target.value))} /></label></div>}
      </div>
      <div className="personalizer-stage">
        {productImage ? <div className="personalizer-canvas"><img className="personalizer-product" src={productImage} alt="Pré-visualização do produto" />
          {printAreas.map((area) => <div key={area.id} className={`personalizer-print-area${(selected === 'photo' ? photoAreaId : textAreaId) === area.id ? ' personalizer-print-area--active' : ''}`} aria-label={`Área de impressão: ${area.label}`} style={{ left: `${area.x}%`, top: `${area.y}%`, width: `${area.width}%`, height: `${area.height}%` }}>
            <span className="personalizer-print-area-label">{area.label}</span>
            {wantsPhoto && photoAreaId === area.id && <DesignElement frame={photoFrame} kind="photo" label="Fotografia" selected={selected === 'photo'} onSelect={() => setSelected('photo')} onChange={setPhotoFrame}>{photoUrl ? <img className="personalizer-photo" src={photoUrl} alt="Fotografia carregada" draggable={false} style={{ left: `${photoCrop.x}%`, top: `${photoCrop.y}%`, transform: `translate(-50%, -50%) scale(${photoCrop.scale})` }} /> : <span className="personalizer-placeholder"><ImagePlus /> Fotografia</span>}</DesignElement>}
            {wantsText && textAreaId === area.id && <DesignElement frame={textFrame} kind="text" label="Texto" selected={selected === 'text'} onSelect={() => setSelected('text')} onChange={setTextFrame}>{text.trim() ? <span className="personalizer-text" style={{ color, fontFamily: font, fontSize: `${size}px` }}>{text}</span> : <span className="personalizer-placeholder"><Type /> Texto</span>}</DesignElement>}
          </div>)}
        </div> : <div className="personalizer-product-empty">Pré-visualização do produto</div>}
      </div>
    </div>
  </section>
}
