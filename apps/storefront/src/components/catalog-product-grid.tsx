import type { Product } from '@knitprint/api-client'
import { Heart } from 'lucide-react'
import {
  mediaUrl,
  productStock,
} from '../catalog-api'
import { useI18n } from '../i18n'
import { useLocalizedCatalog } from '../i18n/catalog'

export function CatalogProductGrid({ products }: Readonly<{ products: Product[] }>) {
  const { t } = useI18n()
  const { priceForProduct, stockText } = useLocalizedCatalog()
  return (
    <div className="product-grid">
      {products.map((product, index) => {
        const stock = productStock(product)
        const localizedStock = stock ? stockText(stock) : null
        const tone = ['mauve', 'sand', 'ink', 'clay'][index % 4]
        const form = ['vase', 'planter', 'tray', 'lamp'][index % 4]

        return (
          <article className="product-card" key={product.id}>
            <div className={`product-image tone--${tone}`}>
              <a
                className="product-visual"
                href={`/products/${product.slug}`}
                aria-label={t('common.viewProduct', { name: product.title })}
              >
                {product.media[0] ? (
                  <img
                    className="catalog-product-photo"
                    src={mediaUrl(product.media[0].card_url)}
                    alt={product.media[0].alt_text}
                  />
                ) : (
                  <span className={`product-form product-form--${form}`} />
                )}
              </a>
              <button className="heart" aria-label={t('common.saveProduct', { name: product.title })} type="button">
                <Heart size={19} />
              </button>
            </div>
            <div className="product-info">
              <div>
                <h2><a href={`/products/${product.slug}`}>{product.title}</a></h2>
                {stock && (
                  <span className={`product-stock product-stock--${stock.state}`}>
                    {localizedStock?.label}
                  </span>
                )}
              </div>
              <p>{priceForProduct(product)}</p>
            </div>
          </article>
        )
      })}
    </div>
  )
}
