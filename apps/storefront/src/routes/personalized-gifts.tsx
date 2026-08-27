import { createFileRoute } from '@tanstack/react-router'
import { ArrowRight, Gift, Heart, Palette } from 'lucide-react'
import { ContentPage } from '../components/content-page'
import { useI18n } from '../i18n'

export const Route = createFileRoute('/personalized-gifts')({
  head: () => ({
    meta: [
      { title: 'Personalized gifts — KnitnPrint' },
      { name: 'description', content: 'Personalized KnitnPrint gift inspiration.' },
    ],
  }),
  component: PersonalizedGiftsPage,
})

function PersonalizedGiftsPage() {
  const { t } = useI18n()
  const inspirations = [
    { icon: Gift, title: t('gifts.card1'), tone: 'mauve' },
    { icon: Palette, title: t('gifts.card2'), tone: 'sand' },
    { icon: Heart, title: t('gifts.card3'), tone: 'clay' },
  ]
  return (
    <ContentPage
      eyebrow={t('gifts.eyebrow')}
      title={t('gifts.title')}
      intro={t('gifts.intro')}
      className="gifts-page"
    >
      <section className="inspiration-grid" aria-label={t('gifts.label')}>
        {inspirations.map(({ icon: Icon, title, tone }) => (
          <article key={title}>
            <div className={`inspiration-visual tone--${tone}`}>
              <Icon aria-hidden="true" />
              <span>{t('gifts.example')}</span>
            </div>
            <p className="eyebrow">{t('gifts.inspiration')}</p>
            <h2>{title}</h2>
            <p>{t('gifts.cardBody')}</p>
          </article>
        ))}
      </section>
      <aside className="page-cta">
        <div>
          <p className="eyebrow">{t('gifts.ctaEyebrow')}</p>
          <h2>{t('gifts.ctaTitle')}</h2>
        </div>
        <a className="button button--primary" href="/products">
          {t('gifts.ctaButton')} <ArrowRight size={17} />
        </a>
      </aside>
    </ContentPage>
  )
}
