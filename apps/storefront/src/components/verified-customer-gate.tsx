import { ApiError } from '@knitnprint/api-client'
import { MailCheck } from 'lucide-react'
import { useEffect, useState, type ReactNode } from 'react'
import { cartApi } from '../cart-api'
import { useI18n } from '../i18n'
import { StorefrontAnnouncement, StorefrontFooter, StorefrontHeader } from './storefront-shell'

type Access = 'checking' | 'guest' | 'unverified' | 'verified' | 'error'

export async function customerAccess(): Promise<Exclude<Access, 'checking' | 'error'>> {
  try {
    const profile = await cartApi.customerAccount()
    return profile.email_verified ? 'verified' : 'unverified'
  } catch (error) {
    if (error instanceof ApiError && error.status === 401) return 'guest'
    throw error
  }
}

export function VerifiedCustomerGate({ children }: Readonly<{ children: ReactNode }>) {
  const { t } = useI18n()
  const [access, setAccess] = useState<Access>('checking')

  useEffect(() => {
    let active = true
    customerAccess().then((result) => {
      if (active) setAccess(result)
    }).catch(() => {
      if (active) setAccess('error')
    })
    return () => { active = false }
  }, [])

  if (access === 'verified') return <>{children}</>

  return <>
    <StorefrontAnnouncement />
    <StorefrontHeader />
    <main className="account-page" id="main-content">
      <section className="account-panel account-access-gate" aria-live="polite">
        <MailCheck size={32} aria-hidden="true" />
        <h1>{t(access === 'unverified' ? 'account.verifyToContinue' : 'account.actionRequiredTitle')}</h1>
        <p>{t(access === 'unverified' ? 'account.actionUnverified' : access === 'error' ? 'account.genericError' : 'account.actionGuest')}</p>
        {access !== 'checking' && access !== 'error' && <a className="button button--primary" href="/account">{t('account.goToAccount')}</a>}
      </section>
    </main>
    <StorefrontFooter />
  </>
}
