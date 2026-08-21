import type { ReactNode } from 'react'
import {
  StorefrontAnnouncement,
  StorefrontFooter,
  StorefrontHeader,
} from './storefront-shell'

export function ContentPage({
  eyebrow,
  title,
  intro,
  children,
  className = '',
}: Readonly<{
  eyebrow: string
  title: string
  intro: string
  children: ReactNode
  className?: string
}>) {
  return (
    <>
      <StorefrontAnnouncement />
      <StorefrontHeader />
      <main
        className={`content-page ${className}`.trim()}
        id="main-content"
        tabIndex={-1}
      >
        <header className="content-page-intro">
          <p className="eyebrow">{eyebrow}</p>
          <h1>{title}</h1>
          <p>{intro}</p>
        </header>
        {children}
      </main>
      <StorefrontFooter />
    </>
  )
}

export function PolicyPlaceholder({
  eyebrow,
  title,
  intro,
  topics,
}: Readonly<{
  eyebrow: string
  title: string
  intro: string
  topics: string[]
}>) {
  return (
    <ContentPage eyebrow={eyebrow} title={title} intro={intro} className="policy-page">
      <div className="policy-layout">
        <aside className="policy-status">
          <span>Draft page</span>
          <strong>Content to be added</strong>
          <p>This structure is ready for the final approved text.</p>
        </aside>
        <article className="policy-document">
          <p className="eyebrow">Planned structure</p>
          {topics.map((topic, index) => (
            <section key={topic}>
              <span>{String(index + 1).padStart(2, '0')}</span>
              <div>
                <h2>{topic}</h2>
                <p>The final wording for this section will be added here.</p>
              </div>
            </section>
          ))}
        </article>
      </div>
    </ContentPage>
  )
}
