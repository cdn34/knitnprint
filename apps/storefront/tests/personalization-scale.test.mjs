import assert from 'node:assert/strict'
import { test } from 'node:test'
import { printAreaForVariant } from '../src/personalization-scale.ts'

const baseArticle = { x: 10, y: 10, width: 80, height: 80, physicalWidthCm: 40, physicalHeightCm: 50 }
const area = { x: 20, y: 20, width: 60, height: 56, physicalWidthCm: 30, physicalHeightCm: 35 }

test('a larger garment keeps the print cap and leaves visible fabric around the design', () => {
  const baseline = printAreaForVariant(area, baseArticle, baseArticle)
  const result = printAreaForVariant(area, baseArticle, { physicalWidthCm: 48, physicalHeightCm: 60 })
  assert.equal(result.physicalWidthCm, 30)
  assert.equal(result.physicalHeightCm, 35)
  assert.deepEqual(result.previewFrame, { x: 25, y: 20, width: 50, height: 46.666666666666664 })
  assert.ok(Math.abs(result.previewFrame.width / baseline.previewFrame.width - 40 / 48) < 1e-10)
  assert.ok(Math.abs(result.previewFrame.height / baseline.previewFrame.height - 50 / 60) < 1e-10)
})

test('a smaller garment uses the full available area without exceeding the garment', () => {
  const result = printAreaForVariant(area, baseArticle, { physicalWidthCm: 32, physicalHeightCm: 40 })
  assert.equal(result.physicalWidthCm, 24)
  assert.equal(result.physicalHeightCm, 28)
  assert.deepEqual(result.previewFrame, { x: 20, y: 20, width: 60, height: 56 })
})
