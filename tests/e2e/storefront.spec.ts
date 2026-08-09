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
  await expect(card.locator('.product-stock')).toBeVisible()
  await card.getByRole('link', { name: `View ${title}` }).click()
  await expect(page.getByRole('heading', { level: 1 })).toHaveText(title ?? '')
  await expect(page.getByRole('radio').first()).toBeVisible()
  await expect(page.getByRole('status')).toContainText(
    /In stock|Only \d+ left|Sold out/,
  )

  const availableVariants = page.locator(
    'input[type="radio"]:not(:disabled)',
  )
  if ((await availableVariants.count()) > 0) {
    await expect(availableVariants.first()).toBeChecked()
  }

  const results = await new AxeBuilder({ page })
    .withTags(['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa'])
    .analyze()
  expect(results.violations).toEqual([])
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

test('shows an accessible persistent cart surface', async ({ page }) => {
  await page.goto('/cart')

  await expect(page).toHaveTitle(/Your cart/)
  await expect(page.getByRole('heading', { level: 1, name: 'Cart' })).toBeVisible()
  await expect(
    page.getByRole('heading', { name: 'Your cart is waiting for its first piece.' }),
  ).toBeVisible()

  const results = await new AxeBuilder({ page })
    .withTags(['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa'])
    .analyze()
  expect(results.violations).toEqual([])
})

test('adds an available product, captures delivery, and creates an order', async ({
  page,
}) => {
  await page.goto('/')
  const card = page.locator('.product-card').first()
  if ((await card.count()) === 0) return
  await card.locator('.product-visual').click()

  if ((await page.locator('input[type="radio"]:not(:disabled)').count()) === 0) return
  const addButton = page.getByRole('button', { name: 'Add to cart' })
  await expect(addButton).toBeEnabled()
  await addButton.click()
  await page.getByRole('link', { name: 'View your cart' }).click()

  await expect(page.locator('.cart-item')).toHaveCount(1)
  await page.getByLabel('Email').fill('browser-cart@example.com')
  await page.getByLabel('First name').fill('Browser')
  await page.getByLabel('Last name').fill('Cart')
  await page.getByLabel('Recipient').fill('Browser Cart')
  await page.getByLabel('Address', { exact: true }).fill('12 Loom Lane')
  await page.getByLabel('City').fill('Lisbon')
  await page.getByLabel('Postal code').fill('1000-001')
  await page.getByRole('button', { name: 'Save delivery details' }).click()

  await expect(page.getByRole('status')).toContainText('Delivery details saved.')
  await expect(page.locator('.cart-ready-state')).toContainText(
    'Cart and delivery details are ready.',
  )
  await page.getByRole('button', { name: 'Create order' }).click()
  await expect(page.getByRole('heading', { level: 1, name: 'Order received' })).toBeVisible()
  await expect(page.getByRole('heading', { name: 'Thank you, Browser.' })).toBeVisible()
  await expect(page.locator('.order-confirmation')).toContainText(/KP-\d{4}-\d{6}/)
  await expect(page.locator('.order-confirmation')).toContainText(
    'awaiting manual payment confirmation',
  )

  const results = await new AxeBuilder({ page })
    .withTags(['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa'])
    .analyze()
  expect(results.violations).toEqual([])
})
