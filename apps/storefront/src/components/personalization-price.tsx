import { useI18n } from '../i18n'

export function PersonalizationPrice({ basePriceMinor, areaPriceMinor, areaCount, currency }: Readonly<{
  basePriceMinor: number
  areaPriceMinor: number
  areaCount: number
  currency: string
}>) {
  const { t, formatCurrency } = useI18n()
  const personalizationMinor = areaCount * areaPriceMinor

  return <div className="personalization-price-summary" aria-live="polite" aria-atomic="true">
    <dl>
      <div><dt>{t('personalization.basePrice')}</dt><dd>{formatCurrency(basePriceMinor, currency)}</dd></div>
      <div><dt>{t('personalization.personalizationPrice')}</dt><dd>+{formatCurrency(personalizationMinor, currency)}</dd></div>
      <div><dt>{t('personalization.unitTotal')}</dt><dd><strong>{formatCurrency(basePriceMinor + personalizationMinor, currency)}</strong></dd></div>
    </dl>
    <p>{t('personalization.areaPrice', { price: formatCurrency(areaPriceMinor, currency) })}</p>
  </div>
}
