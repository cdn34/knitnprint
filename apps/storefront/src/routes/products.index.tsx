import { createFileRoute } from '@tanstack/react-router'
import { PackageCheck, Search } from 'lucide-react'
import { useMemo, useState } from 'react'
import { CatalogProductGrid } from '../components/catalog-product-grid'
import { ContentPage } from '../components/content-page'
import { publishedProducts } from '../catalog-api'
import { useI18n } from '../i18n'

export const Route = createFileRoute('/products/')({
  loader: () => publishedProducts(),
  head: () => ({
    meta: [
      { title: 'All products — KnitnPrint' },
      { name: 'description', content: 'Browse every product currently available from KnitnPrint.' },
    ],
  }),
  component: ProductsPage,
})

function ProductsPage() {
  const products = Route.useLoaderData()
  const { t } = useI18n()
  const [query, setQuery] = useState('')
  const visibleProducts = useMemo(() => {
    const normalized = query.trim().toLowerCase()
    if (!normalized) return products
    return products.filter((product) =>
      [product.title, product.description, product.search_keywords, product.slug]
        .some((value) => value.toLowerCase().includes(normalized)),
    )
  }, [products, query])

  return (
    <ContentPage
      eyebrow={t('catalog.eyebrow')}
      title={t('catalog.title')}
      intro={t('catalog.intro')}
      className="all-products-page"
    >
      <section className="catalog-browser" aria-labelledby="catalog-browser-title">
        <div className="catalog-toolbar">
          <div>
            <p className="eyebrow">{t('catalog.current')}</p>
            <h2 id="catalog-browser-title">
              {visibleProducts.length} {visibleProducts.length === 1 ? t('catalog.piece') : t('catalog.pieces')}
            </h2>
          </div>
          <label className="shop-search">
            <Search size={16} aria-hidden="true" />
            <span className="sr-only">{t('catalog.searchLabel')}</span>
            <input
              type="search"
              placeholder={t('catalog.searchPlaceholder')}
              value={query}
              onChange={(event) => setQuery(event.target.value)}
            />
          </label>
        </div>

        {visibleProducts.length > 0 ? (
          <CatalogProductGrid products={visibleProducts} />
        ) : (
          <div className="storefront-empty">
            <PackageCheck aria-hidden="true" />
            <h2>{query ? t('catalog.noResults') : t('catalog.preparing')}</h2>
            <p>{query ? t('catalog.tryAgain') : t('catalog.publishedSoon')}</p>
          </div>
        )}
      </section>
    </ContentPage>
  )
}
