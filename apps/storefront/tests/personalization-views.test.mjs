import assert from 'node:assert/strict'
import { test } from 'node:test'
import { activePersonalizationViews, configuredViews } from '../src/personalization-views.ts'

const area = (id) => ({ id, label: id, x: 2000, y: 2000, width: 5000, height: 5000, physical_width_cm: 20, physical_height_cm: 20 })
const config = {
  mode: 'photo_text',
  views: [
    { id: 'front', label: 'Frente', media_id: 'front-photo', mode: 'text', print_areas: [area('chest')] },
    { id: 'back', label: 'Costas', media_id: 'back-photo', mode: 'photo', print_areas: [area('back-print')] },
    { id: 'detail', label: 'Detalhe', media_id: 'detail-photo', mode: 'none', print_areas: [area('hidden')] },
    { id: 'empty', label: 'Vazia', media_id: 'empty-photo', mode: 'photo_text', print_areas: [] },
  ],
}
const parse = (value, productMedia = []) => configuredViews(value, 'variant', 'Frente', (i) => `Vista ${i}`, (i) => `Área ${i}`, productMedia)

test('personalization follows the admin photo order while keeping each design on its photograph', () => {
  const original = structuredClone(config)
  const views = parse(config, [
    { id: 'detail-photo' },
    { id: 'back-photo' },
    { id: 'empty-photo' },
    { id: 'front-photo' },
  ])
  assert.deepEqual(views.map(({ id, mediaId, mode, printAreas }) => [id, mediaId, mode, printAreas.map(({ id }) => id)]), [
    ['back', 'back-photo', 'photo', ['back-print']],
    ['front', 'front-photo', 'text', ['chest']],
  ])
  assert.deepEqual(config, original)
})

test('legacy views without a matching photograph retain their relative order after matched photos', () => {
  const legacy = { ...config, views: [
    { id: 'unassigned', mode: 'text', print_areas: [area('unassigned-area')] },
    ...config.views,
    { id: 'missing-photo', media_id: 'unavailable', mode: 'text', print_areas: [area('missing-area')] },
  ] }
  assert.deepEqual(parse(legacy, [{ id: 'back-photo' }, { id: 'front-photo' }]).map(({ id }) => id), [
    'back', 'front', 'unassigned', 'missing-photo',
  ])
})

test('the shop keeps text and photo permissions bound to the correct photographs', () => {
  const views = parse(config)
  assert.deepEqual(views.map(({ id, mediaId, mode, printAreas }) => [id, mediaId, mode, printAreas.map(({ id }) => id)]), [
    ['front', 'front-photo', 'text', ['chest']],
    ['back', 'back-photo', 'photo', ['back-print']],
  ])
  assert.deepEqual(activePersonalizationViews(config).map(({ id }) => id), ['front', 'back'])
})

test('deleting the last print area does not recreate a default area on reload', () => {
  const cleared = { ...config, views: config.views.map((view) => ({ ...view, print_areas: [] })) }
  assert.deepEqual(parse(cleared), [])
  assert.deepEqual(activePersonalizationViews(cleared), [])
})

test('existing photo configurations inherit the product mode until individually changed', () => {
  const legacy = { ...config, mode: 'text', views: [{ id: 'front', print_areas: [area('chest')] }] }
  assert.equal(parse(legacy)[0].mode, 'text')
})
