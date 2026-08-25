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
  productPrice,
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

const categoryPlaceholders = [
  {
    name: 'Personalized gifts',
    description: 'Made especially for the people you love',
  },
  {
    name: 'Home objects',
    description: 'Warm details for everyday spaces',
  },
  {
    name: 'Desk companions',
    description: 'Useful pieces for thoughtful work',
  },
  {
    name: 'Limited editions',
    description: 'Small runs, made with extra care',
  },
  {
    name: 'Made to order',
    description: 'Your idea, shaped in our studio',
  },
]

function ProductCard({
  product,
  index,
  badge,
}: Readonly<{ product: Product; index: number; badge?: string }>) {
  const stock = productStock(product)
  const tone = ['mauve', 'sand', 'ink', 'clay'][index % 4]
  const form = ['vase', 'planter', 'tray', 'lamp'][index % 4]

  return (
    <article className="product-card">
      <div className={`product-image tone--${tone}`}>
        {badge && <span className="product-tag">{badge}</span>}
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
        <button
          className="heart"
          aria-label={`Save ${product.title}`}
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
              {stock.label}
            </span>
          )}
        </div>
        <p>{productPrice(product)}</p>
      </div>
    </article>
  )
}

function PlaceholderProductCard({ index }: Readonly<{ index: number }>) {
  const tone = ['sand', 'mauve', 'clay', 'ink'][index % 4]
  const form = ['planter', 'lamp', 'vase', 'tray'][index % 4]

  return (
    <article className="product-card product-card--placeholder">
      <div className={`product-image tone--${tone}`}>
        <span className="product-tag">Coming soon</span>
        <div className="product-visual" aria-hidden="true">
          <span className={`product-form product-form--${form}`} />
        </div>
      </div>
      <div className="product-info">
        <div>
          <h3>New studio piece</h3>
          <span className="product-placeholder-note">Reserved for your next product</span>
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
  const placeholderCount = Math.max(0, minimum - products.length)

  return (
    <div className="product-grid">
      {products.map((product, index) => (
        <ProductCard
          product={product}
          index={index + placeholderOffset}
          badge={index === 0 ? 'Freshly published' : undefined}
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
  const [query, setQuery] = useState('')
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
        description: category.description || 'Discover the pieces in this collection',
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
  }, [categories, products])

  return (
    <>
      <StorefrontAnnouncement />
      <StorefrontHeader />

      <main id="main-content" tabIndex={-1}>
        <section className="hero">
          <div className="hero-copy">
            <p className="eyebrow">Made between yarn and form</p>
            <h1>Soft ideas, shaped into lasting objects.</h1>
            <div className="hero-actions">
              <a
                className="button button--primary"
                href="/collections"
                target="_blank"
                rel="noopener noreferrer"
              >
                Explore the collection
                <span className="sr-only"> (opens in a new tab)</span>
                <ArrowRight size={18} aria-hidden="true" />
              </a>
            </div>
          </div>
        </section>

        <section className="category-showcase" id="categories">
          <div className="home-section-title home-section-title--centered">
            <p className="eyebrow">Explore by category</p>
            <h2>Find your kind of piece</h2>
          </div>
          <div className="category-grid">
            {categoryCards.map((category, index) => (
              <a
                className={`category-card category-card--${(index % 6) + 1}`}
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
                    ? `${category.count} ${category.count === 1 ? 'piece' : 'pieces'}`
                    : 'New collection'}
                </span>
                <span className="category-copy">
                  <small>{category.isPlaceholder ? 'Coming soon' : 'KnitPrint collection'}</small>
                  <strong>{category.name}</strong>
                  <span>{category.description}</span>
                  <em>View collection <ArrowRight size={14} /></em>
                </span>
              </a>
            ))}
          </div>
        </section>

        <section className="shop-section" id="shop">
          <div className="section-heading">
            <div>
              <p className="eyebrow">Fresh from the studio</p>
              <h2>Recently added</h2>
              <p className="section-subtitle">Small-batch pieces, designed and finished with care.</p>
            </div>
            <label className="shop-search">
              <Search size={16} aria-hidden="true" />
              <span className="sr-only">Search the catalog</span>
              <input
                type="search"
                placeholder="Search the collection"
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
              <h3>No pieces match that search.</h3>
              <p>Try another word or clear the search.</p>
            </div>
          )}
        </section>

        <section className="bestsellers-section" aria-labelledby="bestsellers-title">
          <div className="home-section-title home-section-title--centered">
            <p className="eyebrow">Customer favourites</p>
            <h2 id="bestsellers-title">Our most loved pieces</h2>
            <p>Everyday objects that bring softness, character, and a little joy.</p>
          </div>
          <ProductShelf products={products.slice(0, 4)} minimum={4} placeholderOffset={4} />
          <a className="browse-all-link" href="/products">
            Browse all pieces <ChevronRight size={16} />
          </a>
        </section>

        <section className="story" id="story">
          <div className="story-mark" aria-hidden="true">
            <img src="/knitprint-yarn-story-v6.png" alt="" />
          </div>
          <div className="story-copy">
            <p className="eyebrow">Two crafts, one point of view</p>
            <h2>Every idea tells a story.</h2>
            <p>
              Behind every creation is an idea, a story, and a great deal of care.
              Discover who we are and how we turn special moments into unique products.
              Get to know KnitnPrint better.
            </p>
            <a className="text-link" href="/about">
              Get to know KnitnPrint <ArrowRight size={17} />
            </a>
          </div>
        </section>

        <section className="why-knitprint" aria-labelledby="why-knitprint-title">
          <div className="home-section-title home-section-title--centered">
            <p className="eyebrow">The KnitPrint difference</p>
            <h2 id="why-knitprint-title">Your idea, created with meaning</h2>
          </div>
          <div className="benefit-grid">
            <article>
              <Clock3 aria-hidden="true" />
              <h3>Dedication and commitment</h3>
              <p>We care for every order with attention, responsibility, and dedication, from your idea through to the final result.</p>
              <a href="/our-process">Discover our process <ArrowRight size={14} /></a>
            </article>
            <article>
              <ShieldCheck aria-hidden="true" />
              <h3>Personalization made for you</h3>
              <p>Every creation begins with you. We turn your ideas into unique products, designed around your taste and every occasion.</p>
              <a href="/personalized-gifts">Discover how to personalize <ArrowRight size={14} /></a>
            </article>
            <article>
              <Sparkles aria-hidden="true" />
              <h3>Made to surprise</h3>
              <p>Created in Portugal and finished with care, so every piece feels special and brings a smile.</p>
              <a href="/collections">Find the perfect gift <ArrowRight size={14} /></a>
            </article>
          </div>
        </section>

        <ContextualFaqs
          id="home-faqs"
          eyebrow="Good to know"
          title="A few things you may be wondering"
          items={[
            { question: 'What can I personalise?', answer: 'Selected textiles, bottles, backpacks, accessories and gifts can be made personal. Each product page shows the options available for that piece.' },
            { question: 'How long does a personalised order take?', answer: 'Timing depends on the product and the personalisation involved. Production takes place before the shipping estimate begins.' },
            { question: 'Will I see my design before production?', answer: 'When a digital mock-up is included, we will ask you to confirm the layout, scale and placement before production.' },
            { question: 'Can personalised products be returned?', answer: 'Personalised products generally cannot be returned for a change of mind, but your rights still apply if an item is faulty, damaged or not as agreed.' },
          ]}
          className="contextual-faqs--home"
        />

        <section className="reassurance" aria-label="Shopping benefits">
          <div><PackageCheck /><span><strong>Carefully packed</strong><small>Prepared in our studio</small></span></div>
          <div><ShieldCheck /><span><strong>Secure checkout</strong><small>Your details stay protected</small></span></div>
          <div><Sparkles /><span><strong>Made in Portugal</strong><small>Designed and finished with care</small></span></div>
        </section>
      </main>

      <StorefrontFooter />
    </>
  )
}
