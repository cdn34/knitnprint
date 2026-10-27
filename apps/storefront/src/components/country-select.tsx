import { useMemo, type SelectHTMLAttributes } from 'react'
import { countryOptions } from '../countries'
import { useI18n } from '../i18n'

export function CountrySelect({ defaultValue = 'PT', ...props }: Omit<SelectHTMLAttributes<HTMLSelectElement>, 'children' | 'defaultValue'> & { defaultValue?: string }) {
  const { locale, t } = useI18n()
  const options = useMemo(() => countryOptions(locale), [locale])
  const selected = defaultValue.trim().toUpperCase()
  return <select name="country_code" aria-label={t('account.countryCode')} autoComplete="country" required defaultValue={selected} {...props}>
    {selected && !options.some(option => option.code === selected) && <option value={selected}>{selected}</option>}
    {options.map(option => <option key={option.code} value={option.code}>{option.name}</option>)}
  </select>
}
