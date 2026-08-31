import { useEffect, type ReactNode } from 'react'
import {
  HeadContent,
  Outlet,
  Scripts,
  createRootRoute,
  useRouterState,
} from '@tanstack/react-router'
import '../styles.css'
import { useI18n } from '../i18n'
import type { TranslationKey } from '../i18n/locales/en'

export const Route = createRootRoute({
  head: () => ({
    meta: [
      { charSet: 'utf-8' },
      { name: 'viewport', content: 'width=device-width, initial-scale=1' },
      {
        name: 'description',
        content:
          'KnitnPrint creates thoughtful objects where soft craft meets precise 3D printing.',
      },
      { title: 'KnitnPrint — Made between yarn and form' },
    ],
    links: [{ rel: 'icon', type: 'image/webp', href: '/knitprint-mark.webp' }],
  }),
  component: Root,
  notFoundComponent: NotFoundPage,
})

function Root() {
  return <LocalizedDocument />
}

function LocalizedDocument() {
  const { locale, t } = useI18n()
  return (
    <Document locale={locale} skipLinkLabel={t('common.skipToContent')}>
      <LocalizedMetadata />
      <Outlet />
    </Document>
  )
}

const metadataByPath: Record<string, { title: TranslationKey; description: TranslationKey }> = {
  '/': { title: 'home.heroTitle', description: 'home.storyBody' },
  '/products': { title: 'catalog.title', description: 'catalog.intro' },
  '/collections': { title: 'collections.title', description: 'collections.intro' },
  '/about': { title: 'about.title', description: 'about.intro' },
  '/b2b': { title: 'b2b.title', description: 'b2b.intro' },
  '/personalized-gifts': { title: 'gifts.title', description: 'gifts.intro' },
  '/discounts': { title: 'discount.title', description: 'discount.intro' },
  '/faq': { title: 'faq.title', description: 'faq.intro' },
  '/terms': { title: 'terms.title', description: 'terms.intro' },
  '/privacy': { title: 'privacy.title', description: 'privacy.intro' },
  '/cookies': { title: 'cookies.title', description: 'cookies.intro' },
  '/returns': { title: 'returns.title', description: 'returns.intro' },
  '/cart': { title: 'cart.title', description: 'cart.emptyBody' },
  '/account': { title: 'account.guestTitle', description: 'account.guestIntro' },
}

function LocalizedMetadata() {
  const { t } = useI18n()
  const pathname = useRouterState({ select: (state) => state.location.pathname })

  useEffect(() => {
    const metadata = metadataByPath[pathname]
    if (pathname === '/our-process') {
      document.title = `${t('process.title1')} ${t('process.title2')} — KnitnPrint`
      updateMetaDescription(t('process.intro'))
      return
    }
    if (!metadata) return
    document.title = `${t(metadata.title)} — KnitnPrint`
    updateMetaDescription(t(metadata.description))
  }, [pathname, t])

  return null
}

function updateMetaDescription(content: string) {
  let element = document.querySelector<HTMLMetaElement>('meta[name="description"]')
  if (!element) {
    element = document.createElement('meta')
    element.name = 'description'
    document.head.append(element)
  }
  element.content = content
}

function NotFoundPage() {
  const { t } = useI18n()
  return (
    <main className="empty-page">
      <p className="eyebrow">404</p>
      <h1>{t('errors.notFoundTitle')}</h1>
      <a className="button button--primary" href="/">{t('errors.backToShop')}</a>
    </main>
  )
}

function Document({ children, locale, skipLinkLabel }: Readonly<{
  children: ReactNode
  locale: string
  skipLinkLabel: string
}>) {
  return (
    <html lang={locale}>
      <head>
        <HeadContent />
      </head>
      <body>
        <a className="skip-link" href="#main-content">
          {skipLinkLabel}
        </a>
        {children}
        <Scripts />
      </body>
    </html>
  )
}
