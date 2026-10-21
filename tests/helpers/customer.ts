import { expect, type APIRequestContext } from '@playwright/test'

export async function signInVerifiedCustomer(
  request: APIRequestContext,
  email: string,
  firstName: string,
  lastName: string,
) {
  const origin = 'http://127.0.0.1:3000'
  const password = 'browser-customer-passphrase'
  const registration = await request.post(`${origin}/api/account/register`, {
    data: { email, password, first_name: firstName, last_name: lastName },
  })
  expect(registration.status()).toBe(201)

  const emailResponse = await request.get(
    `${origin}/api/development/emails/latest?to=${encodeURIComponent(email)}&kind=email_verification`,
  )
  expect(emailResponse.ok()).toBe(true)
  const message = await emailResponse.json() as { action_url: string }
  const token = new URL(message.action_url).searchParams.get('verify')
  expect(token).toBeTruthy()
  const confirmation = await request.post(`${origin}/api/account/verification/confirm`, {
    data: { token },
  })
  expect(confirmation.status()).toBe(204)

  const login = await request.post(`${origin}/api/account/login`, {
    data: { email, password },
  })
  expect(login.ok()).toBe(true)
}
