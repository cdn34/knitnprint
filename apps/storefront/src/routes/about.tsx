import { createFileRoute } from '@tanstack/react-router'
import { Film, PenLine, Sparkles } from 'lucide-react'
import { ContentPage } from '../components/content-page'

export const Route = createFileRoute('/about')({
  head: () => ({
    meta: [
      { title: 'Our story — KnitPrint' },
      { name: 'description', content: 'Discover the story behind KnitPrint.' },
    ],
  }),
  component: AboutPage,
})

function AboutPage() {
  return (
    <ContentPage
      eyebrow="About KnitPrint"
      title="Our story is still being made."
      intro="This page is ready to hold the people, ideas, and moments that shaped KnitPrint."
      className="about-page"
    >
      <section className="story-builder" aria-label="Our story content placeholders">
        <div className="video-placeholder">
          <span><Film aria-hidden="true" /></span>
          <div>
            <p className="eyebrow">Video space</p>
            <h2>Your studio film will live here.</h2>
            <p>Prepared for a future brand video, workshop tour, or founder introduction.</p>
          </div>
        </div>
        <article className="story-description-placeholder">
          <PenLine aria-hidden="true" />
          <p className="eyebrow">Story draft</p>
          <h2>A place for the story behind the shop.</h2>
          <p>
            Add the origin of KnitPrint, what inspires the collections, and the
            people or values that make the studio feel personal.
          </p>
        </article>
      </section>

      <section className="page-feature-grid" aria-label="Future story highlights">
        {['Where it began', 'What we believe', 'Where we are going'].map((item, index) => (
          <article key={item}>
            <Sparkles aria-hidden="true" />
            <span>0{index + 1}</span>
            <h2>{item}</h2>
            <p>A short story or milestone can be added here later.</p>
          </article>
        ))}
      </section>
    </ContentPage>
  )
}
