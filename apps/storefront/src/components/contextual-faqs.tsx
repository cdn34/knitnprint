import { useI18n } from '../i18n'

export type ContextualFaq = Readonly<{
  question: string
  answer: string
}>

export function ContextualFaqs({
  id,
  eyebrow,
  title,
  items,
  className = '',
}: Readonly<{
  id: string
  eyebrow: string
  title: string
  items: readonly ContextualFaq[]
  className?: string
}>) {
  const { t } = useI18n()
  return (
    <section
      className={`contextual-faqs ${className}`.trim()}
      aria-labelledby={`${id}-title`}
    >
      <div className="contextual-faqs-heading">
        <p className="eyebrow">{eyebrow}</p>
        <h2 id={`${id}-title`}>{title}</h2>
        <a className="text-link" href="/faq">{t('common.viewAllFaqs')}</a>
      </div>
      <div className="contextual-faqs-list">
        {items.map(({ question, answer }) => (
          <details key={question} name={id}>
            <summary><span>{question}</span><span aria-hidden="true" /></summary>
            <p>{answer}</p>
          </details>
        ))}
      </div>
    </section>
  )
}
