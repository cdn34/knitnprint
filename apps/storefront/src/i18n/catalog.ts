import type { Product, Variant } from '@knitprint/api-client'
import { preferredVariant, type StockPresentation } from '../catalog-api'
import { useI18n } from '.'

export function useLocalizedCatalog() {
  const { formatCurrency, t } = useI18n()

  function priceForVariant(variant: Variant | null | undefined) {
    return variant
      ? formatCurrency(variant.price_minor, variant.currency)
      : t('common.priceUnavailable')
  }

  function priceForProduct(product: Product) {
    return priceForVariant(preferredVariant(product))
  }

  function stockText(stock: StockPresentation) {
    if (stock.state === 'sold-out') {
      return { label: t('stock.soldOut'), detail: t('stock.soldOutDetail') }
    }
    if (stock.state === 'low') {
      return {
        label: t('stock.low', { count: stock.quantity }),
        detail: t('stock.lowDetail'),
      }
    }
    return { label: t('stock.available'), detail: t('stock.availableDetail') }
  }

  return { priceForProduct, priceForVariant, stockText }
}
