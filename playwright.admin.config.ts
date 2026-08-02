import { defineConfig, devices } from '@playwright/test'

export default defineConfig({
  testDir: './tests/admin-e2e',
  fullyParallel: false,
  forbidOnly: Boolean(process.env.CI),
  retries: process.env.CI ? 2 : 0,
  reporter: process.env.CI ? 'github' : 'list',
  use: {
    baseURL: 'http://127.0.0.1:3001',
    trace: 'on-first-retry',
  },
  projects: [
    {
      name: 'admin-chromium',
      use: { ...devices['Desktop Chrome'] },
    },
  ],
  webServer: [
    {
      name: 'api',
      command: 'cargo run -p knitprint-api --bin knitprint-api',
      url: 'http://127.0.0.1:8080/api/health',
      reuseExistingServer: !process.env.CI,
      timeout: 120_000,
      stdout: 'ignore',
      stderr: 'pipe',
    },
    {
      name: 'admin',
      command: 'npm run dev --workspace=@knitprint/admin -- --host 127.0.0.1',
      url: 'http://127.0.0.1:3001',
      reuseExistingServer: !process.env.CI,
      timeout: 120_000,
      stdout: 'ignore',
      stderr: 'pipe',
    },
    {
      name: 'storefront',
      command:
        'npm run dev --workspace=@knitprint/storefront -- --host 127.0.0.1',
      url: 'http://127.0.0.1:3000',
      reuseExistingServer: !process.env.CI,
      timeout: 120_000,
      stdout: 'ignore',
      stderr: 'pipe',
    },
  ],
})
