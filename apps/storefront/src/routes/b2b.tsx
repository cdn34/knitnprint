import { createFileRoute } from '@tanstack/react-router'
import { Send } from 'lucide-react'
import { ContentPage } from '../components/content-page'

export const Route = createFileRoute('/b2b')({
  head: () => ({
    meta: [
      { title: 'B2B — KnitPrint' },
      { name: 'description', content: 'Personalised clothing and corporate gifts for businesses, associations, teams and events.' },
    ],
  }),
  component: B2BPage,
})

const benefits = [
  { title: 'Dedicated guidance', copy: 'A close point of contact to help you choose the right products, finishes and personalisation for your goals.' },
  { title: 'Flexibility', copy: 'Solutions shaped around your quantities, timings and budget, whether for a team, an event or a complete kit.' },
  { title: 'Precision and quality', copy: 'Careful production and quality checks ensure every piece represents your organisation with confidence.' },
]

const steps = [
  { number: '01', title: 'Proposal request', copy: 'Tell us what you need, the intended quantity and the occasion or purpose behind the project.' },
  { number: '02', title: 'Digital mock-up', copy: 'We prepare a visual proposal so you can see the placement, scale and overall result before production.' },
  { number: '03', title: 'Approval and production', copy: 'Once every detail is approved, we carefully personalise each item and check the final finish.' },
  { number: '04', title: 'Delivery', copy: 'Your order is securely packed and prepared to reach your business, association or event on time.' },
]

function B2BPage() {
  return (
    <ContentPage
      eyebrow="B2B*"
      title="Tailored solutions for your business"
      intro={
        <>
          <span>Turn your identity into a recognisable brand. At KnitnPrint, we personalise clothing and corporate gifts that strengthen team spirit, elevate events and make an impression on clients and partners.</span>
          <span className="b2b-minimum-note">*Corporate projects are subject to minimum order quantities. Contact us for more information.</span>
        </>
      }
      className="b2b-page"
    >
      <section className="b2b-hero" aria-labelledby="b2b-hero-title">
        <div>
          <p className="eyebrow">Made to represent you</p>
          <h2 id="b2b-hero-title">
            <span>Your brand</span>
            <span>in every</span>
            <span>detail.</span>
          </h2>
        </div>
      </section>

      <section className="b2b-section" aria-labelledby="b2b-benefits-title">
        <div className="b2b-section-heading">
          <p className="eyebrow">A trusted partner</p>
          <h2 id="b2b-benefits-title">Why KnitnPrint?</h2>
        </div>
        <div className="b2b-benefits">
          {benefits.map(({ title, copy }) => (
            <article key={title}>
              <h3>{title}</h3>
              <p>{copy}</p>
            </article>
          ))}
        </div>
      </section>

      <section className="b2b-section b2b-process" aria-labelledby="b2b-process-title">
        <div className="b2b-section-heading">
          <p className="eyebrow">From brief to delivery</p>
          <h2 id="b2b-process-title">How it works</h2>
        </div>
        <div className="b2b-steps">
          {steps.map(({ number, title, copy }) => (
            <article key={number}>
              <span>{number}</span>
              <h3>{title}</h3>
              <p>{copy}</p>
            </article>
          ))}
        </div>
      </section>

      <section className="b2b-contact" aria-labelledby="b2b-contact-title">
        <div className="b2b-contact-intro">
          <p className="eyebrow">Let’s create together</p>
          <h2 id="b2b-contact-title">Request a tailored proposal</h2>
          <p>Share the essentials below and we will have the right information to understand your project and prepare the next steps.</p>
          <a href="mailto:support@knitnprint.com">support@knitnprint.com</a>
        </div>

        <form className="b2b-form">
          <div className="b2b-form-row">
            <label>Company or organisation<input name="company" type="text" autoComplete="organization" required /></label>
            <label>Contact name<input name="contactName" type="text" autoComplete="name" required /></label>
          </div>
          <div className="b2b-form-row">
            <label>Email<input name="email" type="email" autoComplete="email" required /></label>
            <label>Phone number<input name="phone" type="tel" autoComplete="tel" required /></label>
          </div>
          <label>
            Product type
            <select name="productType" defaultValue="" required>
              <option value="" disabled>Select an option</option>
              <option value="clothing">Clothing</option>
              <option value="bottles">Bottles</option>
              <option value="backpacks">Backpacks</option>
              <option value="complete-kit">Complete kit</option>
              <option value="other">Other</option>
            </select>
          </label>
          <label>Estimated quantity<input name="quantity" type="number" min="1" inputMode="numeric" required /></label>
          <label className="b2b-file-field">
            Brand logo or vector file
            <input name="brandFile" type="file" accept=".ai,.eps,.pdf,.svg,.png,.jpg,.jpeg" required />
            <span>AI, EPS, PDF, SVG, PNG or JPG</span>
          </label>
          <label>
            Project notes <span className="b2b-optional">Optional</span>
            <textarea name="notes" rows={4} placeholder="Tell us about timings, colours, placement or any other useful details." />
          </label>
          <button className="button button--primary" type="submit">Request a proposal <Send size={15} aria-hidden="true" /></button>
          <p className="b2b-form-note">We will only use these details to respond to your enquiry.</p>
        </form>
      </section>
    </ContentPage>
  )
}
