import { createFileRoute, notFound } from '@tanstack/react-router'
import { ArrowLeft, PackageCheck } from 'lucide-react'
import { ContextualFaqs } from '../components/contextual-faqs'
import { StorefrontAnnouncement, StorefrontFooter, StorefrontHeader } from '../components/storefront-shell'
import {
  mediaUrl,
  productPrice,
  productStock,
  publishedCollection,
} from '../catalog-api'

export const Route = createFileRoute('/collections/$slug')({
  loader: async ({ params }) => {
    const collection = await publishedCollection(params.slug)
    if (!collection.category) throw notFound()
    return collection
  },
  head: ({ loaderData }) => ({
    meta: [
      {
        title: loaderData
          ? `${loaderData.category?.name} collection — KnitPrint`
          : 'KnitPrint',
      },
      {
        name: 'description',
        content: loaderData?.category?.description ?? '',
      },
    ],
  }),
  component: CollectionPage,
})

function CollectionPage() {
  const { category, products } = Route.useLoaderData()
  if (!category) return null

  return (
    <>
      <StorefrontAnnouncement />
      <StorefrontHeader />
      <main className="collection-page" id="main-content" tabIndex={-1}>
        <a className="text-link page-back-link" href="/collections">
          <ArrowLeft size={17} /> All collections
        </a>
        <header className="collection-intro">
          <p className="eyebrow">KnitPrint collection</p>
          <h1>{category.name}</h1>
          {category.description && <p>{category.description}</p>}
        </header>
        <section aria-label={`${category.name} products`}>
          <div className="product-grid">
            {products.map((product, index) => {
              const stock = productStock(product)
              return <article className="product-card" key={product.id}>
                <div
                  className={`product-image tone--${['mauve', 'sand', 'ink', 'clay'][index % 4]}`}
                >
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
                      <span className="product-form product-form--planter" />
                    )}
                  </a>
                </div>
                <div className="product-info">
                  <div>
                    <h2>
                      <a href={`/products/${product.slug}`}>{product.title}</a>
                    </h2>
                    {stock && <span className={`product-stock product-stock--${stock.state}`}>{stock.label}</span>}
                  </div>
                  <p>{productPrice(product)}</p>
                </div>
              </article>
            })}
            {products.length === 0 && (
              <div className="storefront-empty">
                <PackageCheck aria-hidden="true" />
                <h2>This collection is taking shape.</h2>
                <p>Fresh pieces will appear here as soon as they leave the studio.</p>
              </div>
            )}
          </div>
        </section>
        <ContextualFaqs
          id="collection-faqs"
          eyebrow="About this collection"
          title="Ideas for making it personal"
          items={[
            { question: 'Which products can be personalised?', answer: 'Personalisation availability is shown on each product page, together with the options offered for that piece.' },
            { question: 'Can I use the same design on different products?', answer: 'Often, yes. We may adapt the scale or placement so the design works well with each product and material.' },
            { question: 'Can these pieces be ordered as a gift set?', answer: 'Contact us with the products you have in mind and we will let you know which combinations and presentation options are possible.' },
            { question: 'Are larger quantities available?', answer: 'For company, association or event quantities, visit our B2B page and request a tailored proposal.' },
          ]}
          className="contextual-faqs--collection"
        />
      </main>
      <StorefrontFooter />
    </>
  )
}
