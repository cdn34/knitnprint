import type { Product } from '@knitprint/api-client'
import { Heart } from 'lucide-react'
import {
  mediaUrl,
  productPrice,
  productStock,
} from '../catalog-api'

export function CatalogProductGrid({ products }: Readonly<{ products: Product[] }>) {
  return (
    <div className="product-grid">
      {products.map((product, index) => {
        const stock = productStock(product)
        const tone = ['mauve', 'sand', 'ink', 'clay'][index % 4]
        const form = ['vase', 'planter', 'tray', 'lamp'][index % 4]

        return (
          <article className="product-card" key={product.id}>
            <div className={`product-image tone--${tone}`}>
              <a
                className="product-visual"
                href={`/products/${product.slug}`}
                aria-label={`View ${product.title}`}
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
              <button className="heart" aria-label={`Save ${product.title}`} type="button">
                <Heart size={19} />
              </button>
            </div>
            <div className="product-info">
              <div>
                <h2><a href={`/products/${product.slug}`}>{product.title}</a></h2>
                {stock && (
                  <span className={`product-stock product-stock--${stock.state}`}>
                    {stock.label}
                  </span>
                )}
              </div>
              <p>{productPrice(product)}</p>
            </div>
          </article>
        )
      })}
    </div>
  )
}
