import assert from 'node:assert/strict'
import { test } from 'node:test'
import { uploadImagesWithProgress, productPhotosInOrder, savedProductPhotoOrder } from '../src/product-image-uploads.ts'

test('a partial upload retries only unfinished photos and retains their view mappings', async () => {
  const images = [{ id: 'front' }, { id: 'back' }]
  const completed = new Map()
  const calls = []
  let fail = true
  const upload = async ({ id }) => {
    calls.push(id)
    if (id === 'back' && fail) throw new Error('storage unavailable')
    return `media-${id}`
  }
  assert.equal(await uploadImagesWithProgress(images, completed, upload), false)
  assert.equal(completed.get('front'), 'media-front')
  fail = false
  assert.equal(await uploadImagesWithProgress(images, completed, upload), true)
  assert.deepEqual(calls, ['front', 'back', 'back'])
  assert.deepEqual([...completed], [['front', 'media-front'], ['back', 'media-back']])
  // Retrying after the final product update fails must reuse every completed upload.
  assert.equal(await uploadImagesWithProgress(images, completed, upload), true)
  assert.deepEqual(calls, ['front', 'back', 'back'])
})


test('new photos can become the main photo while saved photos retain their identity and configuration', () => {
  const front = { id: 'saved-front', personalization: { mode: 'text' } }
  const back = { id: 'saved-back', personalization: { mode: 'photo' } }
  const pending = { id: 'new-detail', queued: true }
  const ordered = productPhotosInOrder([front, back, pending], ['new-detail', 'saved-back', 'saved-front'])
  assert.deepEqual(ordered.map(({ id }) => id), ['new-detail', 'saved-back', 'saved-front'])
  assert.equal(ordered[1], back)
  assert.equal(ordered[2], front)
  assert.deepEqual(savedProductPhotoOrder(ordered.map(({ id }) => id), new Map([['new-detail', 'uploaded-detail']])), ['uploaded-detail', 'saved-back', 'saved-front'])
  assert.deepEqual([front, back].map(({ id }) => id), ['saved-front', 'saved-back'])
})

test('deleted photos disappear and newly added photos append to the chosen order', () => {
  assert.deepEqual(productPhotosInOrder([{ id: 'a' }, { id: 'c' }, { id: 'new' }], ['c', 'deleted', 'a', 'a']).map(({ id }) => id), ['c', 'a', 'new'])
})
