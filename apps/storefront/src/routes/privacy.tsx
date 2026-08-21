import { createFileRoute } from '@tanstack/react-router'
import { PolicyPlaceholder } from '../components/content-page'

export const Route = createFileRoute('/privacy')({
  head: () => ({ meta: [{ title: 'Privacy policy — KnitPrint' }] }),
  component: PrivacyPage,
})

function PrivacyPage() {
  return (
    <PolicyPlaceholder
      eyebrow="Your information"
      title="Privacy policy"
      intro="Our complete privacy notice will explain clearly how KnitPrint handles customer information."
      topics={['Information we collect', 'How information is used', 'How information is protected', 'Your privacy choices']}
    />
  )
}
