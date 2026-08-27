import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from 'react'
import { en, type TranslationKey } from './locales/en'
import { es } from './locales/es'
import { pt } from './locales/pt'

export const supportedLocales = ['en', 'pt', 'es'] as const
export type Locale = (typeof supportedLocales)[number]

export const localeLabels: Record<Locale, string> = { en: 'English', pt: 'Português', es: 'Español' }
const dictionaries: Record<Locale, Record<TranslationKey, string>> = { en, pt, es }
const STORAGE_KEY = 'knitprint-language'

type TranslationValues = Record<string, string | number>
type I18nContextValue = {
  locale: Locale
  setLocale: (locale: Locale) => void
  t: (key: TranslationKey, values?: TranslationValues) => string
  formatCurrency: (minorAmount: number, currency: string) => string
}
const I18nContext = createContext<I18nContextValue | null>(null)

function resolveLocale(language: string | undefined): Locale | null {
  const normalized = language?.toLowerCase().split('-')[0]
  return supportedLocales.find((locale) => locale === normalized) ?? null
}

function detectBrowserLocale(): Locale {
  const stored = resolveLocale(window.localStorage.getItem(STORAGE_KEY) ?? undefined)
  if (stored) return stored
  for (const language of navigator.languages) {
    const locale = resolveLocale(language)
    if (locale) return locale
  }
  return resolveLocale(navigator.language) ?? 'en'
}

export function I18nProvider({ children }: Readonly<{ children: ReactNode }>) {
  const [locale, updateLocale] = useState<Locale>('en')
  useEffect(() => updateLocale(detectBrowserLocale()), [])
  useEffect(() => { document.documentElement.lang = locale }, [locale])

  const setLocale = useCallback((nextLocale: Locale) => {
    updateLocale(nextLocale)
    window.localStorage.setItem(STORAGE_KEY, nextLocale)
  }, [])
  const value = useMemo<I18nContextValue>(() => ({
    locale,
    setLocale,
    formatCurrency: (minorAmount, currency) => new Intl.NumberFormat(
      locale === 'pt' ? 'pt-PT' : locale === 'es' ? 'es-ES' : 'en-GB',
      { style: 'currency', currency },
    ).format(minorAmount / 100),
    t: (key, values) => {
      const translation = dictionaries[locale][key] ?? dictionaries.en[key]
      if (!values) return translation
      return Object.entries(values).reduce(
        (result, [name, value]) => result.replaceAll(`{${name}}`, String(value)),
        translation,
      )
    },
  }), [locale, setLocale])

  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>
}

export function useI18n() {
  const context = useContext(I18nContext)
  if (!context) throw new Error('useI18n must be used inside I18nProvider')
  return context
}
