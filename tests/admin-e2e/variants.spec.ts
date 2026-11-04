import { expect, test } from '@playwright/test'
import { signInVerifiedCustomer } from '../helpers/customer'
const email = process.env.E2E_OWNER_EMAIL ?? 'owner@knitnprint.test'
const password = process.env.E2E_OWNER_PASSWORD ?? 'ci-owner-development-passphrase'

test('copies templates, maps existing stock, and sells a generated color/size combination', async ({ page }) => {
  expect((await page.request.post('/api/admin/auth/login', { data: { email, password } })).ok()).toBe(true)
  const slug = `variant-shirt-${Date.now()}`
  const response = await page.request.post('/api/admin/products', { data: { title: slug, slug, variants: [{ title: 'Default', price_minor: 2000, currency: 'EUR', available_quantity: 3 }] } })
  expect(response.status()).toBe(201)
  const product = await response.json()
  await page.goto('/#products')
  await page.getByRole('article').filter({ hasText: slug }).getByRole('button', { name: 'Edit', exact: true }).click()
  const matrix = page.locator('.variant-matrix')
  await expect(matrix).toContainText(product.sku)
  await expect(matrix.getByLabel('Use color-specific photos (optional)')).not.toBeChecked()
  await matrix.getByLabel('Add option template').selectOption('10000000-0000-0000-0000-000000000001')
  for (const color of ['Black', 'White', 'Green', 'Yellow', 'Pink', 'Grey']) await matrix.getByRole('button', { name: `Remove ${color}`, exact: true }).click()
  await matrix.getByLabel('Add option template').selectOption('10000000-0000-0000-0000-000000000002')
  await matrix.getByLabel('Map Color 1', { exact: true }).selectOption('RED')
  await matrix.getByLabel('Map Size 1', { exact: true }).selectOption('M')
  await matrix.getByRole('button', { name: 'Generate / refresh matrix preview' }).click()
  await expect(matrix).toContainText('12/100 active combinations')
  await matrix.getByLabel('Bulk price (€)', { exact: true }).fill('22.50')
  await Promise.all([
    page.waitForResponse(r => r.url().endsWith('/variant-matrix') && r.request().method() === 'PUT' && r.ok()),
    matrix.getByRole('button', { name: 'Save options and variants' }).click(),
  ])
  const saved = await (await page.request.get(`/api/admin/products/${product.id}`)).json()
  expect(saved.variants).toHaveLength(12)
  expect(saved.variants.find((v: { id: string }) => v.id === product.variants[0].id).sku).toBe(product.variants[0].sku)
  expect(saved.variants.find((v: { id: string }) => v.id === product.variants[0].id).available_quantity).toBe(3)
  expect(saved.variants.every((v: { sku: string; price_minor: number }) => v.sku.startsWith(saved.sku) && v.price_minor === 2250)).toBe(true)
  // The shared palette is unaffected by this product's removals.
  const templates = await (await page.request.get('/api/admin/variant-templates')).json()
  expect(templates[0].definition.values).toHaveLength(8)
  expect((await page.request.post(`/api/admin/products/${product.id}/status`, { data: { status: 'active' } })).ok()).toBe(true)
  const redM = saved.variants.find((v: { selections: {COL: string; SIZ: string} }) => v.selections.COL === 'RED' && v.selections.SIZ === 'M')
  const blueM = saved.variants.find((v: { selections: {COL: string; SIZ: string} }) => v.selections.COL === 'BLU' && v.selections.SIZ === 'M')
  await signInVerifiedCustomer(page.request, `${slug}@example.test`, 'Variant', 'Tester')
  await page.goto(`http://127.0.0.1:3000/products/${slug}`)
  const picker = page.locator('.structured-variant-picker')
  await picker.getByRole('button', { name: 'Red', exact: true }).click()
  await picker.getByRole('button', { name: 'M', exact: true }).click()
  await expect(picker.getByRole('button', { name: 'M', exact: true })).toHaveAttribute('aria-pressed', 'true')
  await picker.getByRole('button', { name: 'Blue', exact: true }).click()
  await expect(picker.getByRole('button', { name: 'Blue', exact: true })).toHaveAttribute('aria-pressed', 'true')
  await expect(picker.getByRole('button', { name: 'M', exact: true })).toHaveAttribute('aria-pressed', 'true')
  const added = page.waitForResponse(r => r.url().endsWith('/api/cart/items') && r.request().method() === 'POST')
  await page.getByRole('button', { name: /Add to (bag|cart)/i }).click()
  const addedResponse = await added
  expect(addedResponse.status(), await addedResponse.text()).toBe(201)
  expect((await addedResponse.json()).items.some((item: {variant_id: string}) => item.variant_id === blueM.id)).toBe(true)
  expect(redM.id).toBe(product.variants[0].id)
})

test('color-specific photos are opt-in and unmapped colors keep the shared gallery', async ({ page }) => {
  expect((await page.request.post('/api/admin/auth/login', { data: { email, password } })).ok()).toBe(true)
  const slug = `color-photos-${Date.now()}`
  const created = await page.request.post('/api/admin/products', {data:{title:slug,slug,variants:['Red','Blue'].map(title=>({title,price_minor:1000,currency:'EUR'}))}})
  expect(created.status()).toBe(201)
  let product=await created.json()
  const bytes=Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=', 'base64')
  const ids:string[]=[]
  for(const label of ['Shared photo','Red photo']) {
    const initiated=await page.request.post('/api/admin/media/uploads',{data:{filename:'photo.png',content_type:'image/png',byte_size:bytes.length}})
    expect(initiated.status()).toBe(201)
    const media=await initiated.json();ids.push(media.id)
    expect((await page.request.put(media.upload_url,{data:bytes,headers:{'content-type':'image/png'}})).ok()).toBe(true)
    expect((await page.request.post(`/api/admin/media/uploads/${media.id}/complete`,{data:{product_id:product.id,alt_text:label}})).ok()).toBe(true)
  }
  const config={color_photos:false,options:[{code:'COL',label:'Color',control:'swatch',values:[{code:'RED',label:'Red',color:'#ff0000',media_ids:[ids[1]]},{code:'BLU',label:'Blue',color:'#0000ff',media_ids:[]}]}]}
  const variants=product.variants.map((v:{id:string},i:number)=>({id:v.id,selections:{COL:i?'BLU':'RED'},price_minor:1000,available_quantity:0,expected_quantity:0,retired:false}))
  expect((await page.request.put(`/api/admin/products/${product.id}/variant-matrix`,{data:{revision:0,config,variants}})).ok()).toBe(true)
  expect((await page.request.post(`/api/admin/products/${product.id}/status`,{data:{status:'active'}})).ok()).toBe(true)
  await page.goto(`http://127.0.0.1:3000/products/${slug}`)
  await expect(page.locator('.product-gallery img').first()).toHaveAttribute('alt','Shared photo')
  product=await(await page.request.get(`/api/admin/products/${product.id}`)).json()
  config.color_photos=true
  expect((await page.request.put(`/api/admin/products/${product.id}/variant-matrix`,{data:{revision:product.variant_revision,config,variants}})).ok()).toBe(true)
  await page.reload()
  await page.locator('.structured-variant-picker').getByRole('button',{name:'Red',exact:true}).click()
  await expect(page.locator('.product-gallery img').first()).toHaveAttribute('alt','Red photo')
  await page.locator('.structured-variant-picker').getByRole('button',{name:'Blue',exact:true}).click()
  await expect(page.locator('.product-gallery img').first()).toHaveAttribute('alt','Shared photo')
  await page.setViewportSize({width:390,height:844})
  await page.locator('.structured-variant-picker').getByRole('button',{name:'Red',exact:true}).click()
  await expect(page.locator('.product-gallery img').first()).toHaveAttribute('alt','Red photo')
})
