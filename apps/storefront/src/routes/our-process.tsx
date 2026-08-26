import { createFileRoute } from '@tanstack/react-router'
import {
  Lightbulb,
  PackageCheck,
  Palette,
  Printer,
} from 'lucide-react'
import { ContentPage } from '../components/content-page'
import { ContextualFaqs } from '../components/contextual-faqs'
import { useI18n } from '../i18n'

export const Route = createFileRoute('/our-process')({
  head: () => ({
    meta: [
      { title: 'Our process — KnitnPrint' },
      { name: 'description', content: 'Discover how a KnitnPrint idea becomes a personalised piece, made with care from first detail to final delivery.' },
    ],
  }),
  component: ProcessPage,
})

function ProcessPage() {
  const { t } = useI18n()
  const processSteps = [
    { icon: Lightbulb, title: t('process.step1Title'), copy: t('process.step1Body') },
    { icon: Palette, title: t('process.step2Title'), copy: t('process.step2Body') },
    { icon: Printer, title: t('process.step3Title').replace(' ', '\u00a0'), copy: t('process.step3Body') },
    { icon: PackageCheck, title: t('process.step4Title').replaceAll(' ', '\u00a0'), copy: t('process.step4Body') },
  ]
  const craftDetails = [
    { label: t('process.craft1Label'), title: t('process.craft1Title'), copy: t('process.craft1Body'), image: '/process-textiles.jpg', imageAlt: t('process.craft1Alt') },
    { label: t('process.craft2Label'), title: t('process.craft2Title'), copy: t('process.craft2Body'), image: '/process-everyday-objects.jpg', imageAlt: t('process.craft2Alt') },
    { label: t('process.craft3Label'), title: t('process.craft3Title'), copy: t('process.craft3Body'), image: '/process-finishing.jpg', imageAlt: t('process.craft3Alt') },
  ]
  const manifestoLines = [
    t('process.manifestoLine1'),
    t('process.manifestoLine2'),
    t('process.manifestoLine3'),
    t('process.manifestoLine4'),
    t('process.manifestoLine5'),
  ].filter(Boolean)

  return (
    <ContentPage
      eyebrow={t('process.eyebrow')}
      title={<><span>{t('process.title1')}</span>{' '}<span>{t('process.title2')}</span></>}
      intro={t('process.intro')}
      className="process-page"
    >
      <section className="process-manifesto" aria-labelledby="process-manifesto-title">
        <div>
          <h2 id="process-manifesto-title">
            {manifestoLines.map((line, index) => <span key={line}>{line}{index < manifestoLines.length - 1 && <br />}</span>)}
          </h2>
          <p className="eyebrow">{t('process.gift')}</p>
        </div>
      </section>

      <section className="process-steps" aria-label={t('process.stepsLabel')}>
        {processSteps.map(({ icon: Icon, title, copy }) => (
          <article key={title}>
            <Icon className="process-step-icon" aria-hidden="true" />
            <div className="process-step-heading">
              <h2>{title}</h2>
            </div>
            <p>{copy}</p>
          </article>
        ))}
      </section>

      <section className="page-cta" aria-labelledby="process-cta-title">
        <div>
          <p className="eyebrow">{t('process.ctaEyebrow')}</p>
          <h2 id="process-cta-title">{t('process.ctaTitle')}</h2>
        </div>
        <a className="button button--primary" href="/products">{t('process.ctaButton')}</a>
      </section>

      <section className="process-gallery" aria-label={t('process.galleryLabel')}>
        {craftDetails.map(({ label, title, copy, image, imageAlt }) => (
          <article className="process-craft-card" key={title}>
            <div className="process-craft-visual">
              <img src={image} alt={imageAlt} loading="lazy" />
            </div>
            <div className="process-craft-copy">
              <p className="eyebrow">{label}</p>
              <h2>{title}</h2>
              <p>{copy}</p>
            </div>
          </article>
        ))}
      </section>

      <ContextualFaqs
        id="process-faqs"
        eyebrow={t('process.faqEyebrow')}
        title={t('process.faqTitle')}
        items={[
          { question: t('process.faq1Question'), answer: t('process.faq1Answer') },
          { question: t('process.faq2Question'), answer: t('process.faq2Answer') },
          { question: t('process.faq3Question'), answer: t('process.faq3Answer') },
          { question: t('process.faq4Question'), answer: t('process.faq4Answer') },
          { question: t('process.faq5Question'), answer: t('process.faq5Answer') },
        ]}
        className="contextual-faqs--process"
      />

    </ContentPage>
  )
}
