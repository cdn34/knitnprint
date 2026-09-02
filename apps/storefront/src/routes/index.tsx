import { createFileRoute } from '@tanstack/react-router'
import type { Product } from '@knitprint/api-client'
import { useMemo, useState } from 'react'
import {
  ArrowRight,
  ChevronRight,
  Clock3,
  Heart,
  PackageCheck,
  Search,
  ShieldCheck,
  Sparkles,
} from 'lucide-react'
import {
  mediaUrl,
  productStock,
  publishedCategories,
  publishedProducts,
} from '../catalog-api'
import {
  StorefrontAnnouncement,
  StorefrontFooter,
  StorefrontHeader,
} from '../components/storefront-shell'
import { ContextualFaqs } from '../components/contextual-faqs'
import { useI18n } from '../i18n'
import { useLocalizedCatalog } from '../i18n/catalog'

export const Route = createFileRoute('/')({
  loader: async () => {
    const [products, categories] = await Promise.all([
      publishedProducts(),
      publishedCategories(),
    ])
    return { products, categories }
  },
  component: HomePage,
})

function ProductCard({
  product,
  index,
  badge,
}: Readonly<{ product: Product; index: number; badge?: string }>) {
  const { t } = useI18n()
  const { priceForProduct, stockText } = useLocalizedCatalog()
  const stock = productStock(product)
  const localizedStock = stock ? stockText(stock) : null
  const tone = ['mauve', 'sand', 'ink', 'clay'][index % 4]
  const form = ['vase', 'planter', 'tray', 'lamp'][index % 4]

  return (
    <article className="product-card">
      <div className={`product-image tone--${tone}`}>
        {badge && <span className="product-tag">{badge}</span>}
        <a
          className="product-visual"
          href={`/products/${product.slug}`}
          aria-label={t('home.viewProduct', { name: product.title })}
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
        <button
          className="heart"
          aria-label={t('home.saveProduct', { name: product.title })}
          type="button"
        >
          <Heart size={19} />
        </button>
      </div>
      <div className="product-info">
        <div>
          <h3>
            <a href={`/products/${product.slug}`}>{product.title}</a>
          </h3>
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
}

function PlaceholderProductCard({ index }: Readonly<{ index: number }>) {
  const { t } = useI18n()
  const tone = ['sand', 'mauve', 'clay', 'ink'][index % 4]
  const form = ['planter', 'lamp', 'vase', 'tray'][index % 4]

  return (
    <article className="product-card product-card--placeholder">
      <div className={`product-image tone--${tone}`}>
        <span className="product-tag">{t('home.comingSoon')}</span>
        <div className="product-visual" aria-hidden="true">
          <span className={`product-form product-form--${form}`} />
        </div>
      </div>
      <div className="product-info">
        <div>
          <h3>{t('home.newStudioPiece')}</h3>
          <span className="product-placeholder-note">{t('home.reservedProduct')}</span>
        </div>
      </div>
    </article>
  )
}

function ProductShelf({
  products,
  minimum,
  placeholderOffset = 0,
}: Readonly<{
  products: Product[]
  minimum: number
  placeholderOffset?: number
}>) {
  const { t } = useI18n()
  const placeholderCount = Math.max(0, minimum - products.length)

  return (
    <div className="product-grid">
      {products.map((product, index) => (
        <ProductCard
          product={product}
          index={index + placeholderOffset}
          badge={index === 0 ? t('home.freshlyPublished') : undefined}
          key={product.id}
        />
      ))}
      {Array.from({ length: placeholderCount }, (_, index) => (
        <PlaceholderProductCard
          index={index + products.length + placeholderOffset}
          key={`placeholder-${placeholderOffset}-${index}`}
        />
      ))}
    </div>
  )
}

function HomePage() {
  const { products, categories } = Route.useLoaderData()
  const { t } = useI18n()
  const [query, setQuery] = useState('')
  const categoryPlaceholders = useMemo(() => [
    { name: t('home.category1Name'), description: t('home.category1Description') },
    { name: t('home.category2Name'), description: t('home.category2Description') },
    { name: t('home.category3Name'), description: t('home.category3Description') },
    { name: t('home.category4Name'), description: t('home.category4Description') },
    { name: t('home.category5Name'), description: t('home.category5Description') },
  ], [t])
  const visibleProducts = useMemo(() => {
    const normalized = query.trim().toLowerCase()
    if (!normalized) return products
    return products.filter((product) =>
      [
        product.title,
        product.description,
        product.search_keywords,
        product.slug,
      ].some((value) => value.toLowerCase().includes(normalized)),
    )
  }, [products, query])

  const categoryCards = useMemo(() => {
    const published = categories.map((category) => {
      const categoryProducts = products.filter((product) =>
        product.categories.some(({ id }) => id === category.id),
      )
      return {
        id: category.id,
        name: category.name,
        description: category.description || t('home.collectionFallback'),
        href: `/collections/${category.slug}`,
        count: categoryProducts.length,
        image: categoryProducts[0]?.media[0]?.card_url,
        isPlaceholder: false,
      }
    })
    const missing = Math.max(0, 6 - published.length)
    const placeholders = categoryPlaceholders.slice(0, missing).map((category, index) => ({
      ...category,
      id: `category-placeholder-${index}`,
      href: '/collections',
      count: 0,
      image: undefined,
      isPlaceholder: true,
    }))
    return [...published, ...placeholders].slice(0, 6)
  }, [categories, categoryPlaceholders, products, t])

  return (
    <>
      <StorefrontAnnouncement />
      <StorefrontHeader />

      <main id="main-content" tabIndex={-1}>
        <section className="hero">
          <div className="hero-copy">
            <p className="eyebrow">{t('home.heroEyebrow')}</p>
            <h1>{t('home.heroTitle')}</h1>
            <div className="hero-actions">
              <a
                className="button button--primary"
                href="/collections"
                target="_blank"
                rel="noopener noreferrer"
              >
                {t('home.exploreCollection')}
                <span className="sr-only"> {t('home.opensNewTab')}</span>
                <ArrowRight size={18} aria-hidden="true" />
              </a>
            </div>
          </div>
        </section>

        <section className="category-showcase" id="categories">
          <div className="home-section-title home-section-title--centered home-section-title--categories">
            <p className="eyebrow">{t('home.categoriesEyebrow')}</p>
            <h2>{t('home.categoriesTitle')}</h2>
          </div>
          <div className="category-grid">
            {categoryCards.map((category, index) => (
              <a
                className={`category-card category-card--${(index % 6) + 1}${index === 0 && !category.isPlaceholder ? ' category-card--featured' : ''}`}
                href={category.href}
                key={category.id}
              >
                {category.image ? (
                  <img
                    className="category-card-photo"
                    src={mediaUrl(category.image)}
                    alt=""
                  />
                ) : (
                  <span className="category-card-art" aria-hidden="true">
                    <span />
                  </span>
                )}
                <span className="category-card-shade" />
                <span className="category-count">
                  {category.count > 0
                    ? `${category.count} ${category.count === 1 ? t('home.piece') : t('home.pieces')}`
                    : t('home.newCollection')}
                </span>
                <span className="category-copy">
                  <small>{category.isPlaceholder ? t('home.comingSoon') : index === 0 ? t('home.featuredCollection') : t('home.knitprintCollection')}</small>
                  <strong>{category.name}</strong>
                  <span>{category.description}</span>
                  <em>{t('home.viewCollection')} <ArrowRight size={14} /></em>
                </span>
              </a>
            ))}
          </div>
        </section>

        <section className="shop-section" id="shop">
          <div className="section-heading">
            <div>
              <h2>{t('home.recentlyAdded')}</h2>
            </div>
            <label className="shop-search">
              <Search size={16} aria-hidden="true" />
              <span className="sr-only">{t('home.searchCatalog')}</span>
              <input
                type="search"
                placeholder={t('home.searchPlaceholder')}
                value={query}
                onChange={(event) => setQuery(event.target.value)}
              />
            </label>
          </div>

          {visibleProducts.length > 0 || !query ? (
            <ProductShelf
              products={visibleProducts.slice(0, 8)}
              minimum={query ? 0 : 8}
            />
          ) : (
            <div className="storefront-empty">
              <PackageCheck aria-hidden="true" />
              <h3>{t('home.noSearchResults')}</h3>
              <p>{t('home.searchAgain')}</p>
            </div>
          )}
        </section>

        <section className="bestsellers-section" aria-labelledby="bestsellers-title">
          <div className="home-section-title home-section-title--centered">
            <p className="eyebrow">{t('home.favouritesEyebrow')}</p>
            <h2 id="bestsellers-title">{t('home.favouritesTitle')}</h2>
            <p>{t('home.favouritesIntro')}</p>
          </div>
          <ProductShelf products={products.slice(0, 4)} minimum={4} placeholderOffset={4} />
          <a className="browse-all-link" href="/products">
            {t('home.browseAll')} <ChevronRight size={16} />
          </a>
        </section>

        <section className="story" id="story">
          <div className="story-mark" aria-hidden="true">
            <img src="/knitprint-yarn-story-v6.png" alt="" />
          </div>
          <div className="story-copy">
            <p className="eyebrow">{t('home.storyEyebrow')}</p>
            <h2>{t('home.storyTitle')}</h2>
            <p>{t('home.storyBody')}</p>
            <a className="text-link" href="/about">
              {t('home.storyLink')} <ArrowRight size={17} />
            </a>
          </div>
        </section>

        <section className="why-knitprint" aria-labelledby="why-knitprint-title">
          <div className="home-section-title home-section-title--centered">
            <p className="eyebrow">{t('home.differenceEyebrow')}</p>
            <h2 id="why-knitprint-title">{t('home.differenceTitle')}</h2>
          </div>
          <div className="benefit-grid">
            <article>
              <Clock3 aria-hidden="true" />
              <h3>{t('home.benefit1Title')}</h3>
              <p>{t('home.benefit1Body')}</p>
              <a href="/our-process">{t('home.benefit1Link')} <ArrowRight size={14} /></a>
            </article>
            <article>
              <ShieldCheck aria-hidden="true" />
              <h3>{t('home.benefit2Title')}</h3>
              <p>{t('home.benefit2Body')}</p>
              <a href="/personalized-gifts">{t('home.benefit2Link')} <ArrowRight size={14} /></a>
            </article>
            <article>
              <Sparkles aria-hidden="true" />
              <h3>{t('home.benefit3Title')}</h3>
              <p>{t('home.benefit3Body')}</p>
              <a href="/collections">{t('home.benefit3Link')} <ArrowRight size={14} /></a>
            </article>
          </div>
        </section>

        <ContextualFaqs
          id="home-faqs"
          eyebrow={t('home.faqEyebrow')}
          title={t('home.faqTitle')}
          items={[
            { question: t('home.faq1Question'), answer: t('home.faq1Answer') },
            { question: t('home.faq2Question'), answer: t('home.faq2Answer') },
            { question: t('home.faq3Question'), answer: t('home.faq3Answer') },
            { question: t('home.faq4Question'), answer: t('home.faq4Answer') },
          ]}
          className="contextual-faqs--home"
        />

        <section className="reassurance" aria-label={t('home.shoppingBenefits')}>
          <div><PackageCheck /><span><strong>{t('home.packedTitle')}</strong><small>{t('home.packedBody')}</small></span></div>
          <div><ShieldCheck /><span><strong>{t('home.checkoutTitle')}</strong><small>{t('home.checkoutBody')}</small></span></div>
          <div><Sparkles /><span><strong>{t('home.portugalTitle')}</strong><small>{t('home.portugalBody')}</small></span></div>
        </section>
      </main>

      <StorefrontFooter />
    </>
  )
}
