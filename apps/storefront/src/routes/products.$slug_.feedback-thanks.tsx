import { createFileRoute, notFound } from '@tanstack/react-router'
import { ArrowLeft, CircleCheck, ShoppingBag } from 'lucide-react'
import { StorefrontAnnouncement, StorefrontFooter, StorefrontHeader } from '../components/storefront-shell'
import { publishedProduct } from '../catalog-api'
import { useI18n } from '../i18n'

export const Route = createFileRoute('/products/$slug_/feedback-thanks')({
  loader: async ({ params }) => {
    const product = await publishedProduct(params.slug)
    if (!product) throw notFound()
    return product
  },
  head: () => ({
    meta: [
      { title: 'Thank you for your feedback — KnitnPrint' },
      { name: 'robots', content: 'noindex' },
    ],
  }),
  component: FeedbackThanksPage,
})

function FeedbackThanksPage() {
  const product = Route.useLoaderData()
  const { t } = useI18n()

  return (
    <>
      <StorefrontAnnouncement />
      <StorefrontHeader />
      <main className="feedback-thanks-page" id="main-content" tabIndex={-1}>
        <section>
          <span className="feedback-thanks-icon"><CircleCheck aria-hidden="true" /></span>
          <p className="eyebrow">{t('feedbackThanks.eyebrow')}</p>
          <h1>{t('feedbackThanks.title')}</h1>
          <p>{t('feedbackThanks.body')}</p>
          <div>
            <a className="button button--primary" href={`/products/${product.slug}`}>
              <ArrowLeft size={16} aria-hidden="true" /> {t('feedbackThanks.backToProduct')}
            </a>
            <a className="button button--secondary" href="/#shop">
              {t('feedbackThanks.continueShopping')} <ShoppingBag size={16} aria-hidden="true" />
            </a>
          </div>
        </section>
      </main>
      <StorefrontFooter />
    </>
  )
}
