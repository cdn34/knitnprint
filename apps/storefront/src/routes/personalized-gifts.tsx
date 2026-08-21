import { createFileRoute } from '@tanstack/react-router'
import { ArrowRight, Gift, Heart, Palette } from 'lucide-react'
import { ContentPage } from '../components/content-page'

export const Route = createFileRoute('/personalized-gifts')({
  head: () => ({
    meta: [
      { title: 'Personalized gifts — KnitPrint' },
      { name: 'description', content: 'Personalized KnitPrint gift inspiration.' },
    ],
  }),
  component: PersonalizedGiftsPage,
})

const inspirations = [
  { icon: Gift, title: 'A gift for a milestone', tone: 'mauve' },
  { icon: Palette, title: 'A piece in their colours', tone: 'sand' },
  { icon: Heart, title: 'A detail made personal', tone: 'clay' },
]

function PersonalizedGiftsPage() {
  return (
    <ContentPage
      eyebrow="Made especially for someone"
      title="Personal gifts, shaped around your idea."
      intro="This page will become a gallery of finished custom pieces, helping customers imagine what we can create together."
      className="gifts-page"
    >
      <section className="inspiration-grid" aria-label="Personalized gift inspiration placeholders">
        {inspirations.map(({ icon: Icon, title, tone }) => (
          <article key={title}>
            <div className={`inspiration-visual tone--${tone}`}>
              <Icon aria-hidden="true" />
              <span>Example image</span>
            </div>
            <p className="eyebrow">Inspiration</p>
            <h2>{title}</h2>
            <p>A personalized product and its story can be presented here later.</p>
          </article>
        ))}
      </section>
      <aside className="page-cta">
        <div>
          <p className="eyebrow">Start with the collection</p>
          <h2>Choose a piece to make your own.</h2>
        </div>
        <a className="button button--primary" href="/products">
          Browse all pieces <ArrowRight size={17} />
        </a>
      </aside>
    </ContentPage>
  )
}
