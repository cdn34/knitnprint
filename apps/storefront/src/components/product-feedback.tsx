import type { ProductFeedbackSummary } from '@knitnprint/api-client'
import { MessageCircleHeart, Star } from 'lucide-react'
import { submitProductFeedback } from '../catalog-api'
import { useI18n } from '../i18n'
import { FeedbackCard, FeedbackForm, FeedbackStars } from './feedback-ui'

type ProductFeedbackProps = {
  productSlug: string
  productTitle: string
  summary: ProductFeedbackSummary
}

export function ProductFeedback({
  productSlug,
  productTitle,
  summary,
}: Readonly<ProductFeedbackProps>) {
  const { t } = useI18n()
  const average = summary.average_rating ?? 0

  return (
    <section className="product-feedback" aria-labelledby="product-feedback-title">
      <div className="product-feedback-heading">
        <div>
          <p className="eyebrow">{t('feedback.eyebrow')}</p>
          <h2 id="product-feedback-title">{t('feedback.title')}</h2>
          <p>{t('feedback.intro', { name: productTitle })}</p>
        </div>
        {summary.total_reviews > 0 && (
          <div className="feedback-average">
            <strong>{average.toFixed(1)}</strong>
            <span>
              <FeedbackStars
                rating={average}
                label={t('feedback.ratingOutOfFive', { rating: average.toFixed(1) })}
              />
              <small>
                {t(
                  summary.total_reviews === 1 ? 'feedback.reviewCountSingle' : 'feedback.reviewCount',
                  { count: summary.total_reviews },
                )}
              </small>
            </span>
          </div>
        )}
      </div>

      <div className="product-feedback-layout">
        <div className="feedback-publication">
          <MessageCircleHeart aria-hidden="true" />
          <div>
            <strong>{t('feedback.shareTitle')}</strong>
            <p>{t('feedback.moderationNote')}</p>
          </div>
        </div>

        <FeedbackForm
          submitFeedback={(input) => submitProductFeedback(productSlug, input)}
          onSuccess={() => window.location.assign(`/products/${productSlug}/feedback-thanks`)}
        />

        <div className="feedback-list" aria-label={t('feedback.publishedReviews')}>
          {summary.reviews.length === 0 ? (
            <div className="feedback-empty">
              <Star aria-hidden="true" />
              <strong>{t('feedback.emptyTitle')}</strong>
              <p>{t('feedback.emptyBody')}</p>
            </div>
          ) : (
            summary.reviews.map((review) => <FeedbackCard key={review.id} review={review} />)
          )}
        </div>
      </div>
    </section>
  )
}
