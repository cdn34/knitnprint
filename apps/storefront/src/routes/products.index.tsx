import { createFileRoute } from '@tanstack/react-router'
import { PackageCheck, Search } from 'lucide-react'
import { useMemo, useState } from 'react'
import { CatalogProductGrid } from '../components/catalog-product-grid'
import { ContentPage } from '../components/content-page'
import { publishedProducts } from '../catalog-api'

export const Route = createFileRoute('/products/')({
  loader: () => publishedProducts(),
  head: () => ({
    meta: [
      { title: 'All products — KnitPrint' },
      { name: 'description', content: 'Browse every product currently available from KnitPrint.' },
    ],
  }),
  component: ProductsPage,
})

function ProductsPage() {
  const products = Route.useLoaderData()
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
      eyebrow="The complete catalog"
      title="All pieces"
      intro="Every product currently available in the KnitPrint online store, gathered into one simple catalog."
      className="all-products-page"
    >
      <section className="catalog-browser" aria-labelledby="catalog-browser-title">
        <div className="catalog-toolbar">
          <div>
            <p className="eyebrow">Current catalog</p>
            <h2 id="catalog-browser-title">
              {visibleProducts.length} {visibleProducts.length === 1 ? 'piece' : 'pieces'}
            </h2>
          </div>
          <label className="shop-search">
            <Search size={16} aria-hidden="true" />
            <span className="sr-only">Search all products</span>
            <input
              type="search"
              placeholder="Search all pieces"
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
            <h2>{query ? 'No pieces match that search.' : 'The catalog is being prepared.'}</h2>
            <p>{query ? 'Try another word or clear the search.' : 'Published products will appear here automatically.'}</p>
          </div>
        )}
      </section>
    </ContentPage>
  )
}
