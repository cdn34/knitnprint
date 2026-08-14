import { expect, test } from '@playwright/test'

const ownerEmail = process.env.E2E_OWNER_EMAIL ?? 'owner@knitprint.local'
const ownerPassword =
  process.env.E2E_OWNER_PASSWORD ?? 'local-development-passphrase'

test('lets an owner manage commercial settings and complete an order journey', async ({
  page,
}) => {
  const unique = `${Date.now()}-${test.info().retry}`
  const title = `Woven Planter ${unique}`
  const slug = `woven-planter-${unique}`
  const sku = `PLANTER-${unique}`
  const plumSku = `PLANTER-PLUM-${unique}`
  const oatSku = `PLANTER-OAT-${unique}`
  const discountCode = `BROWSER-${unique}`.toUpperCase()

  await page.goto('/')
  await page.getByLabel('Email address').fill(ownerEmail)
  await page.getByLabel('Password').fill(ownerPassword)
  await page.getByRole('button', { name: 'Sign in' }).click()
  await expect(page.getByRole('link', { name: 'Settings' })).toBeVisible()
  const resetSettings = await page.request.post('/api/admin/settings', {
    data: {
      store_name: 'KnitPrint',
      support_email: 'hello@knitprint.local',
      currency: 'EUR',
      tax_enabled: false,
      shipping_zones: [{
        name: 'Worldwide',
        country_codes: [],
        active: true,
        methods: [{ name: 'Standard shipping', flat_rate_minor: 0, active: true }],
      }],
      tax_rules: [],
      reason: 'Reset browser commercial configuration',
    },
  })
  expect(resetSettings.ok()).toBeTruthy()

  await page.getByRole('link', { name: 'Settings' }).click()
  const settings = page.getByRole('region', { name: 'Settings' })
  const shippingCard = settings.locator('.settings-card').filter({ hasText: 'Shipping zones' })
  const zoneForm = shippingCard.locator('form').filter({ hasText: 'Add shipping zone' })
  await zoneForm.getByLabel('Zone name').fill('Portugal')
  await zoneForm.getByLabel('Countries').fill('PT')
  await zoneForm.getByLabel('Initial method name').fill('Standard tracked')
  await zoneForm.getByLabel('Flat rate (EUR)').fill('6.00')
  await zoneForm.getByLabel('Audit reason').fill('Configure Portugal browser shipping')
  await zoneForm.getByRole('button', { name: 'Add zone' }).click()
  await expect(shippingCard).toContainText('Standard tracked · €6.00')

  const methodForm = shippingCard.locator('form').filter({ hasText: 'Add shipping method' })
  await methodForm.getByLabel('Shipping zone').selectOption({ label: 'Portugal' })
  await methodForm.getByLabel('Additional method name').fill('Express tracked')
  await methodForm.getByLabel('Flat rate (EUR)').fill('12.00')
  await methodForm.getByLabel('Audit reason').fill('Add browser express shipping')
  await methodForm.getByRole('button', { name: 'Add method' }).click()
  await expect(shippingCard).toContainText('Express tracked · €12.00')

  const taxCard = settings.locator('.settings-card').filter({ hasText: 'Tax rules' })
  await taxCard.getByLabel('Rule name').fill('Portugal browser tax')
  await taxCard.getByLabel('Countries').fill('PT')
  await taxCard.getByLabel('Rate (%)').fill('23')
  await taxCard.getByLabel('Audit reason').fill('Configure confirmed browser tax fixture')
  await taxCard.getByRole('button', { name: 'Add tax rule' }).click()
  await expect(taxCard).toContainText('Portugal browser tax')

  const identityCard = settings.locator('.settings-card').filter({ hasText: 'Store identity' })
  await identityCard.getByLabel('Enable destination tax calculation').check()
  await identityCard.getByLabel('Audit reason').fill('Enable destination tax for browser verification')
  await identityCard.getByRole('button', { name: 'Save store settings' }).click()
  await expect(settings).toContainText(/stripe configured|manual development/)

  await page.getByRole('link', { name: 'Discounts' }).click()
  const discounts = page.getByRole('region', { name: 'Discounts' })
  await discounts.getByLabel('Code').fill(discountCode)
  await discounts.getByRole('spinbutton', { name: 'Percentage' }).fill('10')
  await discounts.getByLabel('Global usage limit').fill('10')
  await discounts.getByLabel('Per-customer limit').fill('1')
  await discounts.getByLabel('Audit reason').fill('Browser checkout promotion')
  await discounts.getByRole('button', { name: 'Create discount' }).click()
  const discountRecord = discounts.getByRole('article').filter({ hasText: discountCode })
  await expect(discountRecord).toContainText('10% off')
  await expect(discountRecord).toContainText('active')

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
  await page
    .getByLabel('Shipping method')
    .selectOption({ label: 'Express tracked · €12.00' })
  await page.getByLabel('Discount code').fill(discountCode.toLowerCase())
  await page.getByRole('button', { name: 'Apply', exact: true }).click()
  await expect(page.locator('.cart-summary')).toContainText(discountCode)
  await expect(page.locator('.cart-summary')).toContainText('Express tracked')
  await expect(page.locator('.cart-summary')).toContainText('Tax · 23%')
  await expect(page.locator('.cart-summary')).toContainText('€67.89')
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
  await expect(page.getByLabel(`Order ${orderNumber}`)).toContainText(`Discount ${discountCode}`)
  await expect(page.getByLabel(`Order ${orderNumber}`)).toContainText('Express tracked')
  await expect(page.getByLabel(`Order ${orderNumber}`)).toContainText('Tax (23%)')
  page.once('dialog', async (dialog) => {
    await dialog.accept('Browser development payment')
  })
  await page.getByRole('button', { name: 'Record manual payment' }).click()
  await expect(page.getByLabel(`Order ${orderNumber}`)).toContainText('Manual payment recorded')
  await expect(orderRow).toContainText('paid')

  await page.getByRole('heading', { name: 'Create fulfillment' }).scrollIntoViewIfNeeded()
  await page.getByRole('spinbutton', { name: /quantity to ship/ }).fill('1')
  await page.getByLabel('Carrier').fill('CTT')
  await page.getByLabel('Tracking number').fill(`TRACK-${unique}`)
  await page.getByLabel('Tracking URL').fill(`https://tracking.example.test/TRACK-${unique}`)
  await page.getByLabel('Internal reason').fill('Packed during browser verification')
  await page.getByRole('button', { name: 'Create shipment' }).click()
  await expect(page.getByLabel(`Order ${orderNumber}`)).toContainText('Order fulfilled')
  await expect(page.getByLabel(`Order ${orderNumber}`)).toContainText(`TRACK-${unique}`)
  await expect(page.getByLabel(`Order ${orderNumber}`)).toContainText('fulfillment created')
  await expect(orderRow).toContainText('paid')

  await page.getByRole('heading', { name: 'Create refund' }).scrollIntoViewIfNeeded()
  await page.getByLabel('Full remaining balance').check()
  await page.getByLabel('Return selected quantities to available stock').check()
  await page.getByLabel('Customer-facing reason').fill('Browser return accepted')
  await page.getByLabel('Internal note').fill('Verified by the Phase 9 browser journey')
  await page.getByRole('button', { name: 'Create refund' }).click()
  const orderDetail = page.getByLabel(`Order ${orderNumber}`)
  await expect(orderDetail).toContainText('Payment refunded')
  await expect(orderDetail).toContainText('Browser return accepted')
  await expect(orderDetail).toContainText('Verified by the Phase 9 browser journey')
  await expect(orderDetail).toContainText('Restocked')
  await expect(orderRow).toContainText('refunded')

  await page.setViewportSize({ width: 390, height: 844 })
  const orderDimensions = await page.evaluate(() => ({
    viewport: document.documentElement.clientWidth,
    content: document.documentElement.scrollWidth,
  }))
  expect(orderDimensions.content).toBeLessThanOrEqual(orderDimensions.viewport)
})
