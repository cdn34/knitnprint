import { createFileRoute } from '@tanstack/react-router'
import {
  Lightbulb,
  PackageCheck,
  Palette,
  Printer,
} from 'lucide-react'
import { ContentPage } from '../components/content-page'
import { ContextualFaqs } from '../components/contextual-faqs'

export const Route = createFileRoute('/our-process')({
  head: () => ({
    meta: [
      { title: 'Our process — KnitPrint' },
      { name: 'description', content: 'Discover how a KnitnPrint idea becomes a personalised piece, made with care from first detail to final delivery.' },
    ],
  }),
  component: ProcessPage,
})

const processSteps = [
  {
    icon: Lightbulb,
    title: 'Your idea',
    copy: 'It begins with what matters to you: a name, a drawing, a message or a memory worth keeping close.',
  },
  {
    icon: Palette,
    title: 'The design',
    copy: 'We shape your idea into a composition that suits the product, balancing colour, scale and placement.',
  },
  {
    icon: Printer,
    title: 'Made\u00a0personal',
    copy: 'Your design is carefully applied using the technique best suited to the material and the desired finish.',
  },
  {
    icon: PackageCheck,
    title: 'Ready\u00a0for\u00a0you',
    copy: 'Every detail is checked before your piece is carefully packed and prepared for its journey to you.',
  },
]

const craftDetails = [
  {
    label: 'Textiles',
    title: 'Designed to belong',
    copy: 'T-shirts, sweatshirts and fabric pieces become a canvas for ideas that feel close to you.',
    image: '/process-textiles.jpg',
    imageAlt: 'Hands smoothing a cream T-shirt with a delicate plum botanical print',
  },
  {
    label: 'Everyday objects',
    title: 'Made meaningful',
    copy: 'Bottles, backpacks and gifts are transformed into objects with a story and a purpose.',
    image: '/process-everyday-objects.jpg',
    imageAlt: 'Personalised bottle, canvas backpack and gift box in a warm studio setting',
  },
  {
    label: 'Finishing',
    title: 'Checked with care',
    copy: 'Placement, colour and finish are reviewed so the final piece feels considered from every angle.',
    image: '/process-finishing.jpg',
    imageAlt: 'Hands tying a plum ribbon around a kraft gift box beside a folded personalised textile',
  },
]

function ProcessPage() {
  return (
    <ContentPage
      eyebrow="From idea to object"
      title={<><span>Your idea,</span><span>made tangible.</span></>}
      intro="Personalisation is more than adding a name. It is a thoughtful process that turns your inspiration into something made to be part of your story."
      className="process-page"
    >
      <section className="process-manifesto" aria-labelledby="process-manifesto-title">
        <div>
          <h2 id="process-manifesto-title">
            No two<br />
            stories<br />
            are<br />
            exactly<br />
            alike.
          </h2>
          <p className="eyebrow">A gift for every occasion</p>
        </div>
      </section>

      <section className="process-steps" aria-label="How personalisation works">
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
          <p className="eyebrow">Begin with an idea</p>
          <h2 id="process-cta-title">What will you make yours?</h2>
        </div>
        <a className="button button--primary" href="/products">Explore our products</a>
      </section>

      <section className="process-gallery" aria-label="Craft and finishing details">
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
        eyebrow="About personalisation"
        title="The details behind the process"
        items={[
          { question: 'Which personalisation techniques do you use?', answer: 'The technique depends on the product, material, design and desired finish. We select the option best suited to each piece.' },
          { question: 'How do you choose the right technique?', answer: 'We consider the material, number of colours, level of detail, intended use and quantity before deciding how the design should be applied.' },
          { question: 'Will I approve the design before production?', answer: 'When a digital mock-up is included, production begins only after the relevant design details have been confirmed.' },
          { question: 'Why can colours look slightly different?', answer: 'Screens reproduce colour differently, and materials absorb or reflect colour in their own way. Small variations are therefore possible.' },
          { question: 'How is each finished piece checked?', answer: 'We review placement, colour, finish and the overall condition of the item before it is carefully packed.' },
        ]}
        className="contextual-faqs--process"
      />

    </ContentPage>
  )
}
