import assert from 'node:assert/strict'
import { test } from 'node:test'
import { productSlugBase, productSlugUrl } from '../src/product-slug.ts'

test('only the parent SKU is appended and existing suffixes are not duplicated', () => {
  assert.equal(productSlugUrl('linen-shirt', 'KP-000123'), 'linen-shirt-kp-000123')
  assert.equal(productSlugBase('linen-shirt-kp-000123', 'KP-000123'), 'linen-shirt')
  assert.equal(productSlugUrl('linen-shirt-kp-000123', 'KP-000123'), 'linen-shirt-kp-000123')
  assert.equal(productSlugUrl('linen-shirt'), 'linen-shirt-[product-sku]')
})
