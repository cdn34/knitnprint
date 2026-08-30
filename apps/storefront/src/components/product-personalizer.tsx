import type { PersonalizationConfig } from '@knitprint/api-client'
import { ImagePlus, Type } from 'lucide-react'
import { useEffect, useMemo, useRef, useState } from 'react'
import { cartApi } from '../cart-api'

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
  const fonts = useMemo(() => (Array.isArray(config.allowed_fonts) ? config.allowed_fonts.filter((v): v is string => typeof v === 'string') : ['Arial']), [config.allowed_fonts])
  const colors = useMemo(() => (Array.isArray(config.allowed_colors) ? config.allowed_colors.filter((v): v is string => typeof v === 'string') : ['#111111']), [config.allowed_colors])
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
        {wantsText && <div className="personalizer-tool"><strong><Type /> Texto</strong><label>O teu texto<textarea rows={2} maxLength={config.text_max_characters} value={text} onChange={(event) => setText(event.target.value)} placeholder="Escreve aqui" /></label><small>{text.length} / {config.text_max_characters}</small><label>Tipo de letra<select value={font} onChange={(event) => setFont(event.target.value)}>{fonts.map((value) => <option key={value}>{value}</option>)}</select></label><label>Cor<select value={color} onChange={(event) => setColor(event.target.value)}>{colors.map((value) => <option key={value} value={value}>{value}</option>)}</select></label><label>Tamanho<input type="range" min={config.text_min_size} max={config.text_max_size} value={size} onChange={(event) => setSize(Number(event.target.value))} /></label></div>}
      </div>
      <div className="personalizer-stage">
        {productImage ? <img className="personalizer-product" src={productImage} alt="Pré-visualização do produto" /> : <div className="personalizer-product-empty">Pré-visualização do produto</div>}
        <div className="personalizer-zone" style={{ left: `${config.area_x / 100}%`, top: `${config.area_y / 100}%`, width: `${config.area_width / 100}%`, height: `${config.area_height / 100}%` }}>
          {photoUrl && <img className="personalizer-photo" src={photoUrl} alt="Fotografia carregada" draggable={false} style={{ left: `${photo.x}%`, top: `${photo.y}%`, transform: `translate(-50%, -50%) scale(${photo.scale})` }} onPointerDown={(event) => { event.currentTarget.setPointerCapture(event.pointerId); drag.current = { x: event.clientX, y: event.clientY, startX: photo.x, startY: photo.y } }} onPointerMove={(event) => { if (!drag.current) return; const rect = event.currentTarget.parentElement?.getBoundingClientRect(); if (!rect) return; setPhoto((current) => ({ ...current, x: Math.max(0, Math.min(100, drag.current!.startX + (event.clientX - drag.current!.x) / rect.width * 100)), y: Math.max(0, Math.min(100, drag.current!.startY + (event.clientY - drag.current!.y) / rect.height * 100)) })) }} onPointerUp={() => { drag.current = undefined }} />}
          {text.trim() && <span className="personalizer-text" style={{ color, fontFamily: font, fontSize: `${size}px` }}>{text}</span>}
          {!photoUrl && !text.trim() && <span className="personalizer-placeholder">A tua personalização aparece aqui</span>}
        </div>
      </div>
    </div>
  </section>
}
