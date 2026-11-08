import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Layers, Plus, Save, RefreshCw, Trash2 } from 'lucide-react'
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
  controlName,
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
  const combinations = config.options.length
    ? config.options.reduce((n, o) => n * o.values.length, 1)
    : 1
  const active = rows.filter((r) => !r.retired).length
  function changeOption(index: number, next: ProductOption) {
    setConfig((c) => ({
      ...c,
      options: c.options.map((o, i) => (i === index ? next : o)),
    }))
    setPools((p) => ({
      ...p,
      [next.code]: [
        ...next.values,
        ...(p[next.code] ?? []).filter(
          (v) => !next.values.some((n) => n.code === v.code),
        ),
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
          'Give every option and value a name before generating variants.',
        )
        return
      }
      if (
        new Set(option.values.map((v) => v.label.trim().toLowerCase())).size !==
        option.values.length
      ) {
        setError(
          `Use a different name for each ${option.label.toLowerCase()} value.`,
        )
        return
      }
    }
    if (
      new Set(config.options.map((o) => o.label.trim().toLowerCase())).size !==
      config.options.length
    ) {
      setError('Each product option needs a different name.')
      return
    }

    if (!combinations || combinations > 1000) {
      setError(
        'Choose at least one value per option, with no more than 1,000 combinations to preview.',
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
      `${generated.length} combinations generated. Review prices and stock, then save.`,
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
      setMessage('Options and variants saved.')
    } catch (e) {
      setError(
        e instanceof ApiError
          ? e.body.error.message
          : 'Variants could not be saved. Please try again.',
      )
    } finally {
      setBusy(false)
    }
  }
  const changeRow = (i: number, patch: Partial<Row>) =>
    setRows((rs) => rs.map((r, n) => (n === i ? { ...r, ...patch } : r)))
  return (
    <section
      className="variant-ui variant-matrix"
      aria-labelledby="product-variants-heading"
    >
      <div className="section-heading">
        <div>
          <p>Product options</p>
          <h2 id="product-variants-heading">Options and variants</h2>
        </div>
        <span>{active}/100 active combinations</span>
      </div>
      <p className="variant-intro">
        Combine choices such as color and size. For example, 2 colors × 3 sizes
        creates 6 variants, each with its own SKU, price, and stock.
      </p>
      {disabled && (
        <p className="variant-message">
          Save the product details above before changing its variants.
        </p>
      )}
      <fieldset className="variant-builder" disabled={disabled || busy}>
        <div className="variant-card">
          <div className="panel-title">
            <Layers size={18} />
            <div>
              <strong>1. Add product options</strong>
              <span>
                Add more than one template to combine options. Manage shared
                defaults in <a href="#variant-templates">Variant templates</a>.
              </span>
            </div>
          </div>
          {templates.isPending && (
            <p role="status" className="panel-message">
              Loading option templates…
            </p>
          )}
          {templates.isError && (
            <p role="alert" className="panel-error">
              Templates could not be loaded.{' '}
              <button type="button" onClick={() => void templates.refetch()}>
                Try again
              </button>
            </p>
          )}
          <div className="variant-template-choices">
            {templates.data?.map((t) => {
              const added = config.options.some(
                (o) =>
                  o.label.trim().toLowerCase() ===
                  t.definition.label.trim().toLowerCase(),
              )
              const colorBlocked =
                t.definition.control === 'swatch' &&
                config.options.some((o) => o.control === 'swatch')
              return (
                <button
                  className={`variant-template-choice${added ? ' added' : ''}`}
                  key={t.id}
                  aria-label={`Add ${t.definition.label} option`}
                  type="button"
                  disabled={added || colorBlocked || config.options.length >= 6}
                  onClick={() => addOption(t.definition)}
                >
                  <Plus size={17} />
                  <span>
                    <strong>{t.definition.label}</strong>
                    <small>
                      {added
                        ? 'Already added'
                        : colorBlocked
                          ? 'One color option per product'
                          : `${t.definition.values.length} values · ${controlName(t.definition.control)}`}
                    </small>
                  </span>
                </button>
              )
            })}
          </div>
          {templates.data?.length === 0 && (
            <p className="panel-message">
              Create your first template in{' '}
              <a href="#variant-templates">Variant templates</a>, then add it
              here.
            </p>
          )}
        </div>
        {config.options.length > 0 && (
          <div className="variant-card">
            <div className="panel-title">
              <Layers size={18} />
              <div>
                <strong>2. Choose values for this product</strong>
                <span>
                  Your selections are independent of the shared templates.
                </span>
              </div>
            </div>
            <div className="product-option-grid">
              {config.options.map((o, i) => (
                <section
                  className="product-option-card"
                  key={o.code}
                  aria-label={`${o.label} option`}
                >
                  <header>
                    <div>
                      <h3>{o.label}</h3>
                      <span>{o.values.length} values selected</span>
                    </div>
                    <button
                      type="button"
                      className="variant-icon-button danger"
                      aria-label={`Remove ${o.label} option`}
                      onClick={() => {
                        setConfig((c) => ({
                          ...c,
                          options: c.options.filter((_, n) => n !== i),
                        }))
                        setNeedsPreview(true)
                      }}
                    >
                      <Trash2 size={16} />
                    </button>
                  </header>
                  <div className="variant-value-choices">
                    {(pools[o.code] ?? o.values).map((v) => (
                      <label className="variant-value-choice" key={v.code}>
                        <input
                          type="checkbox"
                          aria-label={`${o.label}: ${v.label}`}
                          checked={o.values.some((n) => n.code === v.code)}
                          onChange={(e) => {
                            setConfig((c) => ({
                              ...c,
                              options: c.options.map((axis, n) =>
                                n === i
                                  ? {
                                      ...axis,
                                      values: e.target.checked
                                        ? [...axis.values, v]
                                        : axis.values.filter(
                                            (n) => n.code !== v.code,
                                          ),
                                    }
                                  : axis,
                              ),
                            }))
                            setNeedsPreview(true)
                            setMessage('')
                          }}
                        />
                        {o.control === 'swatch' && (
                          <i style={{ background: v.color ?? '#000000' }} />
                        )}
                        {v.label}
                      </label>
                    ))}
                  </div>
                  <details className="variant-customize">
                    <summary>
                      Customize {o.label.toLowerCase()} for this product
                    </summary>
                    <OptionFields
                      option={o}
                      allowSwatch={
                        !config.options.some(
                          (axis) =>
                            axis.code !== o.code && axis.control === 'swatch',
                        )
                      }
                      onChange={(next) => changeOption(i, next)}
                    />
                  </details>
                </section>
              ))}
            </div>
            {config.options.some((o) => o.control === 'swatch') && (
              <>
                <label className="variant-checkbox">
                  <input
                    type="checkbox"
                    checked={config.color_photos}
                    onChange={(e) =>
                      setConfig((c) => ({
                        ...c,
                        color_photos: e.target.checked,
                      }))
                    }
                  />
                  Use color-specific photos (optional)
                </label>
                {config.color_photos && (
                  <div className="variant-photo-grid">
                    {config.options
                      .filter((o) => o.control === 'swatch')
                      .flatMap((o) =>
                        o.values.map((v) => (
                          <fieldset key={v.code}>
                            <legend>Photos for {v.label}</legend>
                            <p className="variant-help">
                              Leave empty to use the shared product gallery.
                            </p>
                            {product.media.map((m) => (
                              <label key={m.id}>
                                <input
                                  type="checkbox"
                                  aria-label={`${v.label}: ${m.alt_text}`}
                                  checked={v.media_ids?.includes(m.id) ?? false}
                                  onChange={(e) =>
                                    setConfig((c) => ({
                                      ...c,
                                      options: c.options.map((axis) =>
                                        axis.code === o.code
                                          ? {
                                              ...axis,
                                              values: axis.values.map(
                                                (value) =>
                                                  value.code === v.code
                                                    ? {
                                                        ...value,
                                                        media_ids: e.target
                                                          .checked
                                                          ? [
                                                              ...(value.media_ids ??
                                                                []),
                                                              m.id,
                                                            ]
                                                          : (
                                                              value.media_ids ??
                                                              []
                                                            ).filter(
                                                              (id) =>
                                                                id !== m.id,
                                                            ),
                                                      }
                                                    : value,
                                              ),
                                            }
                                          : axis,
                                      ),
                                    }))
                                  }
                                />
                                <img src={m.thumbnail_url} alt={m.alt_text} />
                              </label>
                            ))}
                            {!product.media.length && (
                              <p className="variant-help">
                                Add product photos first to assign them to
                                colors.
                              </p>
                            )}
                          </fieldset>
                        )),
                      )}
                  </div>
                )}
              </>
            )}
          </div>
        )}
        {config.options.length > 0 &&
          product.variants.some((v) => !v.selections && !v.retired) && (
            <details className="variant-card legacy-mapping">
              <summary>
                Match existing stock to these options (optional)
              </summary>
              <p className="variant-help">
                Choose the options represented by an existing SKU before
                generating combinations to keep its stock and history. Unmatched
                variants will be retired.
              </p>
              {rows.map(
                (r, i) =>
                  r.id &&
                  !product.variants.find((v) => v.id === r.id)?.selections && (
                    <div className="legacy-mapping-row" key={r.id}>
                      <strong>{r.sku}</strong>
                      {config.options.map((o) => (
                        <label key={o.code}>
                          {o.label}
                          <select
                            aria-label={`Map ${o.label} ${i + 1}`}
                            value={r.selections[o.code] ?? ''}
                            onChange={(e) => {
                              changeRow(i, {
                                selections: {
                                  ...r.selections,
                                  [o.code]: e.target.value,
                                },
                              })
                              setNeedsPreview(true)
                            }}
                          >
                            <option value="">Choose</option>
                            {o.values.map((v) => (
                              <option key={v.code} value={v.code}>
                                {v.label}
                              </option>
                            ))}
                          </select>
                        </label>
                      ))}
                    </div>
                  ),
              )}
            </details>
          )}
        <div className="variant-card">
          <div className="panel-title">
            <Layers size={18} />
            <div>
              <strong>3. Generate and review combinations</strong>
              <span>
                {config.options
                  .map(
                    (o) => `${o.values.length} ${o.label.toLowerCase()} values`,
                  )
                  .join(' × ') || 'No options: one default variant'}{' '}
                = {combinations} combinations
              </span>
            </div>
          </div>
          <div className="variant-actions">
            <button
              className="variant-button"
              type="button"
              disabled={!combinations || combinations > 1000}
              onClick={generate}
            >
              <RefreshCw size={16} />
              Generate {combinations} variants
            </button>
            <small className="variant-help">
              Exclude combinations you do not sell. Existing SKUs are never
              changed.
            </small>
          </div>
          {needsPreview &&
            product.variants.some(
              (v) =>
                !v.retired &&
                v.selections &&
                Object.keys(v.selections).sort().join() !==
                  config.options
                    .map((o) => o.code)
                    .sort()
                    .join(),
            ) && (
              <p className="variant-message">
                Changing the product options creates new variants. Previous
                combinations will be retired when you save; their SKUs and stock
                history stay attached to them. Enter stock for the new
                combinations before saving.
              </p>
            )}
          {needsPreview ? (
            <p className="variant-message" role="status">
              Options changed. Generate the combined variants to continue.
            </p>
          ) : (
            <>
              <div className="variant-bulk-fields">
                <label>
                  Bulk price (€)
                  <input
                    type="number"
                    min="0"
                    step=".01"
                    placeholder="Apply to active variants"
                    onChange={(e) => {
                      if (e.target.value)
                        setRows((rs) =>
                          rs.map((r) =>
                            r.retired
                              ? r
                              : {
                                  ...r,
                                  price_minor: Math.round(
                                    Number(e.target.value) * 100,
                                  ),
                                },
                          ),
                        )
                    }}
                  />
                </label>
                <label>
                  Bulk recorded stock
                  <input
                    type="number"
                    step="1"
                    placeholder="Apply to active variants"
                    onChange={(e) => {
                      if (e.target.value)
                        setRows((rs) =>
                          rs.map((r) =>
                            r.retired
                              ? r
                              : {
                                  ...r,
                                  available_quantity: Number(e.target.value),
                                },
                          ),
                        )
                    }}
                  />
                </label>
              </div>
              <div
                className="matrix-scroll"
                role="region"
                aria-label="Variant combinations"
                tabIndex={0}
              >
                <table>
                  <caption className="sr-only">
                    Variant combinations, prices and recorded stock
                  </caption>
                  <thead>
                    <tr>
                      <th>Sell</th>
                      {config.options.map((o) => (
                        <th key={o.code}>{o.label}</th>
                      ))}
                      <th>SKU</th>
                      <th>Price (€)</th>
                      <th>Recorded stock</th>
                    </tr>
                  </thead>
                  <tbody>
                    {rows.map((r, i) => (
                      <tr
                        className={r.retired ? 'retired' : ''}
                        key={r.id ?? identity(r.selections)}
                      >
                        <td>
                          <input
                            aria-label={`Sell combination ${i + 1}`}
                            type="checkbox"
                            checked={!r.retired}
                            onChange={(e) =>
                              changeRow(i, { retired: !e.target.checked })
                            }
                          />
                        </td>
                        {config.options.map((o) => (
                          <td key={o.code}>
                            {o.values.find(
                              (v) => v.code === r.selections[o.code],
                            )?.label ??
                              r.selections[o.code] ??
                              '—'}
                          </td>
                        ))}
                        <td>
                          <code>{r.sku ?? 'Generated on save'}</code>
                          {r.retired && <small>Excluded</small>}
                        </td>
                        <td>
                          <input
                            aria-label={`Price ${i + 1}`}
                            type="number"
                            min="0"
                            step=".01"
                            value={r.price_minor / 100}
                            onChange={(e) =>
                              changeRow(i, {
                                price_minor: Math.round(
                                  Number(e.target.value) * 100,
                                ),
                              })
                            }
                          />
                        </td>
                        <td>
                          <input
                            aria-label={`Stock ${i + 1}`}
                            type="number"
                            step="1"
                            value={r.available_quantity}
                            onChange={(e) =>
                              changeRow(i, {
                                available_quantity: Number(e.target.value),
                              })
                            }
                          />
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </>
          )}
        </div>
        {active > 100 && (
          <p className="variant-message error" role="alert">
            Choose up to 100 active combinations. Exclude the combinations you
            do not sell.
          </p>
        )}
        {error && (
          <p className="variant-message error" role="alert">
            {error}
          </p>
        )}
        {message && (
          <p className="variant-message success" role="status">
            {message}
          </p>
        )}
        <div className="variant-actions">
          <span className="variant-help">Product SKU: {product.sku}</span>
          <button
            className="variant-button primary"
            type="button"
            disabled={needsPreview || active > 100}
            onClick={() => void save()}
          >
            <Save size={16} />
            {busy ? 'Saving…' : 'Save options and variants'}
          </button>
        </div>
      </fieldset>
    </section>
  )
}
