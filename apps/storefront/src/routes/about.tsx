import { createFileRoute } from '@tanstack/react-router'
import { Gift, Heart, Sparkles } from 'lucide-react'
import { ContentPage } from '../components/content-page'

const storyHighlights = [
  {
    icon: Sparkles,
    title: 'It starts with an idea',
    description: 'A special word, a meaningful drawing or a memory you want to carry with you.',
  },
  {
    icon: Heart,
    title: 'Made to mean more',
    description: 'Thoughtful details turn an everyday object into something that feels completely your own.',
  },
  {
    icon: Gift,
    title: 'Part of your story',
    description: 'Created for new adventures, treasured moments and the feelings that words cannot always express.',
  },
]

export const Route = createFileRoute('/about')({
  head: () => ({
    meta: [
      { title: 'Our story — KnitPrint' },
      { name: 'description', content: 'Discover how KnitnPrint turns meaningful ideas into personalised products.' },
    ],
  }),
  component: AboutPage,
})

function AboutPage() {
  return (
    <ContentPage
      eyebrow="About KnitPrint"
      title="More than products, we create stories."
      intro="Every creation begins with an idea and becomes something personal, meaningful and uniquely yours."
      className="about-page"
    >
      <section className="story-builder" aria-label="The KnitnPrint story">
        <div className="video-placeholder story-video">
          <video
            autoPlay
            controls
            loop
            muted
            playsInline
            preload="metadata"
            aria-label="KnitnPrint brand film"
          >
            <source src="/knitnprint-story.mp4" type="video/mp4" />
            Your browser does not support embedded videos.
          </video>
        </div>
        <article className="story-description-placeholder">
          <Sparkles aria-hidden="true" />
          <p className="eyebrow">Made personal</p>
          <h2>Everything begins with an idea.</h2>
          <p>
            A special word, a meaningful drawing or a memory we want to keep
            close. At KnitnPrint, we believe these small details can transform
            an ordinary object into something that is truly ours.
          </p>
        </article>
      </section>

      <section className="page-feature-grid" aria-label="What makes KnitnPrint personal">
        {storyHighlights.map(({ icon: Icon, title, description }, index) => (
          <article key={title}>
            <Icon aria-hidden="true" />
            <span>0{index + 1}</span>
            <h2>{title}</h2>
            <p>{description}</p>
          </article>
        ))}
      </section>

      <section className="story-closing" aria-labelledby="story-closing-title">
        <p className="eyebrow">Created around you</p>
        <h2 id="story-closing-title">You imagine. We create.</h2>
        <div>
          <p>
            KnitnPrint was born from the desire to bring your ideas to life and
            create products that speak for you. From textiles and bottles to
            backpacks and thoughtful gifts, every piece is personalised with
            care, creativity and close attention to detail.
          </p>
          <p>
            A T-shirt can bring back a moment. A bottle can join you on a new
            adventure. A personalised gift can say what words sometimes cannot.
            We do more than personalise products — we help you create something
            unique, made for you and designed to become part of your story.
          </p>
        </div>
      </section>
      <section className="page-cta about-process-cta" aria-labelledby="about-process-title">
        <div>
          <p className="eyebrow">From your idea to the final piece</p>
          <h2 id="about-process-title">Curious about how we make each piece?</h2>
        </div>
        <a className="button button--primary" href="/our-process">Discover our process</a>
      </section>
    </ContentPage>
  )
}
