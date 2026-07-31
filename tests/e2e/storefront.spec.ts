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
  await expect(page.locator('.product-grid')).toBeVisible()
  expect(
    (await page.locator('.product-card').count()) +
      (await page.locator('.storefront-empty').count()),
  ).toBeGreaterThan(0)
})

test('filters published products when the catalog is available', async ({
  page,
}) => {
  await page.goto('/')
  await page.waitForLoadState('networkidle')
  const cards = page.locator('.product-card')
  if ((await cards.count()) === 0) return

  const title = await cards.first().getByRole('heading').textContent()
  await page.getByLabel('Search the catalog').fill(title ?? '')
  await expect(cards).toHaveCount(1)
})

test('opens a published product detail page when available', async ({ page }) => {
  await page.goto('/')
  const card = page.locator('.product-card').first()
  if ((await card.count()) === 0) return

  const title = await card.getByRole('heading').textContent()
  await card.getByRole('link', { name: `View ${title}` }).click()
  await expect(page.getByRole('heading', { level: 1 })).toHaveText(title ?? '')
  await expect(page.getByText(/SKU /)).toBeVisible()
})

test('navigates live catalog collections when available', async ({ page }) => {
  await page.goto('/')
  const collectionLink = page.locator('#collections a').first()
  if ((await collectionLink.count()) === 0) return

  const collectionName = (await collectionLink.getAttribute('aria-label'))
    ?.replace(/^Shop /, '')
    .replace(/ collection$/, '')
  await collectionLink.click()

  await expect(page).toHaveURL(/\/collections\/[a-z0-9-]+$/)
  await expect(page.getByRole('heading', { level: 1 })).toHaveText(
    collectionName ?? '',
  )
  await expect(page.locator('.product-card').first()).toBeVisible()
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
