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
  await editor.getByLabel('SKU').fill(`PLANTER-PLUM-${unique}`)
  await editor.getByLabel('Price').fill('46.00')
  await editor.getByRole('button', { name: 'Add variant' }).click()
  await expect(editor).toContainText('2 configured for this product')
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
        sku: `PLANTER-PLUM-${unique}`,
        price_minor: 4600,
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

  await page.reload()
  const persistedProduct = catalog.getByRole('article').filter({ hasText: slug })
  await expect(persistedProduct.locator('.product-thumbnail img')).toBeVisible()
})
