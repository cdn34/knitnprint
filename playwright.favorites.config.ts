import { defineConfig, devices } from '@playwright/test'

export default defineConfig({
  testDir: './tests/favorites',
  workers: 1,
  timeout: 60_000,
  forbidOnly: Boolean(process.env.CI),
  use: { baseURL: 'http://127.0.0.1:13001', trace: 'retain-on-failure' },
  projects: [
    { name: 'desktop-chromium', use: { ...devices['Desktop Chrome'] } },
    { name: 'mobile-chromium', use: { ...devices['Pixel 7'] } },
  ],
  webServer: {
    command: 'npm run dev --workspace=@knitnprint/storefront -- --host 127.0.0.1 --port 13001',
    env: { API_BASE_URL: 'http://127.0.0.1:18083' },
    url: 'http://127.0.0.1:13001/health',
    timeout: 120_000,
    reuseExistingServer: !process.env.CI,
  },
})
