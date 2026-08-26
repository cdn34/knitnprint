import { createFileRoute } from '@tanstack/react-router'
import type { ReactNode } from 'react'
import { ContentPage } from '../components/content-page'
import { useI18n } from '../i18n'
import { termsEs, termsPt, type LegalSection } from '../i18n/terms-content'

export const Route = createFileRoute('/terms')({
  head: () => ({
    meta: [
      { title: 'Terms and Conditions — KnitnPrint' },
      { name: 'description', content: 'Read the Terms and Conditions of the KnitnPrint online store.' },
    ],
  }),
  component: TermsPage,
})

function TermsPage() {
  const { locale, t } = useI18n()
  return (
    <ContentPage
      eyebrow={t('terms.eyebrow')}
      title={t('terms.title')}
      intro={t('terms.intro')}
      className="policy-page legal-document-page"
    >
      <div className="policy-layout">
        <article className="policy-document">
          <section className="company-information" aria-labelledby="company-information-title">
            <span aria-hidden="true">01</span>
            <div>
              <p className="eyebrow">{t('terms.eyebrow')}</p>
              <h2 id="company-information-title">{t('terms.companyTitle')}</h2>
              <dl>
                <div><dt>{t('terms.tradingName')}</dt><dd>KnitnPrint</dd></div>
                <div><dt>{t('terms.owner')}</dt><dd>Daniela Tojal</dd></div>
                <div><dt>{t('terms.taxNumber')}</dt><dd>220666768</dd></div>
                <div><dt>{t('terms.email')}</dt><dd><a href="mailto:support@knitnprint.com">support@knitnprint.com</a></dd></div>
                <div><dt>{t('terms.address')}</dt><dd>{t('terms.onlineStore')}</dd></div>
              </dl>
            </div>
          </section>

          <p className="eyebrow policy-document-label">{t('terms.conditions')}</p>

          {locale === 'en' ? <>

          <PolicySection number="02" title="Purpose and scope">
            <p>KnitnPrint is an online store based in Portugal that sells personalised and other products, with the aim of meeting its customers’ needs in a variety of areas. These Terms and Conditions govern access to, browsing and use of the website <a href="https://knitnprint.com">knitnprint.com</a>, as well as the conditions applicable to purchasing products through the online store.</p>
            <p>By using this website or placing an order, you fully accept these Terms and Conditions, as well as the Privacy Policy and Cookie Policy.</p>
            <p>If you do not agree with the terms described here, you must refrain from using this website.</p>
          </PolicySection>

          <PolicySection number="03" title="Changes to the Terms and Conditions">
            <p>KnitnPrint reserves the right to change these Terms and Conditions at any time. Any changes will take effect once published on the website.</p>
          </PolicySection>

          <PolicySection number="04" title="Use of the website">
            <p>By accessing and using this website, you agree to:</p>
            <ul>
              <li>Use the platform solely for legitimate enquiries and orders;</li>
              <li>Refrain from placing false, misleading or fraudulent orders;</li>
              <li>Provide complete, accurate and up-to-date information, including your address, email address and any other necessary contact details.</li>
            </ul>
            <p>Whenever there are reasonable grounds to suspect fraudulent use, the company may cancel the order and, where applicable, report the matter to the relevant authorities.</p>
            <p>By making a purchase through this website, you confirm that you are at least 18 years old and have the legal capacity to enter into contracts.</p>
          </PolicySection>

          <PolicySection number="05" title="Product availability">
            <p>All products displayed on the website are subject to stock availability.</p>
            <p>If, after an order has been confirmed, any purchased product is unavailable, the customer will be informed and may choose to:</p>
            <ul>
              <li>Wait for the product to be restocked;</li>
              <li>Choose an alternative item;</li>
              <li>Request a refund of the amount paid.</li>
            </ul>
          </PolicySection>

          <PolicySection number="06" title="Purchase process">
            <p>To place an order, you must:</p>
            <ol>
              <li>Select the products you wish to purchase;</li>
              <li>Add them to your shopping cart;</li>
              <li>Provide your billing and delivery details;</li>
              <li>Choose a payment method;</li>
              <li>Confirm the order.</li>
            </ol>
            <p>After completing the purchase, the customer will receive an email confirming receipt of the order.</p>
            <p>The contract is only concluded when the order dispatch confirmation email is sent.</p>
          </PolicySection>

          <PolicySection number="07" title="Prices">
            <p>All prices displayed on the website include VAT at the applicable statutory rate, unless expressly stated otherwise.</p>
            <p>Any delivery costs are shown before the purchase is completed and confirmed. They are calculated according to the rates charged by our partner carriers and depend on the order’s weight, size and destination.</p>
            <p>The company may update or change product prices at any time, without affecting orders that have already been duly confirmed.</p>
            <p>If an obvious pricing error is identified, the customer will be informed and may choose to proceed with the order at the correct price or cancel it.</p>
          </PolicySection>

          <PolicySection number="08" title="Payment methods">
            <p>Payments are made by credit card through the Stripe platform.</p>
          </PolicySection>

          <PolicySection number="09" title="Delivery">
            <p>Delivery times may vary due to logistical and personalisation requirements.</p>
            <p>Delivery is completed when the customer, or someone acting on their behalf, receives the order.</p>
          </PolicySection>

          <PolicySection number="10" title="Right of withdrawal">
            <p>Under Decree-Law No. 24/2014, consumers have 14 days to withdraw from the contract without having to provide a reason.</p>
            <p>This period begins on the date the consumer, or a third party appointed by them, receives the product.</p>
            <p>To exercise the right of withdrawal, the customer must communicate their decision by email to <a href="mailto:support@knitnprint.com">support@knitnprint.com</a>.</p>
          </PolicySection>

          <PolicySection number="11" title="Returns">
            <p>Returned items must meet the following conditions:</p>
            <ul>
              <li>Be in perfect condition;</li>
              <li>Show no signs of use;</li>
              <li>Be returned in their original packaging;</li>
              <li>Include the relevant proof of purchase.</li>
            </ul>
            <p>Return costs are borne by the customer, except where the return results from a product defect or a delivery error.</p>
            <p>Refunds will be processed within a maximum of 14 days and, whenever possible, issued through the same payment method used for the purchase.</p>
            <p>Personalised products or products made as part of large-quantity orders cannot be returned, except in the event of a defect, production error or non-conformity with the order placed.</p>
            <p>KnitnPrint reserves the right to refuse returns that do not meet the criteria described here.</p>
            <p>If you receive a damaged or defective product, please contact us as soon as possible at <a href="mailto:support@knitnprint.com">support@knitnprint.com</a>.</p>
          </PolicySection>

          <PolicySection number="12" title="Warranty">
            <p>All products sold are covered by the statutory warranty applicable to consumer goods under Portuguese law.</p>
            <p>For new products, consumers benefit from a three-year warranty period, as provided by law.</p>
            <p>The warranty does not cover situations resulting from:</p>
            <ul>
              <li>Normal wear and tear from use of the product;</li>
              <li>Improper or inappropriate use;</li>
              <li>Alterations, modifications or interventions carried out by the customer;</li>
              <li>Natural variations or minor irregularities inherent to the handcrafted production process and the characteristics of natural leather, such as differences in shade, texture or grain, which are not considered manufacturing defects.</li>
            </ul>
          </PolicySection>

          <PolicySection number="13" title="Data protection">
            <p>KnitnPrint is committed to protecting its customers’ privacy and acts in accordance with the General Data Protection Regulation (GDPR — EU 2016/679).</p>
            <p>The data collected is used solely to manage orders and communicate with customers.</p>
            <p>Customers have the right to access, correct or delete their data at any time.</p>
          </PolicySection>

          <PolicySection number="14" title="Complaints Book">
            <p>In accordance with applicable legislation, the Portuguese Electronic Complaints Book is available at <a href="https://www.livroreclamacoes.pt/">livroreclamacoes.pt</a>.</p>
          </PolicySection>

          <PolicySection number="15" title="Governing law">
            <p>These Terms and Conditions are governed by Portuguese law.</p>
            <p>In the event of a dispute, the court of the district in which the company has its registered office will have jurisdiction, without prejudice to the statutory provisions applicable to consumers.</p>
          </PolicySection>
          </> : (
            <LocalizedTerms sections={locale === 'pt' ? termsPt : termsEs} />
          )}
        </article>
      </div>
    </ContentPage>
  )
}

function LocalizedTerms({ sections }: Readonly<{ sections: LegalSection[] }>) {
  return sections.map((section) => (
    <PolicySection number={section.number} title={section.title} key={section.number}>
      {section.blocks.map((block, index) => block.type === 'p' ? (
        <p key={index}>{renderLegalText(block.text)}</p>
      ) : block.type === 'ul' ? (
        <ul key={index}>{block.items.map((item) => <li key={item}>{renderLegalText(item)}</li>)}</ul>
      ) : (
        <ol key={index}>{block.items.map((item) => <li key={item}>{renderLegalText(item)}</li>)}</ol>
      ))}
    </PolicySection>
  ))
}

function renderLegalText(text: string) {
  return text.split(/(\[site\]|\[email\]|\[complaints\])/).map((part, index) => {
    if (part === '[site]') return <a href="https://knitnprint.com" key={index}>knitnprint.com</a>
    if (part === '[email]') return <a href="mailto:support@knitnprint.com" key={index}>support@knitnprint.com</a>
    if (part === '[complaints]') return <a href="https://www.livroreclamacoes.pt/" key={index}>livroreclamacoes.pt</a>
    return part
  })
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
