import { createFileRoute } from '@tanstack/react-router'
import type { ReactNode } from 'react'
import { ContentPage } from '../components/content-page'
import { ContextualFaqs } from '../components/contextual-faqs'

export const Route = createFileRoute('/privacy')({
  head: () => ({
    meta: [
      { title: 'Privacy Policy — KnitnPrint' },
      { name: 'description', content: 'Learn how KnitnPrint collects, uses and protects your personal data.' },
    ],
  }),
  component: PrivacyPage,
})

function PrivacyPage() {
  return (
    <ContentPage
      eyebrow="Your information"
      title="Privacy Policy"
      intro="This Privacy Policy explains how KnitnPrint collects, uses and protects your personal data when you visit our website or shop online."
      className="policy-page legal-document-page"
    >
      <div className="policy-layout">
        <article className="policy-document">
          <section className="company-information" aria-labelledby="data-controller-title">
            <span aria-hidden="true">01</span>
            <div>
              <p className="eyebrow">Legal information</p>
              <h2 id="data-controller-title">Data controller</h2>
              <dl>
                <div><dt>Trading name</dt><dd>KnitnPrint</dd></div>
                <div><dt>Owner</dt><dd>Daniela Tojal</dd></div>
                <div><dt>Tax identification number</dt><dd>220666768</dd></div>
                <div><dt>Email</dt><dd><a href="mailto:support@knitnprint.com">support@knitnprint.com</a></dd></div>
                <div><dt>Address</dt><dd>Online store</dd></div>
              </dl>
            </div>
          </section>

          <div className="policy-opening">
            <p>This Privacy Policy describes how KnitnPrint, the company responsible for <a href="https://knitnprint.com">knitnprint.com</a>, collects, uses and protects users’ personal data when they visit the website or make purchases through the online store.</p>
            <p>By using this website, you accept the practices described in this Privacy Policy.</p>
          </div>

          <p className="eyebrow policy-document-label">How we handle your data</p>

          <PolicySection number="02" title="Data we collect">
            <p>When you visit our online store, we collect certain information so that we can improve our services and serve our customers more effectively.</p>
            <p>The information we collect includes:</p>
            <ul>
              <li><strong>Contact details:</strong> name, address, telephone number and email address;</li>
              <li><strong>Order information:</strong> name, delivery address, billing address, payment confirmation, email address and telephone number;</li>
              <li><strong>Account information:</strong> username and password.</li>
            </ul>
            <p>We may also obtain information about you from third parties. For example, payment providers may supply information needed to confirm that a transaction complies with our purchasing policy.</p>
            <p>We may collect information about your use of our services through cookies. This may include how you access our website, browser and network connection information, your IP address and other details about your interactions with the services.</p>
          </PolicySection>

          <PolicySection number="03" title="Purposes and processing of personal data">
            <p>Personal data collected through the website may be used to:</p>
            <ul>
              <li>Process, manage and track orders;</li>
              <li>Process the corresponding payments;</li>
              <li>Arrange and track the dispatch and delivery of orders;</li>
              <li>Issue invoices, receipts and purchase confirmations;</li>
              <li>Provide customer service and support;</li>
              <li>Prevent and detect possible fraud or other unlawful activities;</li>
              <li>Improve the website’s operation, security and performance;</li>
              <li>Send promotional communications, news or product information where the user has given prior consent.</li>
            </ul>
          </PolicySection>

          <PolicySection number="04" title="Sharing personal data">
            <p>Personal data may be shared with third parties that provide services essential to the operation of the online store, including:</p>
            <ul>
              <li>Payment service providers;</li>
              <li>Carriers responsible for delivering orders;</li>
              <li>Website traffic and usage analytics platforms and services.</li>
            </ul>
            <p>We may also use analytics services such as Google Analytics to understand how users interact with the website and improve its navigation, performance and user experience.</p>
            <p>These organisations will only have access to the data strictly necessary to provide the contracted services and must also comply with applicable personal data protection legislation.</p>
            <p>Data may also be disclosed where necessary to:</p>
            <ul>
              <li>Comply with legal or regulatory obligations;</li>
              <li>Respond to requests from competent authorities or organisations;</li>
              <li>Defend and protect the company’s rights and legitimate interests.</li>
            </ul>
          </PolicySection>

          <PolicySection number="05" title="Advertising and marketing">
            <p>We may use the data collected to display advertisements or marketing communications that may be of interest to you.</p>
            <p>For this purpose, we may use advertising services from platforms such as:</p>
            <ul>
              <li>Facebook;</li>
              <li>Google;</li>
              <li>Instagram.</li>
            </ul>
            <p>You may unsubscribe from marketing communications at any time.</p>
          </PolicySection>

          <PolicySection number="06" title="Data subject rights">
            <p>Under the General Data Protection Regulation (GDPR), users have the right to:</p>
            <ul>
              <li>Access their personal data;</li>
              <li>Request the correction of inaccurate data;</li>
              <li>Request the deletion of their data;</li>
              <li>Restrict or object to the processing of their data;</li>
              <li>Request data portability.</li>
            </ul>
          </PolicySection>

          <PolicySection number="07" title="International data transfers">
            <p>Some services used by the website may involve transferring data outside the European Economic Area, including to countries such as the United States or Canada.</p>
            <p>Whenever this occurs, appropriate safeguards are adopted to protect personal data in accordance with the GDPR.</p>
          </PolicySection>

          <PolicySection number="08" title="Data retention">
            <p>Personal data will only be retained for as long as necessary to fulfil the purposes for which it was collected, including legal, tax and accounting obligations.</p>
            <p>When you place an order, the related data may be retained for legal and tax purposes unless you expressly request its deletion, where applicable.</p>
          </PolicySection>

          <PolicySection number="09" title="Changes to this Privacy Policy">
            <p>We reserve the right to update this Privacy Policy whenever necessary to reflect legal, technical or operational changes.</p>
            <p>Any changes will be published on this page.</p>
          </PolicySection>

          <PolicySection number="10" title="Contact us">
            <p>If you have any questions about this Privacy Policy or how we process your personal data, please contact us at <a href="mailto:support@knitnprint.com">support@knitnprint.com</a>.</p>
          </PolicySection>
        </article>
      </div>
      <ContextualFaqs
        id="privacy-faqs"
        eyebrow="Privacy questions"
        title="Your data, explained simply"
        items={[
          { question: 'What personal data do you collect?', answer: 'Depending on how you use the website, we may collect contact, order, delivery, payment-related and website usage information.' },
          { question: 'Why do you need my information?', answer: 'We use the information needed to process orders, provide support, meet legal obligations, improve our services and, where permitted, communicate with you.' },
          { question: 'Do you share my data with other organisations?', answer: 'Only where necessary, such as with payment, delivery, hosting or professional service providers, or where disclosure is legally required.' },
          { question: 'How can I exercise my data protection rights?', answer: 'You may request access, correction, deletion, restriction, objection or portability where applicable under data protection law.' },
          { question: 'How can I contact you about my data?', answer: 'Email support@knitnprint.com with your privacy question or request.' },
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
