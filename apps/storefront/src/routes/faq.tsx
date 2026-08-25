import { createFileRoute } from '@tanstack/react-router'
import { useMemo, useState } from 'react'
import { ContentPage } from '../components/content-page'

export const Route = createFileRoute('/faq')({
  head: () => ({
    meta: [
      { title: 'Frequently asked questions — KnitPrint' },
      { name: 'description', content: 'Answers about KnitnPrint personalisation, products, orders, delivery, returns and B2B projects.' },
    ],
  }),
  component: FaqPage,
})

const faqGroups = [
  {
    id: 'personalisation',
    title: 'Personalisation',
    questions: [
      { question: 'What products can I personalise?', answer: 'You can personalise selected textiles, bottles, backpacks, accessories and gifts. The available options are shown on each product page.' },
      { question: 'What can I add to a product?', answer: 'Depending on the piece, you may be able to add a name, initials, a short phrase, an illustration or another graphic element.' },
      { question: 'Can I use my own design or illustration?', answer: 'Yes, provided the file has enough quality for the chosen technique and you have permission to use the design.' },
      { question: 'Will I see the design before production?', answer: 'When a digital mock-up is part of the service, we will ask you to confirm the layout, scale and placement before production begins.' },
      { question: 'Can I change my personalisation after ordering?', answer: 'Contact us as soon as possible. Changes may be possible before production starts, but cannot be guaranteed once your piece is being made.' },
    ],
  },
  {
    id: 'products',
    title: 'Products and materials',
    questions: [
      { question: 'What materials do you use?', answer: 'Materials vary by product and are described on the relevant product page. If you need help choosing, contact us before ordering.' },
      { question: 'Are product colours exactly as shown online?', answer: 'We take care to represent colours accurately, but screens, production batches and natural materials may create small variations.' },
      { question: 'How do I choose the right size?', answer: 'Please use the measurements provided in the product size guide and compare them with a similar piece you already own.' },
      { question: 'Are your products handmade?', answer: 'Personalisation and finishing include hands-on processes, even when specialist equipment is used to achieve a consistent result.' },
      { question: 'Can I request a product that is not listed?', answer: 'You are welcome to contact us with your idea. For larger quantities or company projects, use our B2B proposal form.' },
    ],
  },
  {
    id: 'orders',
    title: 'Orders and payments',
    questions: [
      { question: 'How do I place an order?', answer: 'Choose your product, select the available options, add your personalisation details and review everything carefully before checkout.' },
      { question: 'Which payment methods do you accept?', answer: 'The payment methods currently available for your order and location are shown securely during checkout.' },
      { question: 'Is my payment secure?', answer: 'Payments are handled by specialist payment providers. KnitnPrint does not store your complete card details.' },
      { question: 'Can I use more than one discount code?', answer: 'Unless a promotion states otherwise, discount codes cannot be combined. Enter your code during checkout to confirm whether it applies.' },
      { question: 'Where can I find my order confirmation?', answer: 'We send it to the email address used at checkout. If it is not in your inbox, please check your spam folder before contacting us.' },
    ],
  },
  {
    id: 'delivery',
    title: 'Production and delivery',
    questions: [
      { question: 'How long does production take?', answer: 'Production time depends on the product, quantity and complexity of the personalisation. The estimate provided with your order applies before shipping time.' },
      { question: 'Is production included in the delivery estimate?', answer: 'Production and shipping are separate stages. A personalised item must first be made and checked before it is handed to the carrier.' },
      { question: 'How will I know when my order has been shipped?', answer: 'We will send a dispatch confirmation and, whenever available, the information needed to follow the delivery.' },
      { question: 'Where do you deliver?', answer: 'Available destinations are shown during checkout. Contact us before ordering if your destination is not listed.' },
      { question: 'What should I do if my parcel arrives damaged?', answer: 'Contact us promptly with your order number and clear photographs of the parcel, packaging and affected items so we can assess what happened.' },
    ],
  },
  {
    id: 'returns',
    title: 'Returns and refunds',
    questions: [
      { question: 'Can personalised products be returned?', answer: 'Personalised products generally cannot be returned for a change of mind. This does not affect your rights when an item is faulty, damaged or different from what was agreed.' },
      { question: 'What if there is an error in my personalisation?', answer: 'Send us your order number and clear photographs. We will compare the finished item with the confirmed personalisation and explain the next step.' },
      { question: 'Can I return a non-personalised product?', answer: 'Eligible non-personalised products may be returned under the conditions and deadlines set out in our Return Policy.' },
      { question: 'Who pays the return shipping costs?', answer: 'This depends on the reason for the return. Please read our Return Policy or contact us before sending an item back.' },
      { question: 'When will I receive my refund?', answer: 'Once an eligible return has been received and inspected, the refund is processed within the period stated in our Return Policy.' },
    ],
  },
  {
    id: 'b2b',
    title: 'B2B orders',
    questions: [
      { question: 'Do you work with businesses and associations?', answer: 'Yes. We welcome enquiries from businesses, associations, schools, clubs, teams and event organisers.' },
      { question: 'Is there a minimum order quantity?', answer: 'Corporate projects are subject to minimum quantities, which vary according to the product and personalisation technique.' },
      { question: 'Can you help us choose the right products?', answer: 'Yes. We can help you consider the purpose, quantity, budget and finish before preparing a tailored proposal.' },
      { question: 'Do you provide a digital mock-up?', answer: 'A digital mock-up is prepared so you can review the composition, scale and placement before approving production.' },
      { question: 'Which logo file formats do you accept?', answer: 'You can send AI, EPS, PDF, SVG, PNG or JPG files. Vector artwork usually provides the best production result.' },
      { question: 'How can I request a proposal?', answer: 'Complete the form on our B2B page with your company details, product type, estimated quantity and logo file.' },
    ],
  },
  {
    id: 'care',
    title: 'Care instructions',
    questions: [
      { question: 'How should I wash personalised clothing?', answer: 'Unless the product instructions say otherwise, wash it inside out at a low temperature with a mild detergent and similar colours.' },
      { question: 'Can personalised clothing go in the tumble dryer?', answer: 'Natural air drying is usually the gentlest option and helps preserve both the garment and its personalisation.' },
      { question: 'Can I iron over the personalised area?', answer: 'Avoid ironing directly over the design. Turn the item inside out and follow the care label supplied with the garment.' },
      { question: 'How should I clean bottles and accessories?', answer: 'Care requirements depend on the material and finish. Follow the instructions provided with the product and avoid abrasive cleaning tools.' },
      { question: 'How can I make the personalisation last longer?', answer: 'Handle the piece with care, follow its washing or cleaning instructions and store it away from excessive heat, moisture and direct sunlight.' },
    ],
  },
]

function FaqPage() {
  const [query, setQuery] = useState('')
  const filteredGroups = useMemo(() => {
    const term = query.trim().toLowerCase()
    if (!term) return faqGroups
    return faqGroups
      .map((group) => ({
        ...group,
        questions: group.questions.filter(({ question, answer }) =>
          `${question} ${answer}`.toLowerCase().includes(term),
        ),
      }))
      .filter((group) => group.questions.length > 0)
  }, [query])
  const resultCount = filteredGroups.reduce((total, group) => total + group.questions.length, 0)

  return (
    <ContentPage
      eyebrow="Here to help"
      title="Frequently asked questions"
      intro="Everything you need to know about personalisation, orders, delivery and caring for your KnitnPrint pieces."
      className="faq-page"
    >
      <section className="faq-tools" aria-label="Find an answer">
        <label htmlFor="faq-search">What can we help you find?</label>
        <input
          id="faq-search"
          type="search"
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          placeholder="Search questions and answers"
        />
        <p aria-live="polite">{query ? `${resultCount} ${resultCount === 1 ? 'answer' : 'answers'} found` : 'Browse by topic'}</p>
      </section>

      {!query && (
        <nav className="faq-category-nav" aria-label="FAQ topics">
          {faqGroups.map((group) => <a href={`#${group.id}`} key={group.id}>{group.title}</a>)}
        </nav>
      )}

      <div className="faq-groups">
        {filteredGroups.map((group) => (
          <section className="faq-group" id={group.id} key={group.id} aria-labelledby={`${group.id}-title`}>
            <div className="faq-group-heading">
              <span>{String(faqGroups.findIndex((item) => item.id === group.id) + 1).padStart(2, '0')}</span>
              <h2 id={`${group.id}-title`}>{group.title}</h2>
            </div>
            <div className="faq-accordion">
              {group.questions.map(({ question, answer }) => (
                <details key={question} name={`faq-${group.id}`}>
                  <summary><span>{question}</span><span aria-hidden="true" /></summary>
                  <p>{answer}</p>
                </details>
              ))}
            </div>
          </section>
        ))}
        {filteredGroups.length === 0 && (
          <div className="faq-empty">
            <h2>No answers found</h2>
            <p>Try a different search or contact us and we will be happy to help.</p>
            <button type="button" className="text-link" onClick={() => setQuery('')}>Clear search</button>
          </div>
        )}
      </div>

      <section className="faq-contact" aria-labelledby="faq-contact-title">
        <div>
          <p className="eyebrow">Still need help?</p>
          <h2 id="faq-contact-title">We’re here to help.</h2>
          <p>If you could not find the answer you were looking for, send us a message and we will be happy to help.</p>
        </div>
        <div className="faq-contact-actions">
          <a className="button button--primary" href="mailto:support@knitnprint.com">Email support</a>
          <a className="text-link" href="/b2b">Request a B2B proposal</a>
        </div>
      </section>
    </ContentPage>
  )
}
