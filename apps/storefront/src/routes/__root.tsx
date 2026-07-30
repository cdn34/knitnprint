import type { ReactNode } from 'react'
import {
  HeadContent,
  Outlet,
  Scripts,
  createRootRoute,
} from '@tanstack/react-router'
import '../styles.css'

export const Route = createRootRoute({
  head: () => ({
    meta: [
      { charSet: 'utf-8' },
      { name: 'viewport', content: 'width=device-width, initial-scale=1' },
      {
        name: 'description',
        content:
          'KnitPrint creates thoughtful objects where soft craft meets precise 3D printing.',
      },
      { title: 'KnitPrint — Made between yarn and form' },
    ],
    links: [{ rel: 'icon', type: 'image/webp', href: '/knitprint-mark.webp' }],
  }),
  component: Root,
  notFoundComponent: () => (
    <main className="empty-page">
      <p className="eyebrow">404</p>
      <h1>That thread leads nowhere.</h1>
      <a className="button button--primary" href="/">
        Back to the shop
      </a>
    </main>
  ),
})

function Root() {
  return (
    <Document>
      <Outlet />
    </Document>
  )
}

function Document({ children }: Readonly<{ children: ReactNode }>) {
  return (
    <html lang="en">
      <head>
        <HeadContent />
      </head>
      <body>
        <a className="skip-link" href="#main-content">
          Skip to content
        </a>
        {children}
        <Scripts />
      </body>
    </html>
  )
}
