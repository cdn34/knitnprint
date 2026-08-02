import AxeBuilder from '@axe-core/playwright'
import { expect, test } from '@playwright/test'

const ownerEmail = process.env.E2E_OWNER_EMAIL ?? 'owner@knitprint.local'
const ownerPassword =
  process.env.E2E_OWNER_PASSWORD ?? 'local-development-passphrase'

test('lets authorized staff search and inspect a guest customer', async ({
  page,
}) => {
  const unique = `${Date.now()}-${test.info().retry}`
  const firstName = 'Marta'
  const lastName = `Needle ${unique}`
  const customerEmail = `marta-${unique}@example.com`
  const recipientName = `${firstName} ${lastName}`

  const fixture = await page.request.post(
    'http://127.0.0.1:8080/api/customers/guest',
    {
      headers: { 'idempotency-key': `customer-browser-${unique}` },
      data: {
        email: customerEmail,
        first_name: firstName,
        last_name: lastName,
        phone: '+351 912 345 678',
        address: {
          recipient_name: recipientName,
          line1: '24 Rua das Malhas',
          line2: 'Atelier 3',
          city: 'Porto',
          region: 'Porto',
          postal_code: '4000-123',
          country_code: 'PT',
          phone: '+351 912 345 678',
        },
      },
    },
  )
  expect(fixture.status()).toBe(201)

  await page.goto('/')
  await page.getByLabel('Email address').fill(ownerEmail)
  await page.getByLabel('Password').fill(ownerPassword)
  await page.getByRole('button', { name: 'Sign in' }).click()
  await page.getByRole('link', { name: 'Customers' }).click()

  const customers = page.getByRole('region', { name: 'Customers' })
  await expect(customers).toBeVisible()
  await customers.getByLabel('Search customers').fill(customerEmail)

  const customerRow = customers
    .getByRole('button')
    .filter({ hasText: customerEmail })
  await expect(customerRow).toBeVisible()
  await expect(customerRow).toContainText(recipientName)
  await expect(customerRow).toContainText('1 address')
  await customerRow.click()

  const detail = customers.getByRole('region', { name: recipientName })
  await expect(detail).toBeVisible()
  await expect(detail.getByRole('link', { name: customerEmail })).toBeVisible()
  await expect(detail).toContainText('+351 912 345 678')
  await expect(
    detail.getByRole('article', { name: `Address for ${recipientName}` }),
  ).toContainText('24 Rua das Malhas')
  await expect(detail).toContainText('Porto, Porto, 4000-123')
  await expect(
    detail.getByRole('region', { name: 'Order history' }),
  ).toContainText('No orders yet')

  const accessibility = await new AxeBuilder({ page })
    .withTags(['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa'])
    .analyze()
  expect(accessibility.violations).toEqual([])

  await page.setViewportSize({ width: 390, height: 844 })
  await expect(detail).toBeVisible()
  const dimensions = await page.evaluate(() => ({
    viewport: document.documentElement.clientWidth,
    content: document.documentElement.scrollWidth,
  }))
  expect(dimensions.content).toBeLessThanOrEqual(dimensions.viewport)
})
