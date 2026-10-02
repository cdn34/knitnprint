import { useState } from 'react'
import type { ProductFeedbackSummary } from '@knitnprint/api-client'
import { CircleCheck, MessageCircleHeart, Star } from 'lucide-react'
import { submitSiteFeedback } from '../catalog-api'
import { useI18n } from '../i18n'
import { FeedbackCard, FeedbackForm, FeedbackStars } from './feedback-ui'

export function SiteFeedback({ summary }: Readonly<{ summary: ProductFeedbackSummary }>) {
  const { t } = useI18n()
  const [submitted, setSubmitted] = useState(false)
  const average = summary.average_rating ?? 0

  return (
    <section className="home-feedback" aria-labelledby="home-feedback-title">
      <div className="home-feedback-heading">
        <div>
          <p className="eyebrow">{t('home.feedbackEyebrow')}</p>
          <h2 id="home-feedback-title">{t('home.feedbackTitle')}</h2>
          <p>{t('home.feedbackIntro')}</p>
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

      <div className="home-feedback-layout">
        <div className="feedback-list" aria-label={t('feedback.publishedReviews')}>
          {summary.reviews.length === 0 ? (
            <div className="feedback-empty">
              <Star aria-hidden="true" />
              <strong>{t('home.feedbackEmptyTitle')}</strong>
              <p>{t('home.feedbackEmptyBody')}</p>
            </div>
          ) : (
            summary.reviews.map((review) => (
              <FeedbackCard key={review.id} review={review} showSource />
            ))
          )}
        </div>

        <div className="home-feedback-share">
          <div className="feedback-publication">
            <MessageCircleHeart aria-hidden="true" />
            <div>
              <strong>{t('home.feedbackShareTitle')}</strong>
              <p>{t('feedback.moderationNote')}</p>
            </div>
          </div>
          {submitted ? (
            <div className="home-feedback-success" role="status">
              <CircleCheck aria-hidden="true" />
              <strong>{t('feedbackThanks.title')}</strong>
              <p>{t('feedbackThanks.body')}</p>
            </div>
          ) : (
            <FeedbackForm
              submitFeedback={submitSiteFeedback}
              onSuccess={() => setSubmitted(true)}
              commentPlaceholder={t('home.feedbackCommentPlaceholder')}
            />
          )}
        </div>
      </div>
    </section>
  )
}
