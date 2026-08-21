import { createFileRoute } from '@tanstack/react-router'
import type { Product } from '@knitprint/api-client'
import { useMemo, useState } from 'react'
import {
  ArrowRight,
  ChevronRight,
  CircleUserRound,
  Clock3,
  Heart,
  Menu,
  PackageCheck,
  Search,
  ShieldCheck,
  ShoppingBag,
  Sparkles,
} from 'lucide-react'
import {
  mediaUrl,
  productPrice,
  productStock,
  publishedCategories,
  publishedProducts,
} from '../catalog-api'

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

function IconButton({
  label,
  children,
}: Readonly<{ label: string; children: React.ReactNode }>) {
  return (
    <button className="icon-button" aria-label={label} type="button">
      {children}
    </button>
  )
}

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
      href: '#shop',
      count: 0,
      image: undefined,
      isPlaceholder: true,
    }))
    return [...published, ...placeholders].slice(0, 6)
  }, [categories, products])

  return (
    <>
      <div className="announcement">
        <Sparkles size={14} aria-hidden="true" />
        Small-batch objects, made slowly in Portugal
      </div>

      <header className="site-header">
        <a className="brand" href="/" aria-label="KnitPrint home">
          <img
            src="/knitprint-wordmark.webp"
            alt="KnitPrint"
            width="750"
            height="195"
          />
        </a>

        <nav className="desktop-nav" aria-label="Main navigation">
          <a href="#shop">Shop</a>
          <a href="#categories">Collections</a>
          <a href="#story">Our story</a>
        </nav>

        <div className="header-actions">
          <span className="desktop-action">
            <IconButton label="Search">
              <Search />
            </IconButton>
          </span>
          <a className="icon-button" aria-label="Account" href="/account">
            <CircleUserRound />
          </a>
          <a className="icon-button" aria-label="View cart" href="/cart">
            <ShoppingBag />
          </a>
          <span className="mobile-action">
            <IconButton label="Open menu">
              <Menu />
            </IconButton>
          </span>
        </div>
      </header>

      <main id="main-content" tabIndex={-1}>
        <section className="hero">
          <div className="hero-copy">
            <p className="eyebrow">Made between yarn and form</p>
            <h1>Soft ideas, shaped into lasting objects.</h1>
            <div className="hero-actions">
              <a className="button button--primary" href="#shop">
                Explore the collection <ArrowRight size={18} />
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
          <a className="browse-all-link" href="#shop">
            Browse all pieces <ChevronRight size={16} />
          </a>
        </section>

        <section className="story" id="story">
          <div className="story-mark" aria-hidden="true">
            <div className="yarn-ball" />
            <div className="story-thread" />
            <div className="printed-cube">KP</div>
          </div>
          <div className="story-copy">
            <p className="eyebrow">Two crafts, one point of view</p>
            <h2>Technology can still feel human.</h2>
            <p>
              We borrow the rhythm, softness, and patience of knitting, then
              build each object one precise layer at a time. The result is
              useful design with the warmth of something handmade.
            </p>
            <a className="text-link" href="/about">
              Meet KnitPrint <ArrowRight size={17} />
            </a>
          </div>
        </section>

        <section className="why-knitprint" aria-labelledby="why-knitprint-title">
          <div className="home-section-title home-section-title--centered">
            <p className="eyebrow">The KnitPrint difference</p>
            <h2 id="why-knitprint-title">Made slowly, chosen thoughtfully</h2>
          </div>
          <div className="benefit-grid">
            <article>
              <Clock3 aria-hidden="true" />
              <h3>Made in small batches</h3>
              <p>We make only what is needed, with time for every detail and less material waste.</p>
              <a href="/about">Our process <ArrowRight size={14} /></a>
            </article>
            <article>
              <ShieldCheck aria-hidden="true" />
              <h3>Secure from start to finish</h3>
              <p>Your payment details stay protected and every order is carefully prepared.</p>
              <a href="/terms">Shopping with us <ArrowRight size={14} /></a>
            </article>
            <article>
              <Sparkles aria-hidden="true" />
              <h3>Made to delight</h3>
              <p>Designed in Portugal and finished by hand, so every piece feels a little personal.</p>
              <a href="/personalized-gifts">Personalized gifts <ArrowRight size={14} /></a>
            </article>
          </div>
        </section>

        <section className="reassurance" aria-label="Shopping benefits">
          <div><PackageCheck /><span><strong>Carefully packed</strong><small>Prepared in our studio</small></span></div>
          <div><ShieldCheck /><span><strong>Secure checkout</strong><small>Your details stay protected</small></span></div>
          <div><Sparkles /><span><strong>Made in Portugal</strong><small>Designed and finished with care</small></span></div>
        </section>
      </main>

      <footer className="site-footer">
        <div className="footer-lead">
          <img
            src="/knitprint-wordmark.webp"
            alt="KnitPrint"
            width="750"
            height="195"
          />
          <p>Objects with the soul of craft and the precision of print.</p>
        </div>
        <div className="footer-links">
          <div>
            <h2>About us</h2>
            <a href="/about">About us</a>
            <a href="/discounts">Discount code</a>
            <a href="/personalized-gifts">Personalized gifts</a>
          </div>
          <div>
            <h2>Shop</h2>
            <a href="#shop">All pieces</a>
            <a href="#categories">Collections</a>
          </div>
          <div>
            <h2>Help</h2>
            <a href="/terms">Terms and conditions</a>
            <a href="/privacy">Privacy policy</a>
            <a href="/cookies">Cookies policy</a>
            <a href="/returns">Return policy</a>
            <a href="/complaints-book">Complaints book</a>
          </div>
          <div>
            <h2>Customer support and contacts</h2>
            <a href="mailto:hello@knitprint.local">Our email</a>
            <a href="/contact#phone">Phone number</a>
            <a href="/contact#social-media">Social media</a>
          </div>
        </div>
        <div className="footer-bottom">
          <span>© 2026 KnitPrint</span>
          <span>Made with care in Portugal</span>
          <a href="/privacy">Privacy</a>
          <a href="/terms">Terms</a>
        </div>
      </footer>
    </>
  )
}
