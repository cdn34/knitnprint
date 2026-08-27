import { createFileRoute } from '@tanstack/react-router'
import { ArrowRight, PackageCheck } from 'lucide-react'
import { useMemo, useState } from 'react'
import { ContentPage } from '../components/content-page'
import { mediaUrl, publishedCategories, publishedProducts } from '../catalog-api'
import { useI18n } from '../i18n'

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
      { title: 'Collections — KnitnPrint' },
      { name: 'description', content: 'Browse KnitnPrint collections by category.' },
    ],
  }),
  component: CollectionsPage,
})

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
  const { t } = useI18n()
  const [activeFilter, setActiveFilter] = useState('all')
  const categoryPlaceholders = useMemo(() => [
    { id: 'textiles', name: t('collections.textilesName'), description: t('collections.textilesDescription'), group: 'textiles' },
    { id: 'accessories', name: t('collections.accessoriesName'), description: t('collections.accessoriesDescription'), group: 'accessories' },
    { id: 'home', name: t('collections.homeName'), description: t('collections.homeDescription'), group: 'home' },
    { id: 'gifts', name: t('collections.giftsName'), description: t('collections.giftsDescription'), group: 'gifts' },
  ], [t])
  const filterOptions = [
    { id: 'all', label: t('collections.filterAll') },
    { id: 'textiles', label: t('collections.filterTextiles') },
    { id: 'accessories', label: t('collections.filterAccessories') },
    { id: 'home', label: t('collections.filterHome') },
    { id: 'gifts', label: t('collections.filterGifts') },
  ]

  const cards = useMemo(() => {
    const published = categories.map((category) => {
      const categoryProducts = products.filter((product) =>
        product.categories.some(({ id }) => id === category.id),
      )
      return {
        id: category.id,
        name: category.name,
        description: category.description || t('collections.descriptionFallback'),
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
  }, [categories, categoryPlaceholders, products, t])

  const visibleCards = activeFilter === 'all'
    ? cards
    : cards.filter(({ group }) => group === activeFilter || group === 'all')

  return (
    <ContentPage
      eyebrow={t('collections.eyebrow')}
      title={t('collections.title')}
      intro={t('collections.intro')}
      className="collections-index-page"
    >
      <section className="collection-browser" aria-labelledby="collection-browser-title">
        <div className="filter-bar">
          <div>
            <p className="eyebrow">{t('collections.filterEyebrow')}</p>
            <h2 id="collection-browser-title">{t('collections.filterTitle')}</h2>
          </div>
          <div className="filter-chips" aria-label={t('collections.filtersLabel')}>
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
                  {category.count > 0
                    ? `${category.count} ${category.count === 1 ? t('collections.piece') : t('collections.pieces')}`
                    : t('collections.newCollection')}
                </span>
                <span className="category-copy">
                  <small>{category.placeholder ? t('collections.structureReady') : t('collections.collectionLabel')}</small>
                  <strong>{category.name}</strong>
                  <span>{category.description}</span>
                  <em>{t('collections.viewCollection')} <ArrowRight size={14} /></em>
                </span>
              </a>
            ))}
          </div>
        ) : (
          <div className="storefront-empty">
            <PackageCheck aria-hidden="true" />
            <h2>{t('collections.emptyTitle')}</h2>
            <p>{t('collections.emptyBody')}</p>
          </div>
        )}
      </section>
    </ContentPage>
  )
}
