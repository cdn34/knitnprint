import { defineConfig, devices } from '@playwright/test'

export default defineConfig({
  testDir: './tests/cart-drawer',
  workers: 1,
  timeout: 60_000,
  forbidOnly: Boolean(process.env.CI),
  use: { baseURL: 'http://127.0.0.1:13002', trace: 'retain-on-failure', actionTimeout: 10_000 },
  projects: [
    { name: 'desktop-chromium', use: { ...devices['Desktop Chrome'] } },
    { name: 'mobile-chromium', use: { ...devices['Pixel 7'] } },
  ],
  webServer: {
    command: 'npm run dev --workspace=@knitnprint/storefront -- --host 127.0.0.1 --port 13002',
    env: { API_BASE_URL: 'http://127.0.0.1:18084' },
    url: 'http://127.0.0.1:13002/health',
    timeout: 120_000,
    reuseExistingServer: !process.env.CI,
  },
})
