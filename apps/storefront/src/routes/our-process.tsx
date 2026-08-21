import { createFileRoute } from '@tanstack/react-router'
import { Image, Layers3, PackageCheck, Sparkles } from 'lucide-react'
import { ContentPage } from '../components/content-page'

export const Route = createFileRoute('/our-process')({
  head: () => ({
    meta: [
      { title: 'Our process — KnitPrint' },
      { name: 'description', content: 'A closer look at how KnitPrint pieces are made.' },
    ],
  }),
  component: ProcessPage,
})

const processSteps = [
  {
    icon: Sparkles,
    title: 'The idea',
    copy: 'A space for sketches, inspiration, and the story behind each new piece.',
  },
  {
    icon: Layers3,
    title: 'Making the piece',
    copy: 'A space to explain materials, production, personalization, and finishing.',
  },
  {
    icon: PackageCheck,
    title: 'The final details',
    copy: 'A space for quality checks, careful packing, and preparation for delivery.',
  },
]

function ProcessPage() {
  return (
    <ContentPage
      eyebrow="From idea to object"
      title="A thoughtful process, one layer at a time."
      intro="This first structure gives us room to explain how every KnitPrint piece moves from an idea to a finished object."
      className="process-page"
    >
      <section className="process-steps" aria-label="Production process outline">
        {processSteps.map(({ icon: Icon, title, copy }, index) => (
          <article key={title}>
            <span>0{index + 1}</span>
            <Icon aria-hidden="true" />
            <h2>{title}</h2>
            <p>{copy}</p>
          </article>
        ))}
      </section>

      <section className="process-gallery" aria-label="Future process photographs">
        {[1, 2, 3].map((item) => (
          <figure key={item}>
            <Image aria-hidden="true" />
            <figcaption>Production image {item}</figcaption>
          </figure>
        ))}
      </section>
    </ContentPage>
  )
}
