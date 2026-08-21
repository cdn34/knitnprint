import { createFileRoute } from '@tanstack/react-router'
import { PolicyPlaceholder } from '../components/content-page'

export const Route = createFileRoute('/terms')({
  head: () => ({ meta: [{ title: 'Terms and conditions — KnitPrint' }] }),
  component: TermsPage,
})

function TermsPage() {
  return (
    <PolicyPlaceholder
      eyebrow="Shopping with KnitPrint"
      title="Terms and conditions"
      intro="The approved terms for using our store and purchasing KnitPrint products will be published here."
      topics={['Using our online store', 'Orders and payment', 'Delivery and fulfilment', 'Customer responsibilities']}
    />
  )
}
