import { createFileRoute } from '@tanstack/react-router'
import { useState } from 'react'
import { ArrowRight } from 'lucide-react'
import { ContentPage } from '../components/content-page'
import { useI18n } from '../i18n'

export const Route = createFileRoute('/discounts')({
  head: () => ({
    meta: [
      { title: '10% welcome discount — KnitnPrint' },
      {
        name: 'description',
        content: 'Join the KnitnPrint newsletter and receive 10% off your first eligible order.',
      },
    ],
  }),
  component: DiscountsPage,
})

function DiscountsPage() {
  const [submitted, setSubmitted] = useState(false)
  const { t } = useI18n()

  return (
    <ContentPage
      eyebrow={t('discount.eyebrow')}
      title={t('discount.title')}
      intro={t('discount.intro')}
      className="discount-page"
    >
      <section className="discount-signup" aria-labelledby="discount-signup-title">
        <div className="discount-offer" aria-hidden="true">
          <span>{t('discount.welcome')}</span>
          <strong>10%</strong>
          <span>{t('discount.offer')}</span>
        </div>

        <div className="discount-form-panel">
          <h2 id="discount-signup-title">{t('discount.formTitle')}</h2>
          <p>{t('discount.formIntro')}</p>

          {submitted ? (
            <div className="discount-success" role="status">
              <span>{t('discount.successEyebrow')}</span>
              <strong>{t('discount.successTitle')}</strong>
              <p>{t('discount.successBody')}</p>
            </div>
          ) : (
            <form
              className="discount-form"
              onSubmit={(event) => {
                event.preventDefault()
                setSubmitted(true)
              }}
            >
              <label htmlFor="discount-email">{t('discount.email')}</label>
              <div>
                <input
                  id="discount-email"
                  name="email"
                  type="email"
                  autoComplete="email"
                  placeholder={t('discount.emailPlaceholder')}
                  required
                />
                <button className="button button--primary" type="submit">
                  {t('discount.submit')} <ArrowRight size={16} aria-hidden="true" />
                </button>
              </div>
              <p>{t('discount.consent')}</p>
            </form>
          )}

          <aside className="discount-exclusion">
            <span aria-hidden="true">*</span>
            <p><strong>{t('discount.noteLabel')}</strong> {t('discount.b2bExclusion')}</p>
          </aside>
        </div>
      </section>
    </ContentPage>
  )
}
