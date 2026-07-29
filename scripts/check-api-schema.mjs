import { execFileSync } from 'node:child_process'
import { readFile } from 'node:fs/promises'

const schema = new URL('../packages/api-client/src/schema.ts', import.meta.url)
const openapi = new URL('../openapi/knitprint.json', import.meta.url)
const schemaBefore = await readFile(schema, 'utf8')
const openapiBefore = await readFile(openapi, 'utf8')

execFileSync(
  'cargo',
  ['run', '-q', '-p', 'knitprint-api', '--bin', 'export_openapi', '--', 'openapi/knitprint.json'],
  { stdio: 'inherit' },
)
execFileSync('node', ['scripts/generate-api-schema.mjs'], { stdio: 'inherit' })

const schemaAfter = await readFile(schema, 'utf8')
const openapiAfter = await readFile(openapi, 'utf8')
if (schemaBefore !== schemaAfter || openapiBefore !== openapiAfter) {
  console.error('Generated API contract was stale. Run npm run api:generate and commit the result.')
  process.exitCode = 1
}
