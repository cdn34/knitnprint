import assert from 'node:assert/strict'
import { test } from 'node:test'
import { uploadImagesWithProgress } from '../src/product-image-uploads.ts'

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
