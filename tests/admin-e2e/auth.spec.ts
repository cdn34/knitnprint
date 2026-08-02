import AxeBuilder from '@axe-core/playwright'
import { expect, test } from '@playwright/test'

const email = process.env.E2E_OWNER_EMAIL ?? 'owner@knitprint.local'
const password =
  process.env.E2E_OWNER_PASSWORD ?? 'local-development-passphrase'

test('protects the admin shell with a persistent staff session', async ({
  page,
}) => {
  await page.goto('/')

  await expect(
    page.getByRole('heading', { name: 'Welcome back.' }),
  ).toBeVisible()
  const loginScan = await new AxeBuilder({ page })
    .withTags(['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa'])
    .analyze()
  expect(loginScan.violations).toEqual([])

  await page.getByLabel('Email address').fill(email)
  await page.getByLabel('Password').fill(password)
  await page.getByRole('button', { name: 'Sign in' }).click()

  await expect(page.getByRole('heading', { name: /Good to see you/ })).toBeVisible()
  await page.reload()
  await expect(page.getByRole('heading', { name: /Good to see you/ })).toBeVisible()
  const metrics = page.getByRole('region', { name: 'Inventory metrics' })
  await expect(
    metrics
      .getByRole('article')
      .filter({ hasText: 'Low stock' })
      .locator('strong'),
  ).not.toHaveText('—')
  await expect(page.getByRole('region', { name: 'Low-stock variants' })).toBeVisible()

  const dashboardScan = await new AxeBuilder({ page })
    .withTags(['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa'])
    .analyze()
  expect(dashboardScan.violations).toEqual([])

  await page.getByRole('button', { name: 'Sign out' }).click()
  await expect(
    page.getByRole('heading', { name: 'Welcome back.' }),
  ).toBeVisible()
})
