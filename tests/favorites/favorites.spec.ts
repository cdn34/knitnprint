import AxeBuilder from '@axe-core/playwright'
import { expect, test, type Page } from '@playwright/test'
import { createServer, type Server } from 'node:http'

const category = { id: 'favorites-category', name: 'Presentes', slug: 'presentes', description: '', position: 0 }
const products = ['Caneca Aurora', 'Sweat com capuz'].map((title, index) => ({
  id: `019a4a00-0000-7000-8000-00000000000${index}`, title, slug: index ? 'sweat-com-capuz' : 'caneca-aurora',
  description: 'Uma peça pensada para guardar boas memórias.', status: 'active', search_keywords: '', categories: [category], personalization_area_price_minor: 100,
  variants: [{ id: `variant-${index}`, title: 'Rosa', sku: `FAVORITE-${index}`, price_minor: index ? 3500 : 1500, currency: 'EUR', option_values: {}, position: 0, available_quantity: 100, low_stock: false }],
  media: [{ id: `photo-${index}`, alt_text: title, position: 0, url: `/favorite-${index}.svg`, thumbnail_url: `/favorite-${index}.svg`, card_url: `/favorite-${index}.svg`, detail_url: `/favorite-${index}.svg` }],
  personalization: { mode: 'none', views: [], print_areas: [], text_max_characters: 35, text_min_size: 12, text_max_size: 72, allowed_fonts: ['Roboto'], allowed_colors: ['#111111'] },
}))
const profile = { id: 'favorite-customer', email: 'ana@example.test', first_name: 'Ana', last_name: 'Silva', phone: '', email_verified: true, addresses: [] }
const feedback = { average_rating: null, total_reviews: 0, rating_counts: [], reviews: [] }

function catalog(path: string) {
  if (path === '/api/products') return products
  if (path === '/api/categories') return [category]
  if (path.endsWith('/feedback') || path.includes('site-feedback')) return feedback
  return products.find(product => path === `/api/products/${product.slug}`) ?? []
}

let server: Server
test.beforeAll(async () => {
  server = createServer((request, response) => {
    response.setHeader('content-type', 'application/json')
    response.end(JSON.stringify(catalog(request.url!.split('?')[0])))
  })
  await new Promise<void>((resolve, reject) => {
    server.once('error', reject)
    server.listen(18083, '127.0.0.1', resolve)
  })
})
test.afterAll(async () => { await new Promise<void>(resolve => server.close(() => resolve())) })

async function fixture(page: Page, signedIn = true) {
  const state = { signedIn, ids: new Set<string>(), failSave: false, failList: false, saves: 0 }
  await page.addInitScript(() => localStorage.setItem('knitnprint-language', 'pt'))
  await page.route('https://fonts.googleapis.com/**', route => route.fulfill({ contentType: 'text/css', body: '' }))
  await page.route('**/favorite-*.svg', route => route.fulfill({ contentType: 'image/svg+xml', body: '<svg xmlns="http://www.w3.org/2000/svg" width="600" height="600"><rect width="600" height="600" fill="#f9f3ee"/><ellipse cx="290" cy="470" rx="160" ry="25" fill="#e8dcd6"/><path d="M390 220h40c100 0 100 150 0 150h-40" fill="none" stroke="#bc8b9a" stroke-width="35"/><path d="M145 170h260v225q0 65-130 65t-130-65z" fill="#d7adbb"/><ellipse cx="275" cy="170" rx="130" ry="25" fill="#f7e9ed"/><path d="M260 290q-55-65-75-10t75 100q110-80 75-115t-75 25" fill="#9c5263"/></svg>' }))
  await page.route('**/api/**', async route => {
    const path = new URL(route.request().url()).pathname
    const method = route.request().method()
    const unauthorized = () => route.fulfill({ status: 401, json: { error: { code: 'authentication_required', message: 'Sign in required.' } } })
    if (path === '/api/account/login') { state.signedIn = true; return route.fulfill({ json: profile }) }
    if (path === '/api/account/logout') { state.signedIn = false; return route.fulfill({ status: 204 }) }
    if (path === '/api/account/me') return state.signedIn ? route.fulfill({ json: profile }) : unauthorized()
    if (path === '/api/account/orders') return route.fulfill({ json: { orders: [], total: 0, page: 1, page_size: 10 } })
    if (path.startsWith('/api/account/favorites')) {
      if (!state.signedIn) return unauthorized()
      if (method === 'GET') {
        if (state.failList) return route.fulfill({ status: 503, json: { error: { code: 'favorites_unavailable', message: 'Unavailable.' } } })
        return route.fulfill({ json: products.filter(product => state.ids.has(product.id)) })
      }
      if (state.failSave) return route.fulfill({ status: 503, json: { error: { code: 'favorites_unavailable', message: 'Unavailable.' } } })
      const id = path.split('/').at(-1)!
      if (method === 'PUT') { state.saves++; state.ids.add(id) }
      else if (method === 'DELETE') state.ids.delete(id)
      return route.fulfill({ status: 204 })
    }
    if (path === '/api/cart') return route.fulfill({ json: { items: [], item_count: 0 } })
    return route.fulfill({ json: catalog(path) })
  })
  return state
}

test('hearts persist across catalog, home, collections, detail and the customer favorites section', async ({ page }, testInfo) => {
  const state = await fixture(page)
  await page.goto('/products')
  const add = (name: string) => page.getByRole('button', { name: `Adicionar ${name} aos favoritos`, exact: true })
  const remove = (name: string) => page.getByRole('button', { name: `Remover ${name} dos favoritos`, exact: true })
  await add(products[0].title).focus()
  await page.keyboard.press('Space')
  await expect(remove(products[0].title)).toHaveAttribute('aria-pressed', 'true')
  expect(state.saves).toBe(1)
  await page.goto('/')
  await expect(remove(products[0].title).first()).toHaveAttribute('aria-pressed', 'true')
  await page.goto('/collections/presentes')
  await expect(remove(products[0].title)).toHaveAttribute('aria-pressed', 'true')
  await add(products[1].title).click()
  await expect(remove(products[1].title)).toHaveAttribute('aria-pressed', 'true')
  await page.goto('/products/caneca-aurora')
  await expect(remove(products[0].title)).toHaveAttribute('aria-pressed', 'true')
  await page.goto('/account#favorites')
  const section = page.getByRole('region', { name: 'Os meus favoritos' })
  await expect(section.locator('.product-card')).toHaveCount(2)
  await page.reload()
  await expect(section.locator('.product-card')).toHaveCount(2)
  await section.scrollIntoViewIfNeeded()
  await page.screenshot({ path: testInfo.outputPath('account-favorites.png') })
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true)
  expect((await new AxeBuilder({ page }).include('#favorites').analyze()).violations).toEqual([])
  await section.getByRole('button', { name: `Remover ${products[0].title} dos favoritos` }).click()
  await expect(section.locator('.product-card')).toHaveCount(1)
  await section.getByRole('button', { name: `Remover ${products[1].title} dos favoritos` }).click()
  await expect(section.getByRole('heading', { name: 'As tuas peças preferidas ficam aqui' })).toBeVisible()
  await page.reload()
  await expect(section.locator('.product-card')).toHaveCount(0)
  expect(state.ids.size).toBe(0)
})

test('a guest can sign in and keep the product they wanted to save, then sign out', async ({ page }) => {
  const state = await fixture(page, false)
  await page.goto('/products')
  await page.getByRole('button', { name: `Adicionar ${products[0].title} aos favoritos` }).click()
  await expect(page).toHaveURL(/\/account#favorites$/)
  await expect(page.getByRole('status')).toContainText('Inicia sessão ou cria uma conta')
  await page.getByLabel('Endereço de email', { exact: true }).fill(profile.email)
  await page.getByLabel('Palavra-passe', { exact: true }).fill('favorite-test-passphrase')
  await page.locator('button.account-submit').click()
  const section = page.locator('#favorites')
  await expect(section.locator('.product-card')).toHaveCount(1)
  expect(state.saves).toBe(1)
  await page.getByRole('button', { name: 'Terminar sessão' }).click()
  await expect(section).toHaveCount(0)
  await page.goto('/products')
  await expect(page.getByRole('button', { name: `Adicionar ${products[0].title} aos favoritos` })).toHaveAttribute('aria-pressed', 'false')
})

test('failed saves and failed loading show a recoverable error', async ({ page }) => {
  const state = await fixture(page)
  state.failSave = true
  await page.goto('/products')
  const heart = page.getByRole('button', { name: `Adicionar ${products[0].title} aos favoritos` })
  await heart.click()
  await expect(page.getByRole('alert')).toContainText('Não foi possível atualizar os favoritos')
  await expect(heart).toHaveAttribute('aria-pressed', 'false')
  state.failSave = false
  await heart.click()
  await expect(page.getByRole('button', { name: `Remover ${products[0].title} dos favoritos` })).toHaveAttribute('aria-pressed', 'true')
  state.failList = true
  await page.goto('/account#favorites')
  await expect(page.locator('#favorites').getByRole('alert')).toContainText('Não foi possível carregar os favoritos')
  state.failList = false
  await page.locator('#favorites').getByRole('button', { name: 'Tentar novamente' }).click()
  await expect(page.locator('#favorites .product-card')).toHaveCount(1)
  state.signedIn = false
  await page.evaluate(() => window.dispatchEvent(new Event('focus')))
  await expect(page.locator('#favorites')).toContainText('Inicia sessão para veres os teus favoritos.')
  await expect(page.locator('#favorites .product-card')).toHaveCount(0)
  await page.locator('#favorites').getByRole('link', { name: 'Iniciar sessão' }).click()
  await expect(page.getByLabel('Endereço de email', { exact: true })).toBeVisible()
})
