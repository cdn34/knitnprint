import { createFileRoute, notFound } from '@tanstack/react-router'
import { ArrowLeft, PackageCheck } from 'lucide-react'
import { ContextualFaqs } from '../components/contextual-faqs'
import { CatalogProductGrid } from '../components/catalog-product-grid'
import { StorefrontAnnouncement, StorefrontFooter, StorefrontHeader } from '../components/storefront-shell'
import { publishedCollection } from '../catalog-api'
import { useI18n } from '../i18n'

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
          ? `${loaderData.category?.name} collection — KnitnPrint`
          : 'KnitnPrint',
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
  const { t } = useI18n()
  if (!category) return null

  return (
    <>
      <StorefrontAnnouncement />
      <StorefrontHeader />
      <main className="collection-page" id="main-content" tabIndex={-1}>
        <a className="text-link page-back-link" href="/collections">
          <ArrowLeft size={17} /> {t('collection.allCollections')}
        </a>
        <header className="collection-intro">
          <p className="eyebrow">{t('collections.collectionLabel')}</p>
          <h1>{category.name}</h1>
          {category.description && <p>{category.description}</p>}
        </header>
        <section aria-label={t('collection.productsLabel', { name: category.name })}>
          {products.length > 0 ? <CatalogProductGrid products={products} /> : (
            <div className="storefront-empty">
              <PackageCheck aria-hidden="true" />
              <h2>{t('collection.emptyTitle')}</h2>
              <p>{t('collection.emptyBody')}</p>
            </div>
          )}
        </section>
        <ContextualFaqs
          id="collection-faqs"
          eyebrow={t('collection.faqEyebrow')}
          title={t('collection.faqTitle')}
          items={[
            { question: t('collection.faq1Question'), answer: t('collection.faq1Answer') },
            { question: t('collection.faq2Question'), answer: t('collection.faq2Answer') },
            { question: t('collection.faq3Question'), answer: t('collection.faq3Answer') },
            { question: t('collection.faq4Question'), answer: t('collection.faq4Answer') },
          ]}
          className="contextual-faqs--collection"
        />
      </main>
      <StorefrontFooter />
    </>
  )
}
