import assert from 'node:assert/strict'
import { test } from 'node:test'
import { printAreaOffsets } from '../src/personalization-measurements.ts'

const area = { x: 20, y: 20, width: 60, height: 56 }

test('front and back retain independent collar-to-hem measurements', () => {
  const front = printAreaOffsets(
    { x: 10, y: 10, width: 80, height: 80, physicalWidthCm: 40, physicalHeightCm: 50 },
    area,
    { physicalWidthCm: 48, physicalHeightCm: 60 },
  )
  const back = printAreaOffsets(
    { x: 10, y: 10, width: 80, height: 80, physicalWidthCm: 40, physicalHeightCm: 54 },
    area,
    { physicalWidthCm: 48, physicalHeightCm: 64 },
  )

  assert.deepEqual(front, { width: 30, height: 35, top: 7.5, left: 9, right: 9, bottom: 17.5 })
  assert.deepEqual(back, { width: 30, height: 37.8, top: 8, left: 9, right: 9, bottom: 18.2 })
})
