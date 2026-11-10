import { expect, test } from '@playwright/test'
const email = process.env.E2E_OWNER_EMAIL ?? 'owner@knitnprint.test'
const password = process.env.E2E_OWNER_PASSWORD ?? 'ci-owner-development-passphrase'

const storefrontUrl = process.env.E2E_STOREFRONT_URL ?? 'http://127.0.0.1:3000'

test('SKU URLs keep same-name products and deleted replacements separate', async ({ page }) => {
  expect((await page.request.post('/api/admin/auth/login', {data:{email,password}})).ok()).toBe(true)
  const base = `same-shirt-${Date.now()}`
  const input = {title:'Same shirt',slug:base,variants:[{title:'Default',price_minor:1500,currency:'EUR'}]}
  async function create() {
    const response = await page.request.post('/api/admin/products',{data:input})
    expect(response.status()).toBe(201)
    const product = await response.json()
    expect(product.slug).toBe(`${base}-${product.sku.toLowerCase()}`)
    expect(product.slug).not.toContain('-std')
    expect((await page.request.post(`/api/admin/products/${product.id}/status`,{data:{status:'active'}})).ok()).toBe(true)
    return product
  }
  const first = await create(), second = await create()
  expect(first.slug).not.toBe(second.slug)
  await page.goto('/#products')
  await page.getByRole('article').filter({hasText:first.sku}).getByRole('button',{name:'Edit',exact:true}).click()
  await expect(page.locator('#product-slug')).toHaveValue(base)
  await expect(page.locator('.url-preview')).toHaveText(`/products/${first.slug}`)
  const renamed = `renamed-${base}`
  await page.locator('#product-slug').fill(renamed)
  await Promise.all([page.waitForResponse(r=>r.request().method()==='PUT' && r.url().endsWith(`/products/${first.id}`) && r.ok()),page.getByRole('button',{name:'Save product',exact:true}).click()])
  const current = await (await page.request.get(`/api/admin/products/${first.id}`)).json()
  expect(current.slug).toBe(`${renamed}-${first.sku.toLowerCase()}`)
  const old = await page.request.get(`${storefrontUrl}/products/${first.slug}?source=old-link`, {maxRedirects:0})
  expect(old.status()).toBe(301)
  expect(old.headers().location).toContain(`/products/${current.slug}?source=old-link`)
  await page.goto(`${storefrontUrl}/products/${first.slug}`)
  await expect(page).toHaveURL(`${storefrontUrl}/products/${current.slug}`)
  expect((await page.request.delete(`/api/admin/products/${first.id}`)).status()).toBe(204)
  const replacement = await create()
  expect(replacement.slug).not.toBe(first.slug)
  for (const slug of [base, first.slug, current.slug]) {
    const response = await page.request.get(`${storefrontUrl}/api/products/${slug}`)
    expect(response.status()).toBe(404)
  }
  const surviving = await (await page.request.get(`${storefrontUrl}/api/products/${second.slug}`)).json()
  expect(surviving.id).toBe(second.id)
  await page.goto(`${storefrontUrl}/products/${replacement.slug}`)
  await expect(page.getByRole('heading',{level:1})).toHaveText('Same shirt')
})
