import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Check, ChevronRight, Eye, ImagePlus, Plus, Save, RefreshCw, Trash2 } from 'lucide-react'
import { isSizeOption, PrimaryVariantValues } from './variant-design-controls'
import { VariantStorePreview } from './variant-store-preview'
import {
  ApiError,
  createApiClient,
  type Product,
  type VariantConfig,
  type ProductOption,
  type OptionValue,
  type MatrixVariant,
} from '@knitnprint/api-client'
import {
  OptionFields,
  templateKey,
  optionCode,
} from './variant-fields'
const api = createApiClient()
type Row = Omit<MatrixVariant, 'id' | 'selections'> & {
  id?: string
  sku?: string
  selections: Record<string, string>
}
const identity = (selections: Record<string, string>) =>
  JSON.stringify(Object.entries(selections).sort())

export function VariantMatrix({
  product,
  onSaved,
  disabled = false,
}: {
  product: Product
  onSaved: () => void | Promise<void>
  disabled?: boolean
}) {
  const templates = useQuery({
    queryKey: templateKey,
    queryFn: api.variantTemplates,
    refetchOnWindowFocus: false,
  })
  const [config, setConfig] = useState<VariantConfig>(
    structuredClone(product.variant_config),
  )
  const [pools, setPools] = useState<Record<string, OptionValue[]>>(
    Object.fromEntries(
      product.variant_config.options.map((o) => [
        o.code,
        structuredClone(o.values),
      ]),
    ),
  )
  const [rows, setRows] = useState<Row[]>(
    product.variants.map((v) => ({
      id: v.id,
      sku: v.sku,
      selections: (v.selections ?? {}) as Record<string, string>,
      price_minor: v.price_minor,
      available_quantity: v.available_quantity,
      expected_quantity: v.available_quantity,
      retired: v.retired,
    })),
  )
  const [needsPreview, setNeedsPreview] = useState(false)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  const [message, setMessage] = useState('')
  const [showReferences, setShowReferences] = useState(false)
  const [showRetired, setShowRetired] = useState(false)
  const [view, setView] = useState<'admin' | 'shop'>('admin')
  const [basePrice, setBasePrice] = useState(String((product.variants.find(variant => !variant.retired)?.price_minor ?? 0) / 100))
  const [differentPrices, setDifferentPrices] = useState(new Set(product.variants.filter(variant => !variant.retired).map(variant => variant.price_minor)).size > 1)
  const colorOption = config.options.find(option => option.control === 'swatch')
  const sizeOption = config.options.find(isSizeOption)
  const sizeTemplate = templates.data?.find(template => template.definition.label === 'Size') ?? templates.data?.find(template => isSizeOption(template.definition))
  const colorTemplate = templates.data?.find(template => template.definition.control === 'swatch')
  const recordedStock = rows.filter(row => !row.retired).reduce((total, row) => total + row.available_quantity, 0)
  const combinations = config.options.length
    ? config.options.reduce((n, o) => n * o.values.length, 1)
    : 1
  const active = rows.filter((r) => !r.retired).length
  const currentCombination = (row: Row) => Object.keys(row.selections).length === config.options.length && config.options.every(option => option.values.some(value => value.code === row.selections[option.code]))
  function changeOption(index: number, next: ProductOption) {
    setConfig((c) => ({
      ...c,
      options: c.options.map((o, i) => (i === index ? next : o)),
    }))
    setPools((p) => ({
      ...p,
      [next.code]: [
        ...(p[next.code] ?? []).map(value => next.values.find(selected => selected.code === value.code) ?? value),
        ...next.values.filter(value => !(p[next.code] ?? []).some(existing => existing.code === value.code)),
      ],
    }))
    setNeedsPreview(true)
    setMessage('')
    setError('')
  }
  function addOption(option: ProductOption) {
    if (
      config.options.some(
        (o) =>
          o.label.trim().toLowerCase() === option.label.trim().toLowerCase(),
      )
    )
      return
    const copy = structuredClone(option)
    if (config.options.some((o) => o.code === copy.code))
      copy.code = optionCode(
        copy.label,
        config.options.map((o) => o.code),
      )
    setConfig((c) => ({ ...c, options: [...c.options, copy] }))
    setPools((p) => ({ ...p, [copy.code]: copy.values }))
    setNeedsPreview(true)
    setError('')
    setMessage('')
  }
  function generate() {
    for (const option of config.options) {
      if (!option.label.trim() || option.values.some((v) => !v.label.trim())) {
        setError(
          'Dá um nome a cada opção e valor antes de gerar as variantes.',
        )
        return
      }
      if (
        new Set(option.values.map((v) => v.label.trim().toLowerCase())).size !==
        option.values.length
      ) {
        setError(
          `Usa um nome diferente para cada valor de ${option.label.toLowerCase()}.`,
        )
        return
      }
    }
    if (
      new Set(config.options.map((o) => o.label.trim().toLowerCase())).size !==
      config.options.length
    ) {
      setError('Cada opção do produto precisa de um nome diferente.')
      return
    }

    if (!combinations || combinations > 1000) {
      setError(
        'Escolhe pelo menos um valor por opção, até ao máximo de 1.000 combinações para rever.',
      )
      return
    }
    let choices: Record<string, string>[] = [{}]
    for (const o of config.options)
      choices = choices.flatMap((selection) =>
        o.values.map((v) => ({ ...selection, [o.code]: v.code })),
      )
    const generated = choices.map(
      (selections) =>
        rows.find((r) => identity(r.selections) === identity(selections)) ?? {
          selections,
          price_minor: rows.find((r) => !r.retired)?.price_minor ?? 0,
          available_quantity: 0,
          retired: false,
        },
    )
    setRows([
      ...generated,
      ...rows
        .filter((r) => r.id && !generated.some((g) => g.id === r.id))
        .map((r) => ({ ...r, retired: true })),
    ])
    setNeedsPreview(false)
    setError('')
    setMessage(
      `${generated.length} combinações geradas. Revê os preços e o stock antes de guardar.`,
    )
  }
  async function save() {
    setBusy(true)
    setError('')
    setMessage('')
    try {
      const input = {
        revision: product.variant_revision,
        config,
        variants: rows.filter((r) => r.id || !r.retired),
      }
      await api.previewVariantMatrix(product.id, input)
      await api.saveVariantMatrix(product.id, input)
      await onSaved()
      setMessage('Opções e variantes guardadas.')
    } catch (e) {
      setError(
        e instanceof ApiError
          ? e.body.error.message
          : 'Não foi possível guardar as variantes. Tenta novamente.',
      )
    } finally {
      setBusy(false)
    }
  }
  const changeRow = (i: number, patch: Partial<Row>) =>
    setRows((rs) => rs.map((r, n) => (n === i ? { ...r, ...patch } : r)))
  function removeOption(code: string) {
    setConfig(current => ({ ...current, options: current.options.filter(option => option.code !== code) }))
    setNeedsPreview(true)
  }
  return (
    <section className="variant-ui variant-matrix" aria-labelledby="product-variants-heading">
      <header className="variant-workspace-heading">
        <div>
          <p className="variant-brand">KnitnPrint</p>
          <h2 id="product-variants-heading">Tamanhos e cores</h2>
          <span>{product.title} · Edição do produto</span>
        </div>
      </header>
      <div className="variant-workspace-tabs" role="tablist" aria-label="Vista do produto" onKeyDown={event => {
        if (!['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(event.key)) return
        event.preventDefault()
        const next = event.key === 'Home' ? 'admin' : event.key === 'End' ? 'shop' : view === 'admin' ? 'shop' : 'admin'
        setView(next)
        document.getElementById(`variant-${next}-tab`)?.focus()
      }}>
        <button id="variant-admin-tab" type="button" role="tab" tabIndex={view === 'admin' ? 0 : -1} aria-selected={view === 'admin'} aria-controls="variant-admin-panel" onClick={() => setView('admin')}>No administrador</button>
        <button id="variant-shop-tab" type="button" role="tab" tabIndex={view === 'shop' ? 0 : -1} aria-selected={view === 'shop'} aria-controls="variant-shop-panel" onClick={() => setView('shop')}>Como fica na loja</button>
      </div>
      {disabled && <p className="variant-message variant-workspace-notice">Guarda os detalhes do produto acima antes de alterar as variantes.</p>}
      <fieldset id="variant-admin-panel" className="variant-builder" role="tabpanel" aria-labelledby="variant-admin-tab" hidden={view !== 'admin'} disabled={disabled || busy}>
        <section className="variant-design-step" aria-labelledby="variant-options-title">
          <div className="variant-step-heading"><h3 id="variant-options-title"><span className="variant-step-number">1</span>Opções deste produto</h3></div>
          <div className="variant-primary-settings">
            <div className="variant-primary-toggles">
              <label className="variant-checkbox"><input type="checkbox" checked={Boolean(sizeOption)} disabled={!sizeOption && (!sizeTemplate || config.options.length >= 6)} onChange={event => {
                if (event.target.checked && sizeTemplate) addOption(sizeTemplate.definition)
                else if (sizeOption) removeOption(sizeOption.code)
              }} />Tem tamanhos</label>
              <label className="variant-checkbox"><input type="checkbox" checked={Boolean(colorOption)} disabled={!colorOption && (!colorTemplate || config.options.length >= 6)} onChange={event => {
                if (event.target.checked && colorTemplate) addOption(colorTemplate.definition)
                else if (colorOption) removeOption(colorOption.code)
              }} />Tem cores</label>
            </div>
            <label className="variant-base-price">Preço base (€)<input type="number" min="0" step=".01" value={basePrice} onChange={event => {
              setBasePrice(event.target.value)
              if (event.target.value) setRows(current => current.map(row => row.retired ? row : { ...row, price_minor: Math.round(Number(event.target.value) * 100) }))
            }} /></label>
          </div>
          <div className="variant-primary-grid">
            {sizeOption && <PrimaryVariantValues option={sizeOption} values={pools[sizeOption.code] ?? sizeOption.values} kind="size" onChange={next => changeOption(config.options.indexOf(sizeOption), next)} />}
            {colorOption && <PrimaryVariantValues option={colorOption} values={pools[colorOption.code] ?? colorOption.values} kind="color" onChange={next => changeOption(config.options.indexOf(colorOption), next)} />}
          </div>
          {templates.isPending && <p className="variant-help" role="status">A carregar os modelos…</p>}
          {templates.isError && <p className="panel-error" role="alert">Não foi possível carregar os modelos. <button className="variant-button" type="button" onClick={() => void templates.refetch()}>Tentar novamente</button></p>}
          <details className="variant-model-library">
            <summary>Modelos e outras opções</summary>
            <p className="variant-help">Usa modelos de tamanhos, cores, materiais ou outras opções. <a href="#variant-templates">Variant templates <ChevronRight size={13} aria-hidden="true" /></a></p>
            <div className="variant-template-choices" aria-label="Modelos disponíveis">
              {templates.data?.map(template => {
                const added = config.options.some(option => option.label.trim().toLowerCase() === template.definition.label.trim().toLowerCase())
                const colorBlocked = template.definition.control === 'swatch' && config.options.some(option => option.control === 'swatch')
                return <button className={`variant-template-choice${added ? ' added' : ''}`} key={template.id} aria-label={`Adicionar opção ${template.definition.label}`} aria-pressed={added} type="button" disabled={added || colorBlocked || config.options.length >= 6} onClick={() => addOption(template.definition)}>
                  {added ? <Check size={16} aria-hidden="true" /> : <Plus size={16} aria-hidden="true" />}
                  <span><strong>{template.definition.label}</strong><small>{added ? 'Adicionada' : colorBlocked ? 'Só uma opção de cor por produto' : `${template.definition.values.length} valores`}</small></span>
                </button>
              })}
            </div>
            {templates.data?.length === 0 && <p className="variant-help">Cria um modelo em <a href="#variant-templates">Variant templates</a> para o aplicar a este produto.</p>}
            <div className="product-option-grid">
              {config.options.map((option, index) => <section className="product-option-card" key={option.code} aria-label={`Opção ${option.label}`}>
                <header><div><h4>{option.label}</h4><span>{option.values.length} valores selecionados</span></div><button type="button" className="variant-icon-button danger" aria-label={`Remover opção ${option.label}`} onClick={() => removeOption(option.code)}><Trash2 size={16} aria-hidden="true" /></button></header>
                {option !== sizeOption && option !== colorOption && <div className="variant-value-choices">{(pools[option.code] ?? option.values).map(value => <label className="variant-value-choice" key={value.code}>
                  <input type="checkbox" aria-label={`${option.label}: ${value.label}`} checked={option.values.some(selected => selected.code === value.code)} onChange={event => changeOption(index, { ...option, values: event.target.checked ? [...option.values, value] : option.values.filter(selected => selected.code !== value.code) })} />{value.label}
                </label>)}</div>}
                <details className="variant-customize"><summary>Personalizar nomes e valores de {option.label.toLowerCase()}</summary><OptionFields option={option} allowSwatch={!config.options.some(axis => axis.code !== option.code && axis.control === 'swatch')} onChange={next => changeOption(index, next)} /></details>
              </section>)}
            </div>
          </details>
        </section>

        <section className="variant-design-step" aria-labelledby="variant-photos-title">
          <div className="variant-step-heading">
            <h3 id="variant-photos-title"><span className="variant-step-number">2</span>{colorOption ? 'Fotografias por cor' : 'Fotografias do produto'}</h3>
            <span>As mesmas fotografias para todos os tamanhos</span>
          </div>
          {colorOption ? <>
            <details className="variant-gallery-settings"><summary>Opções da galeria</summary><label className="variant-checkbox variant-color-photo-toggle"><input type="checkbox" checked={config.color_photos} onChange={event => setConfig(current => ({ ...current, color_photos: event.target.checked }))} />Usar fotografias específicas por cor (opcional)</label><p className="variant-help">As cores sem fotografias associadas usam a galeria partilhada do produto.</p></details>
            <div className="variant-photo-grid">
              {colorOption.values.map(value => {
                const assigned = product.media.filter(media => value.media_ids?.includes(media.id))
                const first = config.color_photos ? assigned[0] : undefined
                return <section className="variant-color-gallery" key={value.code} aria-label={`Fotografias de ${value.label}`}>
                  <div className="variant-color-gallery-summary">
                    <div className="variant-color-thumbnail">{first ? <img src={`/api/admin/product-media/${encodeURIComponent(first.id)}/thumbnail`} alt={first.alt_text || `Fotografia de ${value.label}`} /> : <ImagePlus size={24} aria-hidden="true" />}</div>
                    <div>
                      <h4><i className="variant-design-swatch" style={{ background: value.color ?? '#000000' }} aria-hidden="true" />{value.label}</h4>
                      <p>{!product.media.length ? 'Fotografias por adicionar' : config.color_photos && assigned.length ? `${assigned.length} fotografia${assigned.length === 1 ? '' : 's'} associada${assigned.length === 1 ? '' : 's'}` : 'Galeria partilhada'}</p>
                      <span>{config.color_photos && assigned.length ? 'Todos os tamanhos desta cor' : 'Sem fotos próprias · usa a galeria partilhada'}</span>
                    </div>
                  </div>
                  <details className="variant-photo-picker" onToggle={event => {
                    if (!disabled && !busy && event.currentTarget.open && !config.color_photos) setConfig(current => ({ ...current, color_photos: true }))
                  }}>
                    <summary><ImagePlus size={15} aria-hidden="true" />{assigned.length ? 'Alterar fotos' : 'Adicionar fotos'}</summary>
                    {product.media.length ? <div className="variant-photo-choices">{product.media.map(media => <label key={media.id}>
                      <input type="checkbox" aria-label={`${value.label}: ${media.alt_text}`} checked={value.media_ids?.includes(media.id) ?? false} onChange={event => setConfig(current => ({ ...current, options: current.options.map(axis => axis.code === colorOption.code ? {
                        ...axis,
                        values: axis.values.map(selected => selected.code === value.code ? { ...selected, media_ids: event.target.checked ? [...(selected.media_ids ?? []), media.id] : (selected.media_ids ?? []).filter(id => id !== media.id) } : selected),
                      } : axis) }))} />
                      <img src={`/api/admin/product-media/${encodeURIComponent(media.id)}/thumbnail`} alt={media.alt_text} />
                      <span>{media.alt_text || 'Fotografia do produto'}</span>
                    </label>)}</div> : <p className="variant-help">Adiciona fotografias ao produto acima para as associares às cores.</p>}
                  </details>
                </section>
              })}
            </div>
          </> : <div className="variant-shared-gallery">
            {product.media.length ? product.media.map(media => <figure key={media.id}><img src={`/api/admin/product-media/${encodeURIComponent(media.id)}/thumbnail`} alt={media.alt_text || product.title} /><figcaption>{media.alt_text || 'Fotografia do produto'}</figcaption></figure>) : <p className="variant-help">Adiciona as fotografias no editor do produto. Serão partilhadas por todas as variantes.</p>}
          </div>}
        </section>

        {config.options.length > 0 && product.variants.some(variant => !variant.selections && !variant.retired) && <details className="variant-design-legacy legacy-mapping">
          <summary>Associar o stock existente às opções (opcional)</summary>
          <p className="variant-help">Associa as opções a cada referência antes de gerar as combinações para manter o stock e o histórico. As variantes que não forem associadas serão desativadas.</p>
          {rows.map((row, index) => row.id && !product.variants.find(variant => variant.id === row.id)?.selections && <div className="legacy-mapping-row" key={row.id}>
            <strong>{row.sku}</strong>
            {config.options.map(option => <label key={option.code}>{option.label}<select aria-label={`Associar ${option.label} ${index + 1}`} value={row.selections[option.code] ?? ''} onChange={event => {
              changeRow(index, { selections: { ...row.selections, [option.code]: event.target.value } })
              setNeedsPreview(true)
            }}><option value="">Escolher</option>{option.values.map(value => <option key={value.code} value={value.code}>{value.label}</option>)}</select></label>)}
          </div>)}
        </details>}

        <section className="variant-design-step variant-combinations-step" aria-labelledby="variant-combinations-title">
          <div className="variant-step-heading">
            <h3 id="variant-combinations-title"><span className="variant-step-number">3</span>Combinações geradas</h3>
            <span className="variant-stock-total" aria-live="polite">{active} variantes · {recordedStock} peças</span>
          </div>
          {needsPreview && <div className="variant-generation-bar">
            <p className="variant-help">{config.options.map(option => `${option.values.length} ${option.label.toLowerCase()}`).join(' × ') || 'Sem opções: uma variante única'} = {combinations} combinaç{combinations === 1 ? 'ão' : 'ões'}</p>
            <button className="variant-button" type="button" disabled={!combinations || combinations > 1000} onClick={generate}><RefreshCw size={15} aria-hidden="true" />Gerar {combinations} variantes</button>
          </div>}
          {needsPreview && product.variants.some(variant => !variant.retired && variant.selections && Object.keys(variant.selections).sort().join() !== config.options.map(option => option.code).sort().join()) && <p className="variant-message">Alterar as opções cria novas variantes. As combinações anteriores serão desativadas ao guardar, mantendo as referências e o histórico de stock. Introduz o stock das novas combinações antes de guardar.</p>}
          {needsPreview ? <p className="variant-message" role="status">As opções foram alteradas. Gera as combinações para continuar.</p> : <>
            <div className="matrix-scroll" role="region" aria-label="Combinações de variantes" tabIndex={0}>
              <table>
                <caption className="sr-only">Combinações de variantes, preços e stock registado</caption>
                <thead><tr><th scope="col">Ativa</th><th scope="col">Variante</th>{showReferences && <th scope="col">Referência</th>}<th scope="col">Preço</th><th scope="col">Stock</th><th scope="col">Estado</th></tr></thead>
                <tbody>{rows.map((row, index) => (!row.retired || showRetired || (currentCombination(row) && !product.variants.find(variant => variant.id === row.id)?.retired)) && <tr className={row.retired ? 'retired' : ''} key={row.id ?? identity(row.selections)}>
                  <td><input aria-label={`Ativar combinação ${index + 1}`} type="checkbox" checked={!row.retired} onChange={event => changeRow(index, { retired: !event.target.checked })} /></td>
                  <td><span className="variant-cell-value">{colorOption && <i className="variant-design-swatch" style={{ background: (pools[colorOption.code] ?? colorOption.values).find(value => value.code === row.selections[colorOption.code])?.color ?? '#000000' }} aria-hidden="true" />}{[...config.options].sort((a, b) => Number(b.control === 'swatch') - Number(a.control === 'swatch')).map(option => (pools[option.code] ?? option.values).find(value => value.code === row.selections[option.code])?.label ?? row.selections[option.code] ?? '—').join(' · ') || 'Única'}</span></td>
                  {showReferences && <td><code>{row.sku ?? 'Gerada ao guardar'}</code></td>}
                  <td>{differentPrices ? <input aria-label={`Preço ${index + 1}`} type="number" min="0" step=".01" value={row.price_minor / 100} onChange={event => changeRow(index, { price_minor: Math.round(Number(event.target.value) * 100) })} /> : <span className="variant-formatted-price">{new Intl.NumberFormat('pt-PT', { style: 'currency', currency: 'EUR' }).format(row.price_minor / 100)}</span>}</td>
                  <td><input aria-label={`Stock ${index + 1}`} type="number" step="1" value={row.available_quantity} onChange={event => changeRow(index, { available_quantity: Number(event.target.value) })} /></td>
                  <td><span className={`variant-row-state${row.retired ? ' inactive' : row.available_quantity <= 0 ? ' backorder' : ''}`}>{row.retired ? 'Desativada' : row.available_quantity > 0 ? 'Disponível' : 'Por encomenda'}</span></td>
                </tr>)}</tbody>
              </table>
            </div>
            <label className="variant-checkbox variant-price-mode"><input type="checkbox" checked={differentPrices} onChange={event => setDifferentPrices(event.target.checked)} />Definir preços diferentes por variante</label>
            <details className="variant-review-controls variant-more-options">
              <summary>Opções avançadas das combinações</summary>
              <details className="variant-bulk-editor">
                <summary>Aplicar preço ou stock em lote</summary>
                <div className="variant-bulk-fields">
                  <label>Preço para todas (€)<input type="number" min="0" step=".01" placeholder="Aplicar às variantes ativas" onChange={event => {
                    setBasePrice(event.target.value)
                    if (event.target.value) setRows(current => current.map(row => row.retired ? row : { ...row, price_minor: Math.round(Number(event.target.value) * 100) }))
                  }} /></label>
                  <label>Stock registado para todas<input type="number" step="1" placeholder="Aplicar às variantes ativas" onChange={event => {
                    if (event.target.value) setRows(current => current.map(row => row.retired ? row : { ...row, available_quantity: Number(event.target.value) }))
                  }} /></label>
                </div>
              </details>
              <label className="variant-checkbox variant-reference-toggle"><input type="checkbox" checked={showReferences} onChange={event => setShowReferences(event.target.checked)} />Mostrar referências</label>
              <label className="variant-checkbox variant-reference-toggle"><input type="checkbox" checked={showRetired} onChange={event => setShowRetired(event.target.checked)} />Mostrar variantes desativadas</label>
            </details>
          </>}
        </section>
      </fieldset>
        <div className="variant-workspace-feedback">
          {active > 100 && <p className="variant-message error" role="alert">Escolhe até 100 combinações ativas. Desativa as combinações que não vendes.</p>}
          {error && <p className="variant-message error" role="alert">{error}</p>}
          {message && <p className="variant-message success" role="status">{message}</p>}
        </div>
      <section id="variant-shop-panel" role="tabpanel" aria-labelledby="variant-shop-tab" hidden={view !== 'shop'}><VariantStorePreview product={product} config={config} rows={rows} /></section>
        <footer className="variant-workspace-footer">
          <span className="variant-product-reference">Referência do produto <strong>{product.sku}</strong></span>
          <div className="variant-footer-buttons">{view === 'admin' && <button className="variant-button" type="button" onClick={() => setView('shop')}><Eye size={16} aria-hidden="true" />Ver na loja</button>}<button className="variant-button primary" type="button" disabled={disabled || busy || needsPreview || active > 100} onClick={() => void save()}><Save size={16} aria-hidden="true" />{busy ? 'A guardar…' : 'Guardar opções e variantes'}</button></div>
        </footer>
    </section>
  )
}
