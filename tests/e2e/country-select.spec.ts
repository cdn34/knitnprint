import { expect, test } from '@playwright/test'

for (const [locale, spainName] of [['en', 'Spain'], ['pt', 'Espanha'], ['es', 'España']]) {
  test(`delivery and account country selectors show ${locale} names and submit ISO codes`, async ({ page }) => {
    await page.addInitScript(language => localStorage.setItem('knitnprint-language', language), locale)
    const address = { id: 'address-test', address_type: 'delivery', recipient_name: 'Test Customer', line1: 'Calle Mayor 1', line2: '', city: 'Madrid', region: '', postal_code: '28001', country_code: 'ES', phone: '' }
    const profile = { id: 'customer-test', email: 'customer@example.test', email_verified: true, first_name: 'Test', last_name: 'Customer', phone: '', addresses: [address] }
    const cart = { id: 'cart-test', currency: 'EUR', item_count: 1, checkout_ready: false, subtotal_minor: 2000, discount_minor: 0, tax_minor: 0, shipping_minor: 0, total_minor: 2000, issues: [], expires_at: '2030-01-01T00:00:00Z', delivery: { email: profile.email, first_name: profile.first_name, last_name: profile.last_name, phone: '', customer_id: profile.id, address_id: address.id, address }, items: [{ id: 'line-test', variant_id: 'variant-test', product_slug: 'test-shirt', product_title: 'Test shirt', variant_title: 'S', sku: 'TEST', quantity: 1, available: true, available_quantity: 10, currency: 'EUR', unit_price_minor: 2000, line_total_minor: 2000, customization_media_asset_ids: [] }] }
    let submittedCountry = ''
    await page.route('**/api/account/me', route => route.fulfill({ json: profile }))
    await page.route('**/api/account/orders?**', route => route.fulfill({ json: { orders: [], total: 0, page: 1 } }))
    await page.route('**/api/cart', route => route.fulfill({ json: cart }))
    await page.route('**/api/payments/options', route => route.fulfill({ json: { stripe: false, manual: true, checkout_suspended: false } }))
    await page.route('**/api/cart/delivery', async route => {
      const input = route.request().postDataJSON()
      submittedCountry = input.address.country_code
      await route.fulfill({ json: { ...cart, delivery: { ...cart.delivery, ...input } } })
    })
    await page.route('**/api/account/addresses', async route => {
      const input = route.request().postDataJSON()
      submittedCountry = input.country_code
      await route.fulfill({ json: { ...address, ...input, id: 'new-address-test' } })
    })
    await page.route('**/api/account/addresses/address-test', async route => {
      const input = route.request().postDataJSON()
      submittedCountry = input.country_code
      await route.fulfill({ json: { ...address, ...input } })
    })
    await page.goto('/cart')
    await expect(page.locator('html')).toHaveAttribute('lang', locale)
    const country = page.locator('.delivery-form select[name="country_code"]')
    await expect(country).toHaveValue('ES')
    await expect(country).toHaveAccessibleName(locale === 'en' ? 'Country' : 'País')
    await expect(country.locator('option:checked')).toHaveText(spainName)
    await expect(country.locator('option')).toHaveCount(249)
    await expect(country).toHaveAttribute('autocomplete', 'country')
    await country.selectOption('FR')
    await page.locator('.delivery-form button[type="submit"]').click()
    await expect.poll(() => submittedCountry).toBe('FR')
    await page.screenshot({ path: test.info().outputPath('checkout.png'), fullPage: true })

    await page.goto('/account')
    const addForm = page.locator('form').filter({ has: page.locator('[name="address_type"]') }).last()
    await addForm.locator('[name="recipient_name"]').fill('Test Customer')
    await addForm.locator('[name="line1"]').fill('Calle Mayor 1')
    await addForm.locator('[name="city"]').fill('Madrid')
    await addForm.locator('[name="postal_code"]').fill('28001')
    await addForm.locator('select[name="country_code"]').selectOption('ES')
    await addForm.locator('button[type="submit"]').click()
    await expect.poll(() => submittedCountry).toBe('ES')
    const savedAddress = page.locator('.address-card').first()
    await savedAddress.locator('.address-card-top button').click()
    await expect(savedAddress.locator('select[name="country_code"]')).toHaveValue('ES')
    await savedAddress.locator('select[name="country_code"]').selectOption('FR')
    await savedAddress.locator('button[type="submit"]').click()
    await expect.poll(() => submittedCountry).toBe('FR')

    await page.setViewportSize({ width: 375, height: 900 })
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(375)
  })
}
