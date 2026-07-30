import AxeBuilder from '@axe-core/playwright'
import { expect, test } from '@playwright/test'

test('renders the branded storefront shell', async ({ page }) => {
  await page.goto('/')

  await expect(page).toHaveTitle(/KnitPrint/)
  await expect(
    page.getByRole('heading', {
      level: 1,
      name: 'Soft ideas, shaped into lasting objects.',
    }),
  ).toBeVisible()
  await expect(page.getByRole('link', { name: 'KnitPrint home' })).toBeVisible()
  await expect(
    page.getByRole('heading', { name: 'Objects with a softer edge' }),
  ).toBeVisible()
  await expect(page.locator('.product-card')).toHaveCount(4)
})

test('has no detectable WCAG A or AA violations', async ({ page }) => {
  await page.goto('/')

  const results = await new AxeBuilder({ page })
    .withTags(['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa'])
    .analyze()

  expect(results.violations).toEqual([])
})

test('supports keyboard navigation to main content', async ({ page }) => {
  await page.goto('/')
  await page.keyboard.press('Tab')

  const skipLink = page.getByRole('link', { name: 'Skip to content' })
  await expect(skipLink).toBeFocused()
  await skipLink.press('Enter')
  await expect(page.locator('#main-content')).toBeFocused()
})

test('does not overflow the viewport', async ({ page }) => {
  await page.goto('/')

  const dimensions = await page.evaluate(() => ({
    viewport: document.documentElement.clientWidth,
    content: document.documentElement.scrollWidth,
  }))

  expect(dimensions.content).toBeLessThanOrEqual(dimensions.viewport)
})

