import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { countryCodes, countryOptions } from '../src/countries.ts'

test('the country selector contains every shipping country once and sends ISO codes', () => {
  const backend = readFileSync(new URL('../../../backend/src/delivery_rates.rs', import.meta.url), 'utf8')
  const serverCodes = /const COUNTRIES: &str = "([A-Z ]+)"/.exec(backend)[1].split(' ')
  assert.deepEqual(countryCodes, serverCodes)
  assert.equal(new Set(countryCodes).size, countryCodes.length)
  assert(countryCodes.every(code => /^[A-Z]{2}$/.test(code)))
})

test('country names are localized and sorted, while submitted codes remain stable', () => {
  for (const [locale, expectedSpain] of [['en', 'Spain'], ['pt', 'Espanha'], ['es', 'España']]) {
    const options = countryOptions(locale)
    assert.equal(options.find(option => option.code === 'ES').name, expectedSpain)
    assert.equal(options.find(option => option.code === 'PT').name, 'Portugal')
    assert.equal(options.length, countryCodes.length)
    assert.deepEqual([...options].sort((a, b) => a.name.localeCompare(b.name, locale)), options)
  }
})
