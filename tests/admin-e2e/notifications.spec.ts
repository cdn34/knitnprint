import AxeBuilder from '@axe-core/playwright'
import { expect, test, type Page } from '@playwright/test'

const job = {
  id: '01900000-0000-7000-8000-000000000001',
  order_id: '01900000-0000-7000-8000-000000000002',
  order_number: 'KP-1042',
  recipient_email: 'customer@example.com',
  kind: 'order_confirmation', status: 'pending', attempt_count: 1,
  last_error: 'Email provider temporarily unavailable',
  created_at: '2026-10-21T10:00:00Z', next_attempt_at: '2026-10-21T11:00:00Z',
  sent_at: null, can_send: true,
}

async function profile(page: Page, canSend = true) {
  await page.route('**/api/admin/auth/me', (route) => route.fulfill({ json: {
    id: '01900000-0000-7000-8000-000000000003', email: 'staff@example.com',
    display_name: 'Support Staff', role: 'staff',
    capabilities: ['notifications.read', 'orders.read', ...(canSend ? ['notifications.send'] : [])],
  } }))
}

test('searches by order number and manually sends an outstanding notification', async ({ page }) => {
  await profile(page)
  let sent = false
  await page.route('**/api/admin/notifications**', async (route) => {
    if (route.request().method() === 'POST') {
      expect(route.request().url()).toContain(`/notifications/${job.id}/send`)
      expect(route.request().postDataJSON()).toEqual({ reason: 'Customer reported missing confirmation' })
      sent = true
      await route.fulfill({ json: { ...job, status: 'sent', can_send: false, sent_at: job.next_attempt_at, last_error: null } })
    } else {
      await route.fulfill({ json: { items: sent ? [] : [job], total: sent ? 0 : 1, page: 1, page_size: 25 } })
    }
  })
  await page.goto('/#notifications')
  await expect(page.getByRole('heading', { name: 'Email notifications', exact: true })).toBeVisible()
  await page.getByLabel('Search', { exact: true }).fill('KP-1042')
  const searched = page.waitForRequest((request) => request.method() === 'GET' && request.url().includes('q=KP-1042'))
  await page.getByRole('button', { name: 'Search notifications' }).click()
  await searched
  await expect(page.getByRole('link', { name: 'KP-1042' })).toHaveAttribute('href', `#orders/${job.order_id}`)
  await page.getByRole('button', { name: 'Send order confirmation to customer@example.com' }).click()
  await page.getByLabel('Reason for sending').fill('Customer reported missing confirmation')
  await page.getByRole('button', { name: 'Send email now' }).click()
  await expect(page.getByRole('status').filter({ hasText: 'Email sent to customer@example.com.' })).toBeVisible()
  await expect(page.getByText('No notifications match these filters.')).toBeVisible()
  expect(sent).toBe(true)
})

test('shows retry failure accurately and supports pagination without order context', async ({ page }) => {
  await profile(page)
  let queriedPage = 1
  await page.route('**/api/admin/notifications**', async (route) => {
    if (route.request().method() === 'POST') {
      await route.fulfill({ json: { ...job, attempt_count: 2 } })
    } else {
      queriedPage = Number(new URL(route.request().url()).searchParams.get('page') ?? 1)
      await route.fulfill({ json: {
        items: queriedPage === 1 ? [job] : [{ ...job, id: '01900000-0000-7000-8000-000000000004',
          order_id: null, order_number: null, recipient_email: 'signup@example.com', kind: 'signup_confirmation', can_send: false }],
        total: 26, page: queriedPage, page_size: 25,
      } })
    }
  })
  await page.goto('/#notifications')
  await page.getByRole('button', { name: 'Send order confirmation to customer@example.com' }).click()
  await page.getByLabel('Reason for sending').fill('Retry requested by customer')
  await page.getByRole('button', { name: 'Send email now' }).click()
  await expect(page.getByRole('status').filter({ hasText: 'It remains pending for the worker to retry.' })).toBeVisible()
  await page.getByRole('button', { name: 'Next', exact: true }).click()
  await expect(page.getByText('signup@example.com', { exact: true })).toBeVisible()
  await expect(page.getByText('Delivery unavailable')).toBeVisible()
  expect(queriedPage).toBe(2)
  const scan = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa']).analyze()
  expect(scan.violations).toEqual([])
})

test('read-only staff can inspect notifications without a send action', async ({ page }) => {
  await profile(page, false)
  await page.route('**/api/admin/notifications**', (route) => route.fulfill({ json: {
    items: [job], total: 1, page: 1, page_size: 25,
  } }))
  await page.goto('/#notifications')
  await expect(page.getByText('customer@example.com', { exact: true })).toBeVisible()
  await expect(page.getByRole('button', { name: /Send/ })).toHaveCount(0)
})
