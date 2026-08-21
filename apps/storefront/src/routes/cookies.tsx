import { createFileRoute } from '@tanstack/react-router'
import { PolicyPlaceholder } from '../components/content-page'

export const Route = createFileRoute('/cookies')({
  head: () => ({ meta: [{ title: 'Cookies policy — KnitPrint' }] }),
  component: CookiesPage,
})

function CookiesPage() {
  return (
    <PolicyPlaceholder
      eyebrow="Browsing KnitPrint"
      title="Cookies policy"
      intro="The final cookies policy and consent details will be added to this prepared structure."
      topics={['Essential cookies', 'Analytics and preferences', 'Third-party services', 'Managing your choices']}
    />
  )
}
