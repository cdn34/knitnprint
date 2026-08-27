import { createFileRoute } from '@tanstack/react-router'
import { Gift, Heart, Sparkles } from 'lucide-react'
import { ContentPage } from '../components/content-page'
import { useI18n } from '../i18n'

export const Route = createFileRoute('/about')({
  head: () => ({
    meta: [
      { title: 'Our story — KnitnPrint' },
      { name: 'description', content: 'Discover how KnitnPrint turns meaningful ideas into personalised products.' },
    ],
  }),
  component: AboutPage,
})

function AboutPage() {
  const { locale, t } = useI18n()
  const storyHighlights = [
    { icon: Heart, title: t('about.highlight2Title'), description: t('about.highlight2Body') },
    { icon: Gift, title: t('about.highlight3Title'), description: t('about.highlight3Body') },
  ]

  return (
    <ContentPage
      eyebrow={t('about.eyebrow')}
      title={t('about.title')}
      intro={t('about.intro')}
      className={`about-page${locale === 'pt' ? ' about-page--pt' : ''}`}
    >
      <section className="story-builder" aria-label={t('about.storyLabel')}>
        <div className="video-placeholder story-video">
          <video
            autoPlay
            controls
            loop
            muted
            playsInline
            preload="metadata"
            aria-label={t('about.videoLabel')}
          >
            <source src="/knitnprint-story.mp4" type="video/mp4" />
            {t('about.videoFallback')}
          </video>
        </div>
        <article className="story-description-placeholder">
          <Sparkles aria-hidden="true" />
          <p className="eyebrow">{t('about.personal')}</p>
          <h2>{t('about.begins')}</h2>
          <p>{t('about.beginsBody')}</p>
        </article>
      </section>

      <section className="page-feature-grid" aria-label={t('about.highlightsLabel')}>
        {storyHighlights.map(({ icon: Icon, title, description }) => (
          <article key={title}>
            <Icon aria-hidden="true" />
            <h2>{title}</h2>
            <p>{description}</p>
          </article>
        ))}
      </section>

      <section className="story-closing" aria-labelledby="story-closing-title">
        <p className="eyebrow">{t('about.createdAround')}</p>
        <h2 id="story-closing-title">
          {locale === 'pt'
            ? t('about.imagine').split('. ').map((line) => (
                <span key={line}>{line.endsWith('.') ? line : `${line}.`}</span>
              ))
            : t('about.imagine')}
        </h2>
        <div>
          <p>{t('about.closing1')}</p>
          <p>{t('about.closing2')}</p>
        </div>
      </section>
      <section className="page-cta about-process-cta" aria-labelledby="about-process-title">
        <div>
          <p className="eyebrow">{t('about.ctaEyebrow')}</p>
          <h2 id="about-process-title">{t('about.ctaTitle')}</h2>
        </div>
        <a className="button button--primary" href="/our-process">{t('about.ctaButton')}</a>
      </section>
    </ContentPage>
  )
}
