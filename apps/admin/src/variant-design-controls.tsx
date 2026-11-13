import { useState } from 'react'
import { Plus } from 'lucide-react'
import type { ProductOption, OptionValue } from '@knitnprint/api-client'
import { optionCode } from './variant-fields'

export const isSizeOption = (option: ProductOption) =>
  /^SIZ/i.test(option.code) || /size|tamanho/i.test(option.label)

export function PrimaryVariantValues({ option, values, kind, onChange }: {
  option: ProductOption
  values: OptionValue[]
  kind: 'size' | 'color'
  onChange: (option: ProductOption) => void
}) {
  const [name, setName] = useState('')
  const [color, setColor] = useState('#b6a28b')
  function addValue() {
    const label = name.trim()
    if (!label) return
    const existing = values.find(value => value.label.toLocaleLowerCase() === label.toLocaleLowerCase())
    const value = existing ?? {
      code: optionCode(label, values.map(item => item.code)),
      label,
      color: kind === 'color' ? color : '#000000',
      media_ids: [],
    }
    if (!option.values.some(item => item.code === value.code)) onChange({ ...option, values: [...option.values, value] })
    setName('')
  }
  return <div className="variant-primary-values">
    <span className="variant-field-label">{kind === 'size' ? 'Tamanhos disponíveis' : 'Cores disponíveis'}</span>
    <div className="variant-value-choices">
      {values.map(value => <label className="variant-value-choice" key={value.code}>
        <input type="checkbox" aria-label={`${option.label}: ${value.label}`} checked={option.values.some(item => item.code === value.code)} onChange={event => onChange({
          ...option,
          values: event.target.checked ? [...option.values, value] : option.values.filter(item => item.code !== value.code),
        })} />
        {kind === 'color' && <i style={{ background: value.color ?? '#000000' }} aria-hidden="true" />}
        {value.label}
      </label>)}
    </div>
    <div className="variant-quick-add">
      <input aria-label={kind === 'size' ? 'Novo tamanho' : 'Nome da nova cor'} placeholder={kind === 'size' ? 'Ex.: 2 anos' : 'Outra cor'} maxLength={80} value={name} onChange={event => setName(event.target.value)} onKeyDown={event => {
        if (event.key === 'Enter') { event.preventDefault(); addValue() }
      }} />
      {kind === 'color' && <input type="color" aria-label="Amostra da nova cor" value={color} onChange={event => setColor(event.target.value)} />}
      <button type="button" className="variant-icon-button" aria-label={kind === 'size' ? 'Adicionar tamanho' : 'Adicionar cor'} onClick={addValue}><Plus size={18} aria-hidden="true" /></button>
    </div>
  </div>
}
