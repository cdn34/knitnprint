import { expect, test } from '@playwright/test'

const ownerEmail = process.env.E2E_OWNER_EMAIL ?? 'owner@knitprint.local'
const ownerPassword =
  process.env.E2E_OWNER_PASSWORD ?? 'local-development-passphrase'

test('lets an owner create and disable a least-privilege staff account', async ({
  page,
}) => {
  const unique = `${Date.now()}-${test.info().retry}`
  const displayName = `Order Viewer ${unique}`
  const staffEmail = `order-viewer-${unique}@knitprint.test`

  await page.goto('/')
  await page.getByLabel('Email address').fill(ownerEmail)
  await page.getByLabel('Password').fill(ownerPassword)
  await page.getByRole('button', { name: 'Sign in' }).click()
  await page.getByRole('link', { name: 'Staff' }).click()

  const staffSection = page.getByRole('region', { name: 'Staff accounts' })
  await expect(staffSection).toBeVisible()

  await staffSection.getByLabel('Display name').fill(displayName)
  await staffSection.getByLabel('Email address').fill(staffEmail)
  await staffSection
    .getByLabel('Temporary password')
    .fill('browser-test-passphrase')
  await staffSection.getByLabel('View orders').check()
  await staffSection
    .getByRole('button', { name: 'Create staff account' })
    .click()

  const staffRecord = staffSection
    .getByRole('article')
    .filter({ hasText: staffEmail })
  await expect(staffRecord).toContainText(displayName)
  await expect(staffRecord).toContainText('1 capabilities')

  page.once('dialog', async (dialog) => {
    expect(dialog.message()).toContain(displayName)
    await dialog.accept('Browser test access removal')
  })
  await staffRecord.getByRole('button', { name: 'Disable' }).click()

  await expect(staffRecord).toContainText('Disabled')
  await expect(
    staffRecord.getByRole('button', { name: 'Disable' }),
  ).toHaveCount(0)
})
