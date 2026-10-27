import { createFileRoute } from '@tanstack/react-router'
import { useEffect, useRef, useState, type FormEvent } from 'react'
import { CircleCheck, Send } from 'lucide-react'
import { ApiError, createApiClient, type B2bReceipt } from '@knitnprint/api-client'
import { ContentPage } from '../components/content-page'
import { ContextualFaqs } from '../components/contextual-faqs'
import { useI18n } from '../i18n'

const api = createApiClient()

export const Route = createFileRoute('/b2b')({
  head: () => ({
    meta: [
      { title: 'B2B — KnitNPrint' },
      { name: 'description', content: 'Personalised clothing and corporate gifts for businesses, associations, teams and events.' },
    ],
  }),
  component: B2BPage,
})

function B2BPage() {
  const { t } = useI18n()
  const [submitting, setSubmitting] = useState(false)
  const [receipt, setReceipt] = useState<B2bReceipt | null>(null)
  const [error, setError] = useState<'error' | 'fileError' | 'rateLimited' | null>(null)
  const requestKey = useRef<string | null>(null)
  const confirmation = useRef<HTMLDivElement>(null)
  useEffect(() => {
    if (receipt) {
      confirmation.current?.focus({ preventScroll: true })
      confirmation.current?.scrollIntoView({ block: 'center' })
    }
  }, [receipt])

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    if (submitting) return
    const form = new FormData(event.currentTarget)
    const file = form.get('brandFile')
    if (!(file instanceof File) || file.size === 0 || file.size > 10 * 1024 * 1024) {
      setError('fileError')
      return
    }
    setSubmitting(true)
    setError(null)
    requestKey.current ??= crypto.randomUUID()
    try {
      const attachment = await new Promise<string>((resolve, reject) => {
        const reader = new FileReader()
        reader.onload = () => resolve(String(reader.result).split(',')[1])
        reader.onerror = () => reject(new Error('The file could not be read.'))
        reader.readAsDataURL(file)
      })
      const result = await api.submitB2bRequest({
        request_key: requestKey.current,
        company: String(form.get('company') ?? ''),
        contact_name: String(form.get('contactName') ?? ''),
        email: String(form.get('email') ?? ''),
        phone: String(form.get('phone') ?? ''),
        product_type: String(form.get('productType') ?? ''),
        quantity: Number(form.get('quantity')),
        notes: String(form.get('notes') ?? ''),
        attachment_name: file.name,
        attachment_base64: attachment,
      })
      setReceipt(result)
    } catch (error) {
      setError(error instanceof ApiError && error.status === 429 ? 'rateLimited'
        : error instanceof ApiError && error.body?.error.code === 'invalid_b2b_attachment' ? 'fileError' : 'error')
    } finally {
      setSubmitting(false)
    }
  }
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

      <section className="b2b-contact" aria-labelledby="b2b-contact-title">
        <div className="b2b-contact-intro">
          <p className="eyebrow">{t('b2b.createTogether')}</p>
          <h2 id="b2b-contact-title">{t('b2b.requestTitle')}</h2>
          <p>{t('b2b.requestIntro')}</p>
          <a href="mailto:geral@knitnprint.com">geral@knitnprint.com</a>
        </div>

        {receipt ? (
          <div className="b2b-form b2b-success" role="status" aria-live="polite" tabIndex={-1} ref={confirmation}>
            <CircleCheck size={32} aria-hidden="true" />
            <h3>{t('b2b.successTitle')}</h3>
            <p>{t('b2b.successBody')}</p>
            <p><strong>{t('b2b.reference')}: {receipt.reference}</strong></p>
          </div>
        ) : <form className="b2b-form" method="post" onSubmit={submit} onChange={() => { requestKey.current = null }} aria-busy={submitting}>
          <fieldset className="b2b-form-fields" disabled={submitting}>
          <legend className="sr-only">{t('b2b.requestTitle')}</legend>
          <div className="b2b-form-row">
            <label>{t('b2b.company')}<input name="company" type="text" autoComplete="organization" maxLength={200} required /></label>
            <label>{t('b2b.contactName')}<input name="contactName" type="text" autoComplete="name" maxLength={150} required /></label>
          </div>
          <div className="b2b-form-row">
            <label>{t('b2b.email')}<input name="email" type="email" autoComplete="email" maxLength={254} required /></label>
            <label>{t('b2b.phone')}<input name="phone" type="tel" autoComplete="tel" minLength={3} maxLength={40} required /></label>
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
          <label>{t('b2b.quantity')}<input name="quantity" type="number" min="1" max="1000000" inputMode="numeric" required /></label>
          <label className="b2b-file-field">
            {t('b2b.file')}
            <input name="brandFile" type="file" accept=".ai,.eps,.pdf,.svg,.png,.jpg,.jpeg" required />
            <span>{t('b2b.fileHint')}</span>
          </label>
          <label>
            {t('b2b.notes')} <span className="b2b-optional">{t('b2b.optional')}</span>
            <textarea name="notes" rows={4} maxLength={5000} placeholder={t('b2b.notesPlaceholder')} />
          </label>
          </fieldset>
          {error && <p className="b2b-form-error" role="alert">{t(`b2b.${error}`)}</p>}
          <button className="button button--primary" type="submit" disabled={submitting}>{t(submitting ? 'b2b.submitting' : 'b2b.requestButton')} <Send size={15} aria-hidden="true" /></button>
          <p className="b2b-form-note">{t('b2b.formNote')}</p>
        </form>}
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
    </ContentPage>
  )
}
