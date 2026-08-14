import { expect, test } from '@playwright/test'

const ownerEmail = process.env.E2E_OWNER_EMAIL ?? 'owner@knitprint.local'
const ownerPassword =
  process.env.E2E_OWNER_PASSWORD ?? 'local-development-passphrase'

test('lets an owner create, preview, search, and publish a product', async ({
  page,
}) => {
  const unique = `${Date.now()}-${test.info().retry}`
  const title = `Woven Planter ${unique}`
  const slug = `woven-planter-${unique}`
  const sku = `PLANTER-${unique}`
  const plumSku = `PLANTER-PLUM-${unique}`
  const oatSku = `PLANTER-OAT-${unique}`

  await page.goto('/')
  await page.getByLabel('Email address').fill(ownerEmail)
  await page.getByLabel('Password').fill(ownerPassword)
  await page.getByRole('button', { name: 'Sign in' }).click()
  await page.getByRole('link', { name: 'Products' }).click()

  const catalog = page.getByRole('region', { name: 'Products' })
  await expect(catalog).toBeVisible()
  await catalog.getByLabel('Product title').fill(title)
  await catalog.getByLabel('URL slug').fill(slug)
  await catalog
    .getByLabel('Description')
    .fill('A tactile home for printed forms and soft stitches.')
  await catalog.getByLabel('Search keywords').fill(`browsercatalog ${unique}`)
  await catalog.getByLabel('SKU').fill(sku)
  await catalog.getByLabel('Price').fill('42.00')
  await catalog.getByRole('button', { name: 'Create draft' }).click()

  const product = catalog.getByRole('article').filter({ hasText: slug })
  await expect(product).toContainText(title)
  await expect(product).toContainText('draft')
  await expect(
    catalog.getByRole('article', { name: 'Product preview' }),
  ).toContainText(title)

  const editor = catalog.getByLabel(`Edit ${title}`)
  const categoryName = `Homewares ${unique}`
  const categorySlug = `homewares-${unique}`
  await editor.getByLabel('Name').fill(categoryName)
  await editor.getByLabel('URL slug').fill(categorySlug)
  await editor.getByRole('button', { name: 'Create category' }).click()
  await expect(editor.getByLabel(categoryName)).toBeVisible()
  await editor.getByLabel(categoryName).check()
  await editor.getByRole('button', { name: 'Save categories' }).click()

  await editor.getByLabel('Variant title').fill('Plum')
  await editor.getByLabel('SKU').fill(plumSku)
  await editor.getByLabel('Price').fill('46.00')
  await editor.getByRole('button', { name: 'Add variant' }).click()
  await expect(editor).toContainText('2 configured for this product')

  await editor.getByLabel('Variant title').fill('Oat')
  await editor.getByLabel('SKU').fill(oatSku)
  await editor.getByLabel('Price').fill('48.00')
  await editor.getByRole('button', { name: 'Add variant' }).click()
  await expect(editor).toContainText('3 configured for this product')
  await expect(editor).toContainText(categoryName)

  page.once('dialog', async (dialog) => {
    expect(dialog.message()).toContain(title)
    await dialog.accept(`${title} in a soft neutral finish`)
  })
  await product.getByLabel('Image').setInputFiles({
    name: 'woven-planter.png',
    mimeType: 'image/png',
    buffer: Buffer.from(
      'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=',
      'base64',
    ),
  })
  await expect(product.locator('.product-thumbnail img')).toBeVisible()

  await catalog.getByLabel('Search products').fill(`browsercatalog ${unique}`)
  await expect(product).toBeVisible()
  await product.getByRole('button', { name: 'Publish' }).click()
  await expect(product).toContainText('active')

  const publicResponse = await page.request.get(`/api/products/${slug}`)
  expect(publicResponse.ok()).toBeTruthy()
  const publicProduct = await publicResponse.json()
  expect(publicProduct).toMatchObject({
    title,
    slug,
    status: 'active',
    variants: [
      { sku, price_minor: 4200, currency: 'EUR' },
      {
        sku: plumSku,
        price_minor: 4600,
        currency: 'EUR',
      },
      {
        sku: oatSku,
        price_minor: 4800,
        currency: 'EUR',
      },
    ],
    categories: [{ name: categoryName, slug: categorySlug }],
    media: [
      {
        alt_text: `${title} in a soft neutral finish`,
        position: 0,
        thumbnail_url: expect.stringContaining('/thumbnail'),
        card_url: expect.stringContaining('/card'),
        detail_url: expect.stringContaining('/detail'),
      },
    ],
  })
  const mediaResponse = await page.request.get(publicProduct.media[0].url)
  expect(mediaResponse.ok()).toBeTruthy()
  expect(mediaResponse.headers()['content-type']).toBe('image/webp')
  expect(mediaResponse.headers()['cache-control']).toContain('immutable')

  await page.getByRole('link', { name: 'Inventory' }).click()
  const inventory = page.getByRole('region', { name: 'Inventory' })
  const inventoryRow = inventory.getByRole('button').filter({ hasText: oatSku })
  await inventoryRow.click()
  await inventory.getByLabel('Quantity change').fill('7')
  await inventory.getByLabel('Reason').fill('Initial oat stock')
  await inventory.getByLabel('Low-stock threshold').fill('3')
  await inventory.getByRole('button', { name: 'Apply adjustment' }).click()
  await expect(inventoryRow).toContainText('7')
  await expect(inventory).toContainText('Initial oat stock')

  const plumInventoryRow = inventory
    .getByRole('button')
    .filter({ hasText: plumSku })
  await plumInventoryRow.click()
  await inventory.getByLabel('Quantity change').fill('2')
  await inventory.getByLabel('Reason').fill('Small plum batch')
  await inventory.getByLabel('Low-stock threshold').fill('3')
  await inventory.getByRole('button', { name: 'Apply adjustment' }).click()
  await expect(plumInventoryRow).toContainText('2')

  await page.getByRole('link', { name: 'Dashboard' }).click()
  const metrics = page.getByRole('region', { name: 'Inventory metrics' })
  await expect(metrics).toBeVisible()
  await expect(
    metrics
      .getByRole('article')
      .filter({ hasText: 'Available units' })
      .locator('strong'),
  ).not.toHaveText('—')
  await expect(
    metrics
      .getByRole('article')
      .filter({ hasText: 'Low stock' })
      .locator('strong'),
  ).not.toHaveText('—')
  const stockSummary = page.getByRole('region', { name: 'Low-stock variants' })
  await expect(stockSummary).toBeVisible()
  await page.setViewportSize({ width: 390, height: 844 })
  const dashboardDimensions = await page.evaluate(() => ({
    viewport: document.documentElement.clientWidth,
    content: document.documentElement.scrollWidth,
  }))
  expect(dashboardDimensions.content).toBeLessThanOrEqual(
    dashboardDimensions.viewport,
  )
  await stockSummary.getByRole('link', { name: 'Review inventory' }).click()

  const filteredInventory = page.getByRole('region', { name: 'Inventory' })
  await filteredInventory.getByLabel('Search inventory').fill(plumSku)
  await filteredInventory
    .getByRole('button', { name: /Needs attention/ })
    .click()
  await expect(
    filteredInventory.getByRole('button').filter({ hasText: plumSku }),
  ).toBeVisible()
  await expect(
    filteredInventory.getByRole('button').filter({ hasText: oatSku }),
  ).toHaveCount(0)

  await filteredInventory.getByRole('button', { name: /Healthy/ }).click()
  await expect(filteredInventory).toContainText(
    'No inventory matches the current search and stock filter.',
  )
  await filteredInventory.getByLabel('Search inventory').fill('')
  await expect(
    filteredInventory.getByRole('button').filter({ hasText: oatSku }),
  ).toBeVisible()

  await filteredInventory.getByRole('button', { name: /Out of stock/ }).click()
  await expect(
    filteredInventory.getByText(`Default · ${sku}`, { exact: true }),
  ).toBeVisible()
  await expect(
    filteredInventory.getByRole('button').filter({ hasText: plumSku }),
  ).toHaveCount(0)

  await page.setViewportSize({ width: 1280, height: 900 })
  await page.reload()
  await page.getByRole('link', { name: 'Products' }).click()
  const persistedProduct = page
    .getByRole('region', { name: 'Products' })
    .getByRole('article')
    .filter({ hasText: slug })
  await expect(persistedProduct.locator('.product-thumbnail img')).toBeVisible()

  await page.goto(`http://127.0.0.1:3000/products/${slug}`)
  await page.waitForLoadState('networkidle')
  await expect(page.getByRole('heading', { level: 1 })).toHaveText(title)
  await expect(page.getByRole('radio', { name: /Default/ })).toBeDisabled()
  await expect(page.getByRole('radio', { name: /Plum/ })).toBeChecked()
  await expect(page.locator('.product-detail-price')).toContainText('46.00')
  await expect(page.getByRole('status')).toContainText('Only 2 left')

  await page.locator('.variant-option').filter({ hasText: 'Oat' }).click()
  await expect(page.getByRole('radio', { name: /Oat/ })).toBeChecked()
  await expect(page.getByText(`SKU ${oatSku}`)).toBeVisible()
  await expect(page.locator('.product-detail-price')).toContainText('48.00')
  await expect(page.getByRole('status')).toContainText('In stock')

  await page.locator('.variant-option').filter({ hasText: 'Plum' }).click()
  await expect(page.getByRole('radio', { name: /Plum/ })).toBeChecked()
  await expect(page.getByText(`SKU ${plumSku}`)).toBeVisible()
  await expect(page.locator('.product-detail-price')).toContainText('46.00')
  await expect(page.getByRole('status')).toContainText('Only 2 left')

  await page.setViewportSize({ width: 390, height: 844 })
  await expect(page.getByRole('radio', { name: /Plum/ })).toBeVisible()
  const dimensions = await page.evaluate(() => ({
    viewport: document.documentElement.clientWidth,
    content: document.documentElement.scrollWidth,
  }))
  expect(dimensions.content).toBeLessThanOrEqual(dimensions.viewport)

  await page.setViewportSize({ width: 1280, height: 900 })
  await page.locator('.variant-option').filter({ hasText: 'Oat' }).click()
  await page.getByRole('button', { name: 'Add to cart' }).click()
  await page.getByRole('link', { name: 'View your cart' }).click()
  await page.getByLabel('Email').fill(`order-${unique}@example.com`)
  await page.getByLabel('First name').fill('Order')
  await page.getByLabel('Last name').fill('Browser')
  await page.getByLabel('Recipient').fill('Order Browser')
  await page.getByLabel('Address', { exact: true }).fill('8 Timeline Lane')
  await page.getByLabel('City').fill('Lisbon')
  await page.getByLabel('Postal code').fill('1000-008')
  await page.getByRole('button', { name: 'Save delivery details' }).click()
  await page.getByRole('button', { name: 'Create order' }).click()
  const orderNumber = (await page.locator('.order-confirmation .eyebrow').textContent())?.trim()
  expect(orderNumber).toMatch(/KP-\d{4}-\d{6}/)

  await page.goto('http://127.0.0.1:3001/#orders')
  const orders = page.getByRole('region', { name: 'Orders' })
  await expect(orders).toBeVisible()
  const orderRow = orders.getByRole('button').filter({ hasText: orderNumber ?? '' })
  await orderRow.click()
  await expect(page.getByLabel(`Order ${orderNumber}`)).toContainText(oatSku)
  await expect(page.getByLabel(`Order ${orderNumber}`)).toContainText('Order Browser')
  page.once('dialog', async (dialog) => {
    await dialog.accept('Browser development payment')
  })
  await page.getByRole('button', { name: 'Record manual payment' }).click()
  await expect(page.getByLabel(`Order ${orderNumber}`)).toContainText('Manual payment recorded')
  await expect(orderRow).toContainText('paid')

  await page.getByRole('heading', { name: 'Create fulfillment' }).scrollIntoViewIfNeeded()
  await page.locator('.fulfillment-form input[type="number"]').fill('1')
  await page.getByLabel('Carrier').fill('CTT')
  await page.getByLabel('Tracking number').fill(`TRACK-${unique}`)
  await page.getByLabel('Tracking URL').fill(`https://tracking.example.test/TRACK-${unique}`)
  await page.getByLabel('Internal reason').fill('Packed during browser verification')
  await page.getByRole('button', { name: 'Create shipment' }).click()
  await expect(page.getByLabel(`Order ${orderNumber}`)).toContainText('Order fulfilled')
  await expect(page.getByLabel(`Order ${orderNumber}`)).toContainText(`TRACK-${unique}`)
  await expect(page.getByLabel(`Order ${orderNumber}`)).toContainText('fulfillment created')
  await expect(orderRow).toContainText('paid')
})
