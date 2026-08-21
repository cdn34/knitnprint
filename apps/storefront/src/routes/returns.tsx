import { createFileRoute } from '@tanstack/react-router'
import { PolicyPlaceholder } from '../components/content-page'

export const Route = createFileRoute('/returns')({
  head: () => ({ meta: [{ title: 'Return policy — KnitPrint' }] }),
  component: ReturnsPage,
})

function ReturnsPage() {
  return (
    <PolicyPlaceholder
      eyebrow="After your order arrives"
      title="Return policy"
      intro="Our final return conditions and instructions will be published here once the policy text is approved."
      topics={['Return eligibility', 'How to request a return', 'Personalized products', 'Refunds and exchanges']}
    />
  )
}
