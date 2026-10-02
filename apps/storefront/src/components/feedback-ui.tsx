import { useState, type FormEvent } from 'react'
import { Send, Star } from 'lucide-react'
import type { CreateProductFeedbackRequest, ProductFeedback } from '@knitnprint/api-client'
import { useI18n } from '../i18n'

export function FeedbackStars({ rating, label }: Readonly<{ rating: number; label: string }>) {
  return (
    <span className="feedback-stars" aria-label={label}>
      {[1, 2, 3, 4, 5].map((star) => (
        <Star key={star} aria-hidden="true" className={star <= Math.round(rating) ? 'filled' : ''} />
      ))}
    </span>
  )
}

export function FeedbackForm({
  submitFeedback,
  onSuccess,
  commentPlaceholder,
}: Readonly<{
  submitFeedback: (input: CreateProductFeedbackRequest) => Promise<unknown>
  onSuccess: () => void
  commentPlaceholder?: string
}>) {
  const { t } = useI18n()
  const [rating, setRating] = useState(0)
  const [hoveredRating, setHoveredRating] = useState(0)
  const [commentLength, setCommentLength] = useState(0)
  const [submitting, setSubmitting] = useState(false)
  const [error, setError] = useState(false)

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
      await submitFeedback({
        display_name: String(form.get('display_name') ?? ''),
        rating,
        comment: String(form.get('comment') ?? ''),
      })
      onSuccess()
    } catch {
      setError(true)
      setSubmitting(false)
    }
  }

  return (
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
        <div className="feedback-rating-picker" onMouseLeave={() => setHoveredRating(0)}>
          {[1, 2, 3, 4, 5].map((star) => (
            <button
              key={star}
              type="button"
              className={star <= (hoveredRating || rating) ? 'selected' : ''}
              aria-label={t(star === 1 ? 'feedback.chooseOneStar' : 'feedback.chooseStars', { count: star })}
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
        placeholder={commentPlaceholder ?? t('feedback.commentPlaceholder')}
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
  )
}

export function FeedbackCard({
  review,
  showSource = false,
}: Readonly<{ review: ProductFeedback; showSource?: boolean }>) {
  const { locale, t } = useI18n()
  return (
    <article>
      {showSource && (
        <div className="feedback-source">
          {review.product_slug && review.product_title ? (
            <a href={`/products/${review.product_slug}`}>
              {t('home.feedbackProductSource', { name: review.product_title })}
            </a>
          ) : (
            <span>{t('home.feedbackStoreSource')}</span>
          )}
        </div>
      )}
      <header>
        <div className="feedback-avatar" aria-hidden="true">
          {review.display_name.trim().charAt(0).toUpperCase()}
        </div>
        <div>
          <strong>{review.display_name}</strong>
          <time dateTime={review.created_at}>
            {new Intl.DateTimeFormat(locale, {
              day: 'numeric', month: 'long', year: 'numeric',
            }).format(new Date(review.created_at))}
          </time>
        </div>
        <FeedbackStars
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
  )
}
