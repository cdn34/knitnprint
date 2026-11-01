import { expect, test } from '@playwright/test'

test('a changed checkout quote refreshes the displayed address and total before retrying', async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem('knitnprint-language', 'en'))
  const address = { recipient_name: 'Ada Loom', line1: '9 Thread Street', line2: '', city: 'Lisbon', region: '', postal_code: '1000-009', country_code: 'PT', phone: '' }
  const profile = { id: 'customer-test', email: 'order@example.com', email_verified: true, first_name: 'Ada', last_name: 'Loom', phone: '', addresses: [] }
  const cart = {
    id: 'cart-test', currency: 'EUR', item_count: 1, checkout_ready: true,
    checkout_quote: 'reviewed-mainland-quote', subtotal_minor: 3200,
    discount_minor: 0, tax_minor: 0, shipping_minor: 450, total_minor: 3650,
    shipping_quote: { available: true, pricing_enabled: true, revision: 1, currency: 'EUR', amount_minor: 450 },
    issues: [], expires_at: '2030-01-01T00:00:00Z',
    delivery: { ...profile, customer_id: profile.id, address_id: 'address-test', address },
    items: [{ id: 'line-test', variant_id: 'variant-test', product_slug: 'test-shirt', product_title: 'Test shirt', variant_title: 'S', sku: 'TEST', quantity: 1, available: true, available_quantity: 10, currency: 'EUR', unit_price_minor: 3200, line_total_minor: 3200, customization_media_asset_ids: [] }],
  }
  const changed = {
    ...cart, checkout_quote: 'reviewed-madeira-quote', shipping_minor: 800, total_minor: 4000,
    shipping_quote: { ...cart.shipping_quote, amount_minor: 800 },
    delivery: { ...cart.delivery, address: { ...address, line1: '2 Island Street', city: 'Funchal', postal_code: '9000-001' } },
  }
  let rejected = false
  const submitted: Array<{ checkout_quote: string; shipping_rate_revision: number }> = []
  await page.route('**/api/account/me', route => route.fulfill({ json: profile }))
  await page.route('**/api/cart', route => route.fulfill({ json: rejected ? changed : cart }))
  await page.route('**/api/payments/options', route => route.fulfill({ json: { stripe: false, manual: true, checkout_suspended: false } }))
  await page.route('**/api/orders', async route => {
    submitted.push(route.request().postDataJSON())
    rejected = true
    await route.fulfill({ status: 409, json: { error: { code: 'cart_changed', message: 'Review the updated cart.' } } })
  })
  await page.goto('/cart')
  await expect(page.locator('.cart-total')).toContainText('€36.50')
  await expect(page.getByLabel('Address', { exact: true })).toHaveValue('9 Thread Street')
  await page.getByRole('button', { name: 'Place order', exact: true }).click()
  await expect(page.getByText('Delivery details or prices changed while you were checking out. Review the updated address and total before trying again.', { exact: true })).toBeVisible()
  await expect(page.locator('.cart-total')).toContainText('€40.00')
  await expect(page.getByLabel('Address', { exact: true })).toHaveValue('2 Island Street')
  await expect(page.getByLabel('Postal code', { exact: true })).toHaveValue('9000-001')
  expect(submitted[0]).toMatchObject({ checkout_quote: cart.checkout_quote, shipping_rate_revision: 1 })
  await page.getByRole('button', { name: 'Place order', exact: true }).click()
  await expect.poll(() => submitted.length).toBe(2)
  expect(submitted[1]).toMatchObject({ checkout_quote: changed.checkout_quote, shipping_rate_revision: 1 })
})
