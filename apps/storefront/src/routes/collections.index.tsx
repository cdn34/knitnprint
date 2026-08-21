import { createFileRoute } from '@tanstack/react-router'
import { ArrowRight, PackageCheck } from 'lucide-react'
import { useMemo, useState } from 'react'
import { ContentPage } from '../components/content-page'
import { mediaUrl, publishedCategories, publishedProducts } from '../catalog-api'

export const Route = createFileRoute('/collections/')({
  loader: async () => {
    const [categories, products] = await Promise.all([
      publishedCategories(),
      publishedProducts(),
    ])
    return { categories, products }
  },
  head: () => ({
    meta: [
      { title: 'Collections — KnitPrint' },
      { name: 'description', content: 'Browse KnitPrint collections by category.' },
    ],
  }),
  component: CollectionsPage,
})

const categoryPlaceholders = [
  { id: 'textiles', name: 'Textiles', description: 'Soft pieces ready for an idea of your own.', group: 'textiles' },
  { id: 'accessories', name: 'Accessories', description: 'Useful details with a personal point of view.', group: 'accessories' },
  { id: 'home', name: 'Home objects', description: 'Warm, practical pieces for everyday spaces.', group: 'home' },
  { id: 'gifts', name: 'Personalized gifts', description: 'Made especially for the people you love.', group: 'gifts' },
]

const filterOptions = [
  { id: 'all', label: 'All categories' },
  { id: 'textiles', label: 'Textiles' },
  { id: 'accessories', label: 'Accessories' },
  { id: 'home', label: 'Home' },
  { id: 'gifts', label: 'Gifts' },
]

function categoryGroup(name: string, description: string) {
  const value = `${name} ${description}`.toLowerCase()
  if (/(shirt|textile|fabric|apparel|sweat)/.test(value)) return 'textiles'
  if (/(bag|cap|accessor|bottle)/.test(value)) return 'accessories'
  if (/(home|desk|decor|object)/.test(value)) return 'home'
  if (/(gift|personal)/.test(value)) return 'gifts'
  return 'all'
}

function CollectionsPage() {
  const { categories, products } = Route.useLoaderData()
  const [activeFilter, setActiveFilter] = useState('all')

  const cards = useMemo(() => {
    const published = categories.map((category) => {
      const categoryProducts = products.filter((product) =>
        product.categories.some(({ id }) => id === category.id),
      )
      return {
        id: category.id,
        name: category.name,
        description: category.description || 'Discover the pieces in this collection.',
        href: `/collections/${category.slug}`,
        count: categoryProducts.length,
        image: categoryProducts[0]?.media[0]?.card_url,
        group: categoryGroup(category.name, category.description),
        placeholder: false,
      }
    })
    const existingGroups = new Set(published.map(({ group }) => group))
    const placeholders = categoryPlaceholders
      .filter(({ group }) => !existingGroups.has(group))
      .map((category) => ({
        ...category,
        href: category.group === 'gifts' ? '/personalized-gifts' : '/products',
        count: 0,
        image: undefined,
        placeholder: true,
      }))
    return [...published, ...placeholders]
  }, [categories, products])

  const visibleCards = activeFilter === 'all'
    ? cards
    : cards.filter(({ group }) => group === activeFilter || group === 'all')

  return (
    <ContentPage
      eyebrow="Explore by category"
      title="Collections, gathered in one place."
      intro="Browse the current KnitPrint categories and use the first version of our filters to narrow the view."
      className="collections-index-page"
    >
      <section className="collection-browser" aria-labelledby="collection-browser-title">
        <div className="filter-bar">
          <div>
            <p className="eyebrow">Filter the view</p>
            <h2 id="collection-browser-title">Find your kind of piece</h2>
          </div>
          <div className="filter-chips" aria-label="Collection filters">
            {filterOptions.map((filter) => (
              <button
                className={activeFilter === filter.id ? 'is-active' : ''}
                type="button"
                aria-pressed={activeFilter === filter.id}
                onClick={() => setActiveFilter(filter.id)}
                key={filter.id}
              >
                {filter.label}
              </button>
            ))}
          </div>
        </div>

        {visibleCards.length > 0 ? (
          <div className="category-grid collection-page-grid">
            {visibleCards.map((category, index) => (
              <a
                className={`category-card category-card--${(index % 6) + 1}`}
                href={category.href}
                key={category.id}
              >
                {category.image ? (
                  <img className="category-card-photo" src={mediaUrl(category.image)} alt="" />
                ) : (
                  <span className="category-card-art" aria-hidden="true"><span /></span>
                )}
                <span className="category-card-shade" />
                <span className="category-count">
                  {category.count > 0 ? `${category.count} pieces` : 'New collection'}
                </span>
                <span className="category-copy">
                  <small>{category.placeholder ? 'Structure ready' : 'KnitPrint collection'}</small>
                  <strong>{category.name}</strong>
                  <span>{category.description}</span>
                  <em>View collection <ArrowRight size={14} /></em>
                </span>
              </a>
            ))}
          </div>
        ) : (
          <div className="storefront-empty">
            <PackageCheck aria-hidden="true" />
            <h2>This filter is ready for future categories.</h2>
            <p>New collections will appear here as they are published.</p>
          </div>
        )}
      </section>
    </ContentPage>
  )
}
