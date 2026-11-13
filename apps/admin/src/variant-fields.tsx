import { useState } from 'react'
import { Plus, Trash2, ChevronUp } from 'lucide-react'
import type { ProductOption, OptionValue } from '@knitnprint/api-client'

export const templateKey = ['variant-templates'] as const
export function optionCode(label: string, used: string[] = []) {
  const base =
    label
      .normalize('NFKD')
      .replace(/[\u0300-\u036f]/g, '')
      .toUpperCase()
      .replace(/[^A-Z0-9]/g, '')
      .slice(0, 6) || 'OPTION'
  let candidate = base,
    n = 1
  while (used.includes(candidate)) candidate = `${base}${n++}`
  return candidate
}
export const controlName = (control: string) =>
  ({ swatch: 'Color swatches', buttons: 'Choice buttons', select: 'Dropdown' })[
    control
  ] ?? control

export function OptionFields({
  option,
  onChange,
  children,
  allowSwatch = true,
}: {
  option: ProductOption
  onChange: (next: ProductOption) => void
  children?: React.ReactNode
  allowSwatch?: boolean
}) {
  const [newValues, setNewValues] = useState('')
  function changeValue(index: number, patch: Partial<OptionValue>) {
    onChange({
      ...option,
      values: option.values.map((v, i) =>
        i === index ? { ...v, ...patch } : v,
      ),
    })
  }
  function addValues() {
    const values = [...option.values]
    for (const label of newValues
      .split(',')
      .map((v) => v.trim())
      .filter(Boolean)) {
      if (values.some((v) => v.label.toLowerCase() === label.toLowerCase()))
        continue
      values.push({
        code: optionCode(
          label,
          values.map((v) => v.code),
        ),
        label,
        color: '#000000',
        media_ids: [],
      })
    }
    onChange({ ...option, values })
    setNewValues('')
  }
  return (
    <div className="variant-fields">
      <div className="variant-form-grid">
        <label>
          Option name
          <input
            maxLength={80}
            required
            value={option.label}
            onChange={(e) => onChange({ ...option, label: e.target.value })}
          />
        </label>
        <label>
          Display style
          <select
            value={option.control}
            onChange={(e) =>
              onChange({
                ...option,
                control: e.target.value,
                values: option.values.map((v) => ({
                  ...v,
                  color: v.color ?? '#000000',
                })),
              })
            }
          >
            <option value="swatch" disabled={!allowSwatch}>
              Color swatches
            </option>
            <option value="buttons">Choice buttons</option>
            <option value="select">Dropdown</option>
          </select>
        </label>
      </div>
      <div
        className="variant-value-editor"
        aria-label={`${option.label} values`}
      >
        {option.values.map((v, i) => (
          <div className="variant-value-row" key={v.code}>
            {option.control === 'swatch' && (
              <input
                type="color"
                aria-label={`Color for ${v.label}`}
                value={v.color ?? '#000000'}
                onChange={(e) => changeValue(i, { color: e.target.value })}
              />
            )}
            <input
              aria-label={`Value ${i + 1}`}
              maxLength={80}
              required
              value={v.label}
              onChange={(e) => changeValue(i, { label: e.target.value })}
            />
            <button
              type="button"
              className="variant-icon-button"
              aria-label={`Move ${v.label} up`}
              disabled={!i}
              onClick={() => {
                const values = [...option.values]
                ;[values[i - 1], values[i]] = [values[i], values[i - 1]]
                onChange({ ...option, values })
              }}
            >
              <ChevronUp size={16} />
            </button>
            <button
              type="button"
              className="variant-icon-button danger"
              aria-label={`Remove ${v.label}`}
              onClick={() =>
                onChange({
                  ...option,
                  values: option.values.filter((_, n) => n !== i),
                })
              }
            >
              <Trash2 size={16} />
            </button>
          </div>
        ))}
      </div>
      <div className="variant-add-values">
        <label>
          Add values
          <input
            value={newValues}
            placeholder={
              option.control === 'swatch'
                ? 'e.g. Red, Green, Blue'
                : 'e.g. XS, S, M, L, XL'
            }
            onChange={(e) => setNewValues(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter') {
                e.preventDefault()
                addValues()
              }
            }}
          />
        </label>
        <button
          type="button"
          className="variant-button"
          disabled={!newValues.trim()}
          onClick={addValues}
        >
          <Plus size={16} />
          Add values
        </button>
      </div>
      <p className="variant-help">
        Separate values with commas to add several at once. Changes to display
        names never change saved SKUs.
      </p>
      {children}
    </div>
  )
}
