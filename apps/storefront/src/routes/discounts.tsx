import { createFileRoute } from '@tanstack/react-router'
import { useState } from 'react'
import { ArrowRight } from 'lucide-react'
import { ContentPage } from '../components/content-page'

export const Route = createFileRoute('/discounts')({
  head: () => ({
    meta: [
      { title: '10% welcome discount — KnitPrint' },
      {
        name: 'description',
        content: 'Join the KnitnPrint newsletter and receive 10% off your first eligible order.',
      },
    ],
  }),
  component: DiscountsPage,
})

function DiscountsPage() {
  const [submitted, setSubmitted] = useState(false)

  return (
    <ContentPage
      eyebrow="A little something for you"
      title="Make your first story personal."
      intro="Join our newsletter for thoughtful inspiration, new pieces and a 10% welcome discount."
      className="discount-page"
    >
      <section className="discount-signup" aria-labelledby="discount-signup-title">
        <div className="discount-offer" aria-hidden="true">
          <span>Welcome</span>
          <strong>10%</strong>
          <span>off your first order</span>
        </div>

        <div className="discount-form-panel">
          <p className="eyebrow">Made for your inbox</p>
          <h2 id="discount-signup-title">Your ideas deserve a lovely beginning.</h2>
          <p>
            Be the first to discover new collections, personalisation ideas and
            stories made to inspire your next piece.
          </p>

          {submitted ? (
            <div className="discount-success" role="status">
              <span>Thank you for joining us.</span>
              <strong>Your 10% welcome code is on its way.</strong>
              <p>Please check your inbox — and your spam folder, just in case.</p>
            </div>
          ) : (
            <form
              className="discount-form"
              onSubmit={(event) => {
                event.preventDefault()
                setSubmitted(true)
              }}
            >
              <label htmlFor="discount-email">Email address</label>
              <div>
                <input
                  id="discount-email"
                  name="email"
                  type="email"
                  autoComplete="email"
                  placeholder="you@example.com"
                  required
                />
                <button className="button button--primary" type="submit">
                  Get 10% off <ArrowRight size={16} aria-hidden="true" />
                </button>
              </div>
              <p>
                By subscribing, you agree to receive KnitnPrint news and offers.
                You can unsubscribe at any time.
              </p>
            </form>
          )}

          <aside className="discount-exclusion">
            <span aria-hidden="true">*</span>
            <p><strong>Please note:</strong> the welcome discount is not valid for B2B orders.</p>
          </aside>
        </div>
      </section>
    </ContentPage>
  )
}
