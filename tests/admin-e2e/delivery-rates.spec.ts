import { expect, test } from '@playwright/test'

test('country-wide prices need no postal setup and hidden regional exceptions are preserved', async ({ page }) => {
  let saved = {
    enabled: false, revision: 0, currency: 'EUR', country_codes: ['PT', 'ES', 'FR'], updated_at: '2026-10-27T12:00:00Z',
    rates: [
      { id: '00000000-0000-4000-8000-000000000001', country_code: 'PT', region_name: 'Mainland Portugal', postal_prefixes: ['1', '2', '3', '4', '5', '6', '7', '8'], amount_minor: null as number | null, active: false },
      { id: '00000000-0000-4000-8000-000000000002', country_code: 'PT', region_name: 'Madeira', postal_prefixes: ['90', '91', '92', '93', '94'], amount_minor: null as number | null, active: false },
      { id: '00000000-0000-4000-8000-000000000003', country_code: 'PT', region_name: 'Azores', postal_prefixes: ['95', '96', '97', '98', '99'], amount_minor: null as number | null, active: false },
      { id: '00000000-0000-4000-8000-000000000004', country_code: 'ES', region_name: 'All regions', postal_prefixes: [], amount_minor: null as number | null, active: false },
    ],
  }
  const regionalRules = saved.rates.slice(0, 3).map(rate => [...rate.postal_prefixes])
  await page.route('**/api/admin/auth/me', route => route.fulfill({ json: { id: 'test-owner', email: 'owner@example.test', display_name: 'Test Owner', role: 'owner', capabilities: ['settings.manage'] } }))
  await page.route('**/api/admin/delivery-rates', async route => {
    if (route.request().method() === 'PUT') saved = { ...saved, ...route.request().postDataJSON(), revision: saved.revision + 1 }
    await route.fulfill({ json: saved })
  })
  await page.goto('/#delivery-rates')
  await expect(page.getByRole('heading', { name: 'Shipping rates', exact: true })).toBeVisible()
  await expect(page.getByRole('columnheader', { name: 'Postal prefixes' })).toHaveCount(0)
  await expect(page.getByLabel('Postal prefixes for destination 1', { exact: true })).toBeHidden()
  await page.getByRole('button', { name: 'Add destination', exact: true }).click()
  await page.getByLabel('Country for destination 5', { exact: true }).selectOption('FR')
  await expect(page.getByLabel('Region for destination 5', { exact: true })).toHaveValue('All regions')
  // No region or postal fields need to be filled to create a country-wide rate.
  await page.getByLabel('Price for destination 5', { exact: true }).fill('7,90')
  await page.getByLabel('Activate destination 5', { exact: true }).check()
  await page.getByRole('button', { name: 'Save changes', exact: true }).click()
  await expect(page.getByText('Shipping destinations and prices saved.', { exact: true })).toBeVisible()
  expect(saved.rates[4]).toMatchObject({ country_code: 'FR', region_name: 'All regions', postal_prefixes: [], amount_minor: 790, active: true })
  expect(saved.rates.slice(0, 3).map(rate => rate.postal_prefixes)).toEqual(regionalRules)

  const advanced = page.getByText('Advanced configuration for destination 2', { exact: true })
  await advanced.click()
  await expect(page.getByLabel('Postal prefixes for destination 2', { exact: true })).toHaveValue('90, 91, 92, 93, 94')
  await advanced.click()
  await expect(page.getByRole('button', { name: 'Save changes', exact: true })).toBeDisabled()
  await advanced.click()
  await page.getByLabel('Postal prefixes for destination 2', { exact: true }).fill('90, 91, 92, 93, 94, 900')
  await advanced.click()
  await page.getByLabel('Region for destination 5', { exact: true }).fill('')
  await page.getByRole('button', { name: 'Save changes', exact: true }).click()
  await expect(page.getByRole('button', { name: 'Save changes', exact: true })).toBeDisabled()
  expect(saved.rates[1].postal_prefixes).toContain('900')
  expect(saved.rates[4].region_name).toBe('All regions')

  await page.screenshot({ path: test.info().outputPath('desktop.png'), fullPage: true })
  await page.setViewportSize({ width: 375, height: 900 })
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(375)
  await page.evaluate(() => window.scrollTo(0, 0))
  await page.screenshot({ path: test.info().outputPath('mobile.png'), fullPage: true })
})
