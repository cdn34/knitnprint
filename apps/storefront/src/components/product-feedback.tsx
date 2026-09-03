import type { FormEvent } from 'react'
import type { ProductFeedbackSummary } from '@knitprint/api-client'
import { MessageCircleHeart, Send, Star } from 'lucide-react'
import { useState } from 'react'
import { submitProductFeedback } from '../catalog-api'
import { useI18n } from '../i18n'

type ProductFeedbackProps = {
  productSlug: string
  productTitle: string
  summary: ProductFeedbackSummary
}

function Stars({ rating, label }: Readonly<{ rating: number; label: string }>) {
  return (
    <span className="feedback-stars" aria-label={label}>
      {[1, 2, 3, 4, 5].map((star) => (
        <Star
          key={star}
          aria-hidden="true"
          className={star <= Math.round(rating) ? 'filled' : ''}
        />
      ))}
    </span>
  )
}

export function ProductFeedback({
  productSlug,
  productTitle,
  summary,
}: Readonly<ProductFeedbackProps>) {
  const { locale, t } = useI18n()
  const [rating, setRating] = useState(0)
  const [hoveredRating, setHoveredRating] = useState(0)
  const [commentLength, setCommentLength] = useState(0)
  const [submitting, setSubmitting] = useState(false)
  const [error, setError] = useState(false)
  const average = summary.average_rating ?? 0

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    if (rating === 0) {
      setError(true)
      return
    }
    const form = new FormData(event.currentTarget)
    setSubmitting(true)
    setError(false)
    try {
      await submitProductFeedback(productSlug, {
        display_name: String(form.get('display_name') ?? ''),
        rating,
        comment: String(form.get('comment') ?? ''),
      })
      window.location.assign(`/products/${productSlug}/feedback-thanks`)
    } catch {
      setError(true)
      setSubmitting(false)
    }
  }

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
              <Stars
                rating={average}
                label={t('feedback.ratingOutOfFive', { rating: average.toFixed(1) })}
              />
              <small>
                {t(
                  summary.total_reviews === 1
                    ? 'feedback.reviewCountSingle'
                    : 'feedback.reviewCount',
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

        <form className="feedback-form" onSubmit={submit}>
          <label htmlFor="feedback-name">{t('feedback.name')}</label>
          <input
            id="feedback-name"
            name="display_name"
            type="text"
            autoComplete="name"
            minLength={2}
            maxLength={100}
            placeholder={t('feedback.namePlaceholder')}
            required
          />

          <fieldset>
            <legend>{t('feedback.yourRating')}</legend>
            <div
              className="feedback-rating-picker"
              onMouseLeave={() => setHoveredRating(0)}
            >
              {[1, 2, 3, 4, 5].map((star) => (
                <button
                  key={star}
                  type="button"
                  className={star <= (hoveredRating || rating) ? 'selected' : ''}
                  aria-label={t(
                    star === 1 ? 'feedback.chooseOneStar' : 'feedback.chooseStars',
                    { count: star },
                  )}
                  aria-pressed={rating === star}
                  onMouseEnter={() => setHoveredRating(star)}
                  onFocus={() => setHoveredRating(star)}
                  onBlur={() => setHoveredRating(0)}
                  onClick={() => {
                    setRating(star)
                    setError(false)
                  }}
                >
                  <Star aria-hidden="true" />
                </button>
              ))}
            </div>
          </fieldset>

          <label htmlFor="feedback-comment">{t('feedback.comment')}</label>
          <textarea
            id="feedback-comment"
            name="comment"
            minLength={10}
            maxLength={1200}
            rows={6}
            placeholder={t('feedback.commentPlaceholder')}
            onChange={(event) => setCommentLength(event.currentTarget.value.length)}
            required
          />
          <small className="feedback-character-count">{commentLength} / 1200</small>

          {error && <p className="feedback-form-error" role="alert">{t('feedback.error')}</p>}
          <button className="button button--primary" type="submit" disabled={submitting}>
            {submitting ? t('feedback.submitting') : t('feedback.submit')}
            {!submitting && <Send size={16} aria-hidden="true" />}
          </button>
        </form>

        <div className="feedback-list" aria-label={t('feedback.publishedReviews')}>
          {summary.reviews.length === 0 ? (
            <div className="feedback-empty">
              <Star aria-hidden="true" />
              <strong>{t('feedback.emptyTitle')}</strong>
              <p>{t('feedback.emptyBody')}</p>
            </div>
          ) : (
            summary.reviews.map((review) => (
              <article key={review.id}>
                <header>
                  <div className="feedback-avatar" aria-hidden="true">
                    {review.display_name.trim().charAt(0).toUpperCase()}
                  </div>
                  <div>
                    <strong>{review.display_name}</strong>
                    <time dateTime={review.created_at}>
                      {new Intl.DateTimeFormat(locale, {
                        day: 'numeric',
                        month: 'long',
                        year: 'numeric',
                      }).format(new Date(review.created_at))}
                    </time>
                  </div>
                  <Stars
                    rating={review.rating}
                    label={t('feedback.ratingOutOfFive', { rating: review.rating })}
                  />
                </header>
                <p>{review.comment}</p>
                {review.store_reply && (
                  <div className="feedback-store-reply">
                    <span aria-hidden="true">KP</span>
                    <div>
                      <strong>{t('feedback.storeReply')}</strong>
                      <p>{review.store_reply}</p>
                    </div>
                  </div>
                )}
              </article>
            ))
          )}
        </div>
      </div>
    </section>
  )
}
