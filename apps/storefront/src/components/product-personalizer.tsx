import type { PersonalizationConfig } from '@knitprint/api-client'
import { ImagePlus, Type } from 'lucide-react'
import { useEffect, useMemo, useRef, useState } from 'react'
import { cartApi } from '../cart-api'

const SUPPORTED_FONTS = ['Roboto', 'Montserrat', 'Playfair Display', 'Dancing Script', 'Pacifico'] as const
const DEFAULT_COLORS = ['#111111', '#ffffff', '#9c5263', '#1f4f78', '#b3232f']
const safeBasisPoints = (value: unknown, fallback: number) => typeof value === 'number' && Number.isFinite(value) ? value : fallback

export type CustomerCustomization = {
  version: 1
  text?: { content: string; font: string; color: string; size: number; x: number; y: number }
  photo?: { x: number; y: number; scale: number }
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
  const [text, setText] = useState('')
  const [font, setFont] = useState(fonts[0] ?? 'Arial')
  const [color, setColor] = useState(colors[0] ?? '#111111')
  const [size, setSize] = useState(config.text_min_size)
  const [photoUrl, setPhotoUrl] = useState<string>()
  const [mediaId, setMediaId] = useState<string>()
  const [photo, setPhoto] = useState({ x: 50, y: 50, scale: 1 })
  const [uploading, setUploading] = useState(false)
  const drag = useRef<{ x: number; y: number; startX: number; startY: number } | undefined>(undefined)
  const customization: CustomerCustomization = {
    version: 1,
    ...(wantsPhoto && photoUrl ? { photo } : {}),
    ...(wantsText && text.trim() ? { text: { content: text.trim(), font, color, size, x: 50, y: 75 } } : {}),
  }
  const ready = (!wantsPhoto || Boolean(mediaId)) && (!wantsText || Boolean(text.trim()))

  useEffect(() => onChange({ customization: ready ? customization : null, mediaId, ready }), [text, font, color, size, photo.x, photo.y, photo.scale, mediaId, ready])
  useEffect(() => () => { if (photoUrl) URL.revokeObjectURL(photoUrl) }, [photoUrl])

  async function upload(file?: File) {
    if (!file) return
    if (photoUrl) URL.revokeObjectURL(photoUrl)
    setPhotoUrl(URL.createObjectURL(file)); setMediaId(undefined); setUploading(true)
    try {
      const upload = await cartApi.initiatePersonalizationUpload({ filename: file.name, content_type: file.type, byte_size: file.size })
      await cartApi.uploadMediaObject(upload.upload_url, file, file.type)
      const complete = await cartApi.completePersonalizationUpload(upload.id)
      setMediaId(complete.id)
    } finally { setUploading(false) }
  }

  return <section className="personalizer" aria-labelledby="personalizer-title">
    <div className="personalizer-heading"><p>Cria a tua peça</p><h2 id="personalizer-title">Personaliza antes de adicionar</h2><span>A linha tracejada mostra exatamente a zona que pode ser impressa.</span></div>
    <div className="personalizer-layout">
      <div className="personalizer-tools">
        {wantsPhoto && <div className="personalizer-tool"><strong><ImagePlus /> Fotografia</strong><label className="personalizer-upload">{uploading ? 'A preparar fotografia…' : photoUrl ? 'Trocar fotografia' : 'Carregar fotografia'}<input type="file" accept="image/jpeg,image/png,image/webp" disabled={uploading} onChange={(event) => void upload(event.currentTarget.files?.[0])} /></label>{photoUrl && <label>Zoom<input type="range" min="1" max="3" step="0.05" value={photo.scale} onChange={(event) => setPhoto((current) => ({ ...current, scale: Number(event.target.value) }))} /></label>}</div>}
        {wantsText && <div className="personalizer-tool"><strong><Type /> Texto</strong><label>O teu texto<textarea rows={2} maxLength={config.text_max_characters} value={text} onChange={(event) => setText(event.target.value)} placeholder="Escreve aqui" /></label><small>{text.length} / {config.text_max_characters}</small><span className="personalizer-control-label">Tipo de letra</span><div className="font-choice-grid">{fonts.map((value) => <button key={value} type="button" className={font === value ? 'selected' : ''} aria-pressed={font === value} onClick={() => setFont(value)}><b style={{ fontFamily: value }}>Ag</b><small>{value}</small></button>)}</div><label>Cor<select value={color} onChange={(event) => setColor(event.target.value)}>{colors.map((value) => <option key={value} value={value}>{colorName(value)} · {value}</option>)}</select></label><span className="selected-color"><i style={{ background: color }} />{colorName(color)}</span><label>Tamanho<input type="range" min={config.text_min_size} max={config.text_max_size} value={size} onChange={(event) => setSize(Number(event.target.value))} /></label></div>}
      </div>
      <div className="personalizer-stage">
        {productImage ? <div className="personalizer-canvas"><img className="personalizer-product" src={productImage} alt="Pré-visualização do produto" />
        {wantsPhoto && <div className="personalizer-zone personalizer-zone--photo" style={{ left: `${safeBasisPoints(config.area_x, 2500) / 100}%`, top: `${safeBasisPoints(config.area_y, 2500) / 100}%`, width: `${safeBasisPoints(config.area_width, 5000) / 100}%`, height: `${safeBasisPoints(config.area_height, 5000) / 100}%` }}>
          {photoUrl && <img className="personalizer-photo" src={photoUrl} alt="Fotografia carregada" draggable={false} style={{ left: `${photo.x}%`, top: `${photo.y}%`, transform: `translate(-50%, -50%) scale(${photo.scale})` }} onPointerDown={(event) => { event.currentTarget.setPointerCapture(event.pointerId); drag.current = { x: event.clientX, y: event.clientY, startX: photo.x, startY: photo.y } }} onPointerMove={(event) => { if (!drag.current) return; const rect = event.currentTarget.parentElement?.getBoundingClientRect(); if (!rect) return; setPhoto((current) => ({ ...current, x: Math.max(0, Math.min(100, drag.current!.startX + (event.clientX - drag.current!.x) / rect.width * 100)), y: Math.max(0, Math.min(100, drag.current!.startY + (event.clientY - drag.current!.y) / rect.height * 100)) })) }} onPointerUp={() => { drag.current = undefined }} />}
          {!photoUrl && <span className="personalizer-placeholder">A fotografia aparece aqui</span>}
        </div>}
        {wantsText && <div className="personalizer-zone personalizer-zone--text" style={{ left: `${safeBasisPoints(config.text_area_x, 2500) / 100}%`, top: `${safeBasisPoints(config.text_area_y, 3000) / 100}%`, width: `${safeBasisPoints(config.text_area_width, 5000) / 100}%`, height: `${safeBasisPoints(config.text_area_height, 2500) / 100}%` }}>
          {text.trim() ? <span className="personalizer-text" style={{ color, fontFamily: font, fontSize: `${size}px` }}>{text}</span> : <span className="personalizer-placeholder">O texto aparece aqui</span>}
        </div>}</div> : <div className="personalizer-product-empty">Pré-visualização do produto</div>}
      </div>
    </div>
  </section>
}
