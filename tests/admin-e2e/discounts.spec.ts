import { expect, test } from '@playwright/test'

test('a background refresh during discount creation keeps one record per discount', async ({ page }) => {
  await page.clock.install()
  const discount = {
    id: '00000000-0000-4000-8000-000000000001', code: 'RACE10', kind: 'percentage',
    value: 1000, currency: 'EUR', minimum_order_minor: 0, status: 'active',
    usage_count: 0, starts_at: null, ends_at: null, usage_limit: null,
    per_customer_limit: null, created_at: '2026-10-28T00:00:00Z',
  }
  let records: typeof discount[] = []
  let finishCreate = () => { throw new Error('The create request has not started') }
  await page.route('**/api/admin/auth/me', route => route.fulfill({ json: {
    id: 'owner-test', email: 'owner@example.test', display_name: 'Test Owner',
    role: 'owner', capabilities: ['discounts.manage'],
  } }))
  await page.route('**/api/admin/discounts', async route => {
    if (route.request().method() === 'POST') {
      records = [discount]
      await new Promise<void>(resolve => { finishCreate = resolve })
      await route.fulfill({ status: 201, json: discount })
    } else {
      await route.fulfill({ json: records })
    }
  })
  await page.goto('/#discounts')
  await expect(page.getByText('No discount codes yet', { exact: true })).toBeVisible()
  await page.getByLabel('Code', { exact: true }).fill(discount.code)
  await page.getByRole('spinbutton', { name: 'Percentage', exact: true }).fill('10')
  await page.getByRole('textbox', { name: 'Audit reason', exact: true }).fill('Create race regression promotion')
  await page.getByRole('button', { name: 'Create discount', exact: true }).click()
  await expect(page.getByRole('button', { name: 'Creating…', exact: true })).toBeVisible()
  // The list request sees the committed discount before the create response
  // arrives. The mutation must replace that record instead of prepending it twice.
  await page.clock.fastForward(15_000)
  const record = page.getByRole('article').filter({ hasText: discount.code })
  await expect(record).toHaveCount(1)
  finishCreate()
  await expect(page.getByRole('button', { name: 'Create discount', exact: true })).toBeEnabled()
  await expect(record).toHaveCount(1)
})
