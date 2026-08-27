import { createFileRoute } from '@tanstack/react-router'
import { Send } from 'lucide-react'
import { ContentPage } from '../components/content-page'
import { ContextualFaqs } from '../components/contextual-faqs'
import { useI18n } from '../i18n'

export const Route = createFileRoute('/b2b')({
  head: () => ({
    meta: [
      { title: 'B2B — KnitnPrint' },
      { name: 'description', content: 'Personalised clothing and corporate gifts for businesses, associations, teams and events.' },
    ],
  }),
  component: B2BPage,
})

function B2BPage() {
  const { t } = useI18n()
  const heroLines = [t('b2b.hero1'), t('b2b.hero2'), t('b2b.hero3'), t('b2b.hero4')].filter(Boolean)
  const benefits = [1, 2, 3].map((number) => ({
    title: t(`b2b.benefit${number}Title` as 'b2b.benefit1Title'),
    copy: t(`b2b.benefit${number}Body` as 'b2b.benefit1Body'),
  }))
  const steps = [1, 2, 3, 4].map((number) => ({
    number: String(number).padStart(2, '0'),
    title: t(`b2b.step${number}Title` as 'b2b.step1Title'),
    copy: t(`b2b.step${number}Body` as 'b2b.step1Body'),
  }))
  return (
    <ContentPage
      eyebrow="B2B*"
      title={t('b2b.title')}
      intro={
        <>
          <span>{t('b2b.intro')}</span>
          <span className="b2b-minimum-note">{t('b2b.minimum')}</span>
        </>
      }
      className="b2b-page"
    >
      <section className="b2b-hero" aria-labelledby="b2b-hero-title">
        <div>
          <p className="eyebrow">{t('b2b.heroEyebrow')}</p>
          <h2 id="b2b-hero-title">
            {heroLines.map((line) => <span key={line}>{line}</span>)}
          </h2>
        </div>
      </section>

      <section className="b2b-section" aria-labelledby="b2b-benefits-title">
        <div className="b2b-section-heading">
          <p className="eyebrow">{t('b2b.partner')}</p>
          <h2 id="b2b-benefits-title">{t('b2b.why')}</h2>
        </div>
        <div className="b2b-benefits">
          {benefits.map(({ title, copy }) => (
            <article key={title}>
              <h3>{title}</h3>
              <p>{copy}</p>
            </article>
          ))}
        </div>
      </section>

      <section className="b2b-section b2b-process" aria-labelledby="b2b-process-title">
        <div className="b2b-section-heading">
          <p className="eyebrow">{t('b2b.fromBrief')}</p>
          <h2 id="b2b-process-title">{t('b2b.how')}</h2>
        </div>
        <div className="b2b-steps">
          {steps.map(({ number, title, copy }) => (
            <article key={number}>
              <span>{number}</span>
              <h3>{title}</h3>
              <p>{copy}</p>
            </article>
          ))}
        </div>
      </section>

      <ContextualFaqs
        id="b2b-faqs"
        eyebrow={t('b2b.faqEyebrow')}
        title={t('b2b.faqTitle')}
        items={[
          { question: t('b2b.faq1Q'), answer: t('b2b.faq1A') },
          { question: t('b2b.faq2Q'), answer: t('b2b.faq2A') },
          { question: t('b2b.faq3Q'), answer: t('b2b.faq3A') },
          { question: t('b2b.faq4Q'), answer: t('b2b.faq4A') },
          { question: t('b2b.faq5Q'), answer: t('b2b.faq5A') },
        ]}
        className="contextual-faqs--b2b"
      />

      <section className="b2b-contact" aria-labelledby="b2b-contact-title">
        <div className="b2b-contact-intro">
          <p className="eyebrow">{t('b2b.createTogether')}</p>
          <h2 id="b2b-contact-title">{t('b2b.requestTitle')}</h2>
          <p>{t('b2b.requestIntro')}</p>
          <a href="mailto:support@knitnprint.com">support@knitnprint.com</a>
        </div>

        <form className="b2b-form">
          <div className="b2b-form-row">
            <label>{t('b2b.company')}<input name="company" type="text" autoComplete="organization" required /></label>
            <label>{t('b2b.contactName')}<input name="contactName" type="text" autoComplete="name" required /></label>
          </div>
          <div className="b2b-form-row">
            <label>{t('b2b.email')}<input name="email" type="email" autoComplete="email" required /></label>
            <label>{t('b2b.phone')}<input name="phone" type="tel" autoComplete="tel" required /></label>
          </div>
          <label>
            {t('b2b.productType')}
            <select name="productType" defaultValue="" required>
              <option value="" disabled>{t('b2b.select')}</option>
              <option value="clothing">{t('b2b.clothing')}</option>
              <option value="bottles">{t('b2b.bottles')}</option>
              <option value="backpacks">{t('b2b.backpacks')}</option>
              <option value="complete-kit">{t('b2b.completeKit')}</option>
              <option value="other">{t('b2b.other')}</option>
            </select>
          </label>
          <label>{t('b2b.quantity')}<input name="quantity" type="number" min="1" inputMode="numeric" required /></label>
          <label className="b2b-file-field">
            {t('b2b.file')}
            <input name="brandFile" type="file" accept=".ai,.eps,.pdf,.svg,.png,.jpg,.jpeg" required />
            <span>AI, EPS, PDF, SVG, PNG or JPG</span>
          </label>
          <label>
            {t('b2b.notes')} <span className="b2b-optional">{t('b2b.optional')}</span>
            <textarea name="notes" rows={4} placeholder={t('b2b.notesPlaceholder')} />
          </label>
          <button className="button button--primary" type="submit">{t('b2b.requestButton')} <Send size={15} aria-hidden="true" /></button>
          <p className="b2b-form-note">{t('b2b.formNote')}</p>
        </form>
      </section>
    </ContentPage>
  )
}
