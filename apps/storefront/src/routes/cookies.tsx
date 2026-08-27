import { createFileRoute } from '@tanstack/react-router'
import type { ReactNode } from 'react'
import { ContentPage } from '../components/content-page'
import { ContextualFaqs } from '../components/contextual-faqs'
import { useI18n } from '../i18n'
import { cookiesEs, cookiesPt } from '../i18n/cookies-content'

export const Route = createFileRoute('/cookies')({
  head: () => ({
    meta: [
      { title: 'Cookies Policy — KnitnPrint' },
      { name: 'description', content: 'Learn how KnitnPrint uses cookies and how you can manage your preferences.' },
    ],
  }),
  component: CookiesPage,
})

function CookiesPage() {
  const { locale, t } = useI18n()
  const localized = locale === 'pt' ? cookiesPt : cookiesEs
  return (
    <ContentPage
      eyebrow={t('cookies.eyebrow')}
      title={t('cookies.title')}
      intro={t('cookies.intro')}
      className="policy-page legal-document-page"
    >
      <div className="policy-layout">
        <article className="policy-document">
          {locale === 'en' ? <>
          <div className="policy-opening policy-opening--first">
            <p>This Cookies Policy explains how KnitnPrint uses cookies and similar technologies on its website, as well as the choices available to users regarding their use.</p>
            <p>This Policy should be read together with our Privacy Policy, where you can find further information about how we process and protect your personal data.</p>
          </div>

          <p className="eyebrow policy-document-label">{t('cookies.label')}</p>

          <PolicySection number="01" title="What are cookies?">
            <p>Cookies are small information files stored on the computer, smartphone, tablet or other device used to access a website.</p>
            <p>These files allow a website to recognise the browser or device being used, save certain preferences and collect information about how the website is browsed and used.</p>
            <p>Cookies may be placed directly by our website or by services provided by third parties.</p>
          </PolicySection>

          <PolicySection number="02" title="What types of cookies do we use?">
            <p>The website may use different types of cookies depending on their purpose.</p>

            <h3>Strictly necessary cookies</h3>
            <p>These cookies are essential for the website and online store to function correctly. Among other purposes, they may be used to:</p>
            <ul>
              <li>Enable navigation between pages;</li>
              <li>Keep products in the shopping cart;</li>
              <li>Process orders;</li>
              <li>Maintain website security;</li>
              <li>Prevent fraudulent use;</li>
              <li>Save cookie consent preferences.</li>
            </ul>
            <p>Because these cookies are necessary for the website to operate or to provide features requested by the user, their use does not generally require consent.</p>

            <h3>Analytics and performance cookies</h3>
            <p>We use Google Analytics to obtain statistical information about how visitors use the website. This information may include:</p>
            <ul>
              <li>Number of visitors;</li>
              <li>Pages visited;</li>
              <li>Approximate duration of visits;</li>
              <li>Traffic source;</li>
              <li>Type of device or browser used;</li>
              <li>Interactions carried out on the website.</li>
            </ul>
            <p>This information helps us understand how the online store is used, identify problems and improve its performance, content and browsing experience.</p>

            <h3>Advertising and marketing cookies</h3>
            <p>We use Google Ads to promote our products and assess the performance of our advertising campaigns. Technologies associated with Google Ads may be used to:</p>
            <ul>
              <li>Measure conversions resulting from advertisements;</li>
              <li>Determine whether a purchase or other action occurred after an interaction with an advertisement;</li>
              <li>Assess campaign performance;</li>
              <li>Limit or manage the display of advertising;</li>
              <li>Show more relevant advertising where authorised.</li>
            </ul>
            <p>Google may use cookies and advertising identifiers, including cookies whose names begin with <code>gcl</code>, to measure actions carried out after an interaction with advertising, among other purposes.</p>
            <p>The use of cookies for advertising, personalisation or measurement is subject to user consent whenever legally required.</p>
          </PolicySection>

          <PolicySection number="03" title="Google Analytics">
            <p>Our website uses Google Analytics, a service provided by Google, to analyse website usage.</p>
            <p>This service may collect information about how users interact with the website, allowing us to produce aggregated statistics and improve the operation of the online store.</p>
            <p>Google states that Google Analytics uses cookies to collect usage statistics on websites where the service is implemented.</p>
            <p>Where required, Google Analytics will be activated in accordance with the consent preferences selected by the user.</p>
          </PolicySection>

          <PolicySection number="04" title="Google Ads">
            <p>We also use Google Ads to promote our products and measure the effectiveness of advertising campaigns run through Google services.</p>
            <p>Google Ads may use cookies and similar technologies to measure interactions with advertisements and conversions completed on our website.</p>
            <p>Depending on the user’s choices, these technologies may also be used to personalise advertising.</p>
            <p>The use of these technologies is subject to the consent preferences selected by the user.</p>
          </PolicySection>

          <PolicySection number="05" title="Instagram and Meta services">
            <p>We maintain a presence on Instagram, a platform owned by Meta.</p>
            <p>If our website only contains links to our Instagram page, the platform is accessed only when the user chooses to follow the relevant link, and the use of Instagram is then subject to Meta’s own policies.</p>
            <p>If embedded Instagram content, social plugins, the Meta Pixel or other technologies provided by Meta are used, those technologies may collect information about website usage and use cookies or similar identifiers.</p>
            <p>Where these technologies are not strictly necessary, their use will depend on the user’s prior consent.</p>
          </PolicySection>

          <PolicySection number="06" title="Third-party cookies">
            <p>Some cookies used on the website may be placed or managed by third parties, including:</p>
            <ul>
              <li>Google Ireland Limited / Google LLC, through Google Analytics and Google Ads;</li>
              <li>Meta Platforms, where features, embedded content or technologies related to Instagram or other Meta services are used;</li>
              <li>Other technology providers required for the operation of the online store.</li>
            </ul>
            <p>These organisations may process information in accordance with their own privacy and cookie policies.</p>
          </PolicySection>

          <PolicySection number="07" title="Cookie duration">
            <p>Cookies can be classified as:</p>
            <ul>
              <li><strong>Session cookies:</strong> normally deleted when the user closes their browser;</li>
              <li><strong>Persistent cookies:</strong> remain stored on the device for a set period or until deleted by the user.</li>
            </ul>
            <p>Their duration varies according to the type of cookie and the service that places it.</p>
            <p>Some cookies used by Google for advertising may have different retention periods depending on their purpose and the user’s location.</p>
          </PolicySection>

          <PolicySection number="08" title="Cookie consent and management">
            <p>On their first visit to the website, users may see a cookie management panel or banner. Through this tool, where applicable, they may:</p>
            <ul>
              <li>Accept all cookies;</li>
              <li>Reject non-essential cookies;</li>
              <li>Choose individually which cookie categories to authorise;</li>
              <li>Change their preferences at a later date.</li>
            </ul>
            <p>Cookies that require consent will not be activated before the user makes a valid choice.</p>
            <p>Users must be able to withdraw consent as easily as they gave it.</p>
            <p>Preferences may be changed at any time through the “Manage Cookies” option.</p>
          </PolicySection>

          <PolicySection number="09" title="Google Consent Mode">
            <p>The website may use Google Consent Mode to communicate the user’s consent choices to Google.</p>
            <p>This system allows Google Analytics and Google Ads tags to adjust their behaviour according to the selected privacy preferences.</p>
            <p>Consent Mode does not replace the cookie banner or the mechanism used to request the user’s consent.</p>
          </PolicySection>

          <PolicySection number="10" title="Managing cookies through your browser">
            <p>Users may also configure their browser to block or delete cookies.</p>
            <p>These settings are usually available in the browser’s privacy or security options.</p>
            <p>Disabling certain cookies may affect some website features, particularly where cookies are necessary for the online store to function.</p>
          </PolicySection>

          <PolicySection number="11" title="Personal data">
            <p>Some information collected through cookies or similar technologies may constitute personal data, including:</p>
            <ul>
              <li>IP address;</li>
              <li>Online identifiers;</li>
              <li>Browser or device information;</li>
              <li>Website interaction data;</li>
              <li>Information related to advertising campaigns.</li>
            </ul>
            <p>Whenever personal data is processed, it will be handled in accordance with applicable data protection legislation and our Privacy Policy.</p>
          </PolicySection>

          <PolicySection number="12" title="Changes to this Cookies Policy">
            <p>KnitnPrint may update this Cookies Policy whenever necessary, including as a result of legislative or technological changes or changes to the services used on the website.</p>
            <p>We recommend checking this page periodically.</p>
            <p>The date of the latest update will be shown at the beginning of the document.</p>
          </PolicySection>

          <PolicySection number="13" title="Contact us">
            <p>For any questions about this Cookies Policy or the protection of your personal data, please contact KnitnPrint at <a href="mailto:support@knitnprint.com">support@knitnprint.com</a>.</p>
          </PolicySection>
          </> : <LocalizedCookies content={localized} label={t('cookies.label')} />}
        </article>
      </div>
      <ContextualFaqs
        id="cookies-faqs"
        eyebrow={t('cookies.faqEyebrow')}
        title={t('cookies.faqTitle')}
        items={locale === 'en' ? [
          { question: 'What are cookies?', answer: 'Cookies are small files stored by a website on your browser or device to support functionality, remember preferences and understand website use.' },
          { question: 'Which cookies does KnitnPrint use?', answer: 'The website may use necessary, preference, analytics and advertising cookies, as described in the complete Cookies Policy above.' },
          { question: 'Can I reject optional cookies?', answer: 'Yes. Where a consent panel is available, optional cookies should remain inactive until you make a valid choice.' },
          { question: 'How can I change my cookie preferences?', answer: 'Use the Manage Cookies option when available, or review the privacy and cookie controls in your browser settings.' },
          { question: 'Will the website still work if I reject cookies?', answer: 'Necessary cookies support essential store functions. Rejecting optional cookies should not prevent those functions, although some additional features may be affected.' },
        ] : localized.faqs}
        className="contextual-faqs--policy"
      />
    </ContentPage>
  )
}

function LocalizedCookies({ content, label }: Readonly<{
  content: typeof cookiesPt
  label: string
}>) {
  return <>
    <div className="policy-opening policy-opening--first">
      {content.opening.map((paragraph) => <p key={paragraph}>{paragraph}</p>)}
    </div>
    <p className="eyebrow policy-document-label">{label}</p>
    {content.sections.map((section) => (
      <PolicySection number={section.number} title={section.title} key={section.number}>
        {section.blocks.map((block, index) => block.type === 'p' ? (
          <p key={index}>{renderCookieText(block.text)}</p>
        ) : block.type === 'h3' ? (
          <h3 key={index}>{block.text}</h3>
        ) : (
          <ul key={index}>{block.items.map((item) => <li key={item}>{item}</li>)}</ul>
        ))}
      </PolicySection>
    ))}
  </>
}

function renderCookieText(text: string) {
  return text.split('[email]').map((part, index) => index === 0 ? part : <span key={index}><a href="mailto:support@knitnprint.com">support@knitnprint.com</a>{part}</span>)
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
