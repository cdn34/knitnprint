import { createFileRoute } from '@tanstack/react-router'
import type { ReactNode } from 'react'
import { ContentPage } from '../components/content-page'
import { ContextualFaqs } from '../components/contextual-faqs'

export const Route = createFileRoute('/returns')({
  head: () => ({
    meta: [
      { title: 'Exchanges, Returns and Refunds Policy — KnitnPrint' },
      {
        name: 'description',
        content: 'Read the KnitnPrint policy for exchanges, returns and refunds, including the conditions for personalised products.',
      },
    ],
  }),
  component: ReturnsPage,
})

function ReturnsPage() {
  return (
    <ContentPage
      eyebrow="After your order arrives"
      title="Exchanges, Returns and Refunds Policy"
      intro="Learn how to request an exchange, return or refund for a KnitnPrint order."
      className="policy-page legal-document-page"
    >
      <div className="policy-layout">
        <article className="policy-document">
          <p className="eyebrow policy-document-label">Returns, exchanges and refunds</p>

          <PolicySection number="01" title="Scope">
            <p>This Exchanges, Returns and Refunds Policy applies to all purchases made from KnitnPrint and sets out the conditions under which customers may request the return or exchange of a product, as well as the corresponding refund.</p>
          </PolicySection>

          <PolicySection number="02" title="Non-personalised products">
            <h3>2.1 Right of withdrawal</h3>
            <p>Under Decree-Law No. 24/2014, customers may exercise their right of withdrawal within 14 days from the date on which they receive their order, without having to provide a reason.</p>

            <h3>2.2 Return conditions</h3>
            <p>For a return to be accepted, the product must:</p>
            <ul>
              <li>Be in perfect condition and show no signs of use;</li>
              <li>Be returned in its original packaging, together with all components and accessories;</li>
              <li>Be accompanied by the relevant proof of purchase.</li>
            </ul>

            <h3>2.3 Return costs</h3>
            <p>The costs associated with returning the product are the responsibility of the customer, except where the return results from an error attributable to KnitnPrint or from a product defect.</p>

            <h3>2.4 Refund</h3>
            <p>Once the returned product has been received and inspected, the refund will be processed within a maximum of 14 days.</p>
            <p>The amount will be refunded using the same payment method used for the purchase, unless expressly agreed otherwise.</p>
          </PolicySection>

          <PolicySection number="03" title="Personalised products">
            <h3>3.1 Exclusion from the right of withdrawal</h3>
            <p>In accordance with applicable legislation, personalised products, products made to the customer’s specifications or products clearly adapted to their needs cannot be returned under the right of withdrawal.</p>
            <p>This exclusion does not affect the customer’s rights where the product has a defect, error or lack of conformity.</p>

            <h3>3.2 Eligible circumstances</h3>
            <p>KnitnPrint will accept complaints concerning personalised products in the following circumstances:</p>
            <ul>
              <li>Manufacturing defect;</li>
              <li>A personalisation error attributable to KnitnPrint, including differences from the approved design;</li>
              <li>Damage occurring during transport.</li>
            </ul>

            <h3>3.3 Procedure</h3>
            <p>The customer must report the issue within a maximum of five business days after receiving the order, providing photographs of the product and a detailed description of the problem.</p>
            <p>After reviewing the request, KnitnPrint may propose one of the following solutions, depending on the circumstances:</p>
            <ul>
              <li>Replacement of the product;</li>
              <li>A partial or full refund;</li>
              <li>Store credit.</li>
            </ul>
          </PolicySection>

          <PolicySection number="04" title="Damaged or incorrect products">
            <p>If the customer receives a damaged product or a product that is different from the one ordered, they must contact KnitnPrint within a maximum of five business days after receiving it.</p>
            <p>The request must include:</p>
            <ul>
              <li>Photographs of the product received;</li>
              <li>The order number;</li>
              <li>A detailed description of the problem.</li>
            </ul>
            <p>Where damage, an error or a lack of conformity attributable to KnitnPrint is confirmed, KnitnPrint will cover the return costs and, as applicable, replace the product or issue the corresponding refund.</p>
          </PolicySection>

          <PolicySection number="05" title="How to request a return">
            <p>To begin the return process, the customer must:</p>
            <ol>
              <li>Contact KnitnPrint at <a href="mailto:support@knitnprint.com">support@knitnprint.com</a>;</li>
              <li>Provide the order number and reason for the return;</li>
              <li>Wait for the necessary instructions before sending the product.</li>
            </ol>
            <p>Products must not be sent before the relevant return instructions have been provided.</p>
          </PolicySection>

          <PolicySection number="06" title="Contact us">
            <p>For any questions about exchanges, returns or refunds, please contact:</p>
            <p>KnitnPrint<br />Email: <a href="mailto:support@knitnprint.com">support@knitnprint.com</a></p>
          </PolicySection>

          <PolicySection number="07" title="Changes to this Policy">
            <p>KnitnPrint reserves the right to update or amend this Policy at any time. Any changes will be published on the website and will take effect from their date of publication.</p>
          </PolicySection>
        </article>
      </div>
      <ContextualFaqs
        id="returns-faqs"
        eyebrow="Returns at a glance"
        title="Quick answers about returns"
        items={[
          { question: 'Can I return a personalised product?', answer: 'Personalised products cannot be returned under the right of withdrawal, but this does not affect your rights when an item is faulty, damaged or not as agreed.' },
          { question: 'What should I do if my order arrives damaged?', answer: 'Contact us within five business days with your order number, a description of the issue and clear photographs of the product and packaging.' },
          { question: 'How do I start a return?', answer: 'Email support@knitnprint.com with your order number and reason for the return, then wait for our instructions before sending the product.' },
          { question: 'Who pays the return shipping costs?', answer: 'Return costs are normally paid by the customer, except when the return results from a KnitnPrint error or a confirmed product defect.' },
          { question: 'When will I receive my refund?', answer: 'Once an eligible return has been received and inspected, the refund will be processed within a maximum of 14 days.' },
        ]}
        className="contextual-faqs--policy"
      />
    </ContentPage>
  )
}

function PolicySection({ number, title, children }: Readonly<{
  number: string
  title: string
  children: ReactNode
}>) {
  return (
    <section>
      <span aria-hidden="true">{number}</span>
      <div>
        <h2>{title}</h2>
        <div className="policy-section-content">{children}</div>
      </div>
    </section>
  )
}
