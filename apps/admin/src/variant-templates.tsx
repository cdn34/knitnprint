import { useState, type FormEvent } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { Plus, Shapes, Save, Trash2 } from 'lucide-react'
import {
  ApiError,
  createApiClient,
  type VariantTemplate,
} from '@knitnprint/api-client'
import {
  OptionFields,
  optionCode,
  controlName,
  templateKey,
} from './variant-fields'
const api = createApiClient()
export function VariantTemplatesManagement({
  canWrite,
}: {
  canWrite: boolean
}) {
  const client = useQueryClient()
  const templates = useQuery({
    queryKey: templateKey,
    queryFn: api.variantTemplates,
    refetchOnWindowFocus: false,
  })
  const [draft, setDraft] = useState<VariantTemplate | null>(null)
  const [original, setOriginal] = useState<VariantTemplate | null>(null)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  const [success, setSuccess] = useState('')
  const dirty = Boolean(
    draft && JSON.stringify(draft) !== JSON.stringify(original),
  )
  function open(template?: VariantTemplate) {
    if (dirty && !window.confirm('Discard your unsaved template changes?'))
      return
    setOriginal(template ? structuredClone(template) : null)
    setDraft(
      template
        ? structuredClone(template)
        : {
            id: crypto.randomUUID(),
            revision: 0,
            definition: { code: '', label: '', control: 'buttons', values: [] },
          },
    )
    setError('')
    setSuccess('')
  }
  async function save(e: FormEvent) {
    e.preventDefault()
    if (!draft) return
    if (!draft.definition.values.length) {
      setError('Add at least one value before saving.')
      return
    }
    setBusy(true)
    setError('')
    setSuccess('')
    try {
      const input = {
        ...draft,
        definition: {
          ...draft.definition,
          code:
            original?.definition.code ??
            optionCode(
              draft.definition.label,
              (templates.data ?? []).map((t) => t.definition.code),
            ),
        },
      }
      await api.saveVariantTemplate(input)
      const result = await templates.refetch()
      const saved = result.data?.find((t) => t.id === draft.id) ?? input
      setOriginal(structuredClone(saved))
      setDraft(structuredClone(saved))
      setSuccess('Template saved. Existing products keep their own options.')
    } catch (e) {
      setError(
        e instanceof ApiError
          ? e.body.error.message
          : 'The template could not be saved. Please try again.',
      )
    } finally {
      setBusy(false)
    }
  }
  async function remove(template: VariantTemplate) {
    if (
      !window.confirm(
        `Delete the ${template.definition.label} template? Existing products and their variants will keep their copied options.`,
      )
    )
      return
    setBusy(true)
    setError('')
    setSuccess('')
    try {
      await api.deleteVariantTemplate(template.id, {
        revision: template.revision,
      })
      await client.invalidateQueries({ queryKey: templateKey })
      setDraft(null)
      setOriginal(null)
      setSuccess('Template deleted. Existing products are unchanged.')
    } catch (e) {
      setError(
        e instanceof ApiError
          ? e.body.error.message
          : 'The template could not be deleted.',
      )
    } finally {
      setBusy(false)
    }
  }
  return (
    <section className="variant-ui" aria-labelledby="variant-templates-heading">
      <div className="section-heading">
        <div>
          <p>Catalog configuration</p>
          <h2 id="variant-templates-heading">Variant templates</h2>
        </div>
        {canWrite && (
          <button
            className="variant-button primary"
            disabled={busy}
            onClick={() => open()}
          >
            <Plus size={16} />
            New template
          </button>
        )}
      </div>
      <p className="variant-intro">
        Define reusable colors, sizes, materials, or other choices here.
        Products take their own copy, so template edits do not change items
        already on sale.
      </p>
      {error && (
        <p className="variant-message error" role="alert">
          {error}
        </p>
      )}
      {success && (
        <p className="variant-message success" role="status">
          {success}
        </p>
      )}
      <div className="template-workspace">
        <div className="variant-card template-list">
          {templates.isPending && (
            <p className="panel-message">Loading templates…</p>
          )}
          {templates.isError && (
            <p className="panel-error" role="alert">
              Templates could not be loaded.{' '}
              <button onClick={() => void templates.refetch()}>
                Try again
              </button>
            </p>
          )}
          {templates.data?.map((t) => (
            <button
              key={t.id}
              className={`template-list-row${draft?.id === t.id ? ' selected' : ''}`}
              disabled={busy}
              onClick={() => open(t)}
            >
              <Shapes size={19} />
              <span>
                <strong>{t.definition.label}</strong>
                <small>
                  {controlName(t.definition.control)} ·{' '}
                  {t.definition.values.length} values
                </small>
                <span className="template-preview">
                  {t.definition.values.slice(0, 6).map((v) => (
                    <span key={v.code}>
                      {t.definition.control === 'swatch' && (
                        <i style={{ background: v.color ?? '#000000' }} />
                      )}
                      {v.label}
                    </span>
                  ))}
                </span>
              </span>
            </button>
          ))}
          {templates.data?.length === 0 && (
            <p className="panel-message">
              No templates yet. Create a color, size, or another option to get
              started.
            </p>
          )}
        </div>
        {draft ? (
          <form className="variant-card template-editor" onSubmit={save}>
            <div className="panel-title">
              <Shapes size={18} />
              <div>
                <strong>
                  {original
                    ? `Edit ${original.definition.label}`
                    : 'New variant template'}
                </strong>
                <span>Choose the default values products start with.</span>
              </div>
            </div>
            <fieldset disabled={!canWrite || busy}>
              <OptionFields
                key={draft.id}
                option={draft.definition}
                onChange={(definition) => {
                  setDraft({ ...draft, definition })
                  setSuccess('')
                }}
              />
              {canWrite && (
                <div className="variant-actions">
                  <button className="variant-button primary" type="submit">
                    <Save size={16} />
                    {busy ? 'Saving…' : 'Save template'}
                  </button>
                  {original && (
                    <button
                      className="variant-button danger"
                      type="button"
                      onClick={() => void remove(original)}
                    >
                      <Trash2 size={16} />
                      Delete template
                    </button>
                  )}
                </div>
              )}
            </fieldset>
          </form>
        ) : (
          <div className="variant-card variant-empty">
            <Shapes size={28} />
            <h3>Your reusable options</h3>
            <p>
              Select a template to review its defaults, or create one for a new
              kind of product option.
            </p>
          </div>
        )}
      </div>
    </section>
  )
}
