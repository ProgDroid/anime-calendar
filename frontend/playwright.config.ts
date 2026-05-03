import { defineConfig, devices } from '@playwright/test'

export default defineConfig({
  testDir: './e2e',
  fullyParallel: true,
  retries: process.env.CI ? 1 : 0,
  forbidOnly: !!process.env.CI,
  reporter: [['list'], ['html', { open: 'never' }]],
  use: {
    baseURL: 'http://localhost:5173',
    trace: 'on-first-retry',
  },
  projects: [
    {
      name: 'mobile-safari',
      use: { ...devices['iPhone 14'] },
    },
    {
      name: 'mobile-chrome',
      use: { ...devices['Pixel 7'] },
    },
    {
      name: 'mobile-firefox',
      // Playwright doesn't ship a "Pixel 5 Firefox" device profile, so we
      // approximate Android-class mobile firefox manually. Caveats:
      // - Firefox doesn't emulate touch events the way chromium/webkit do.
      //   Touch-dependent tests must run only on chromium/webkit projects.
      // - Firefox doesn't support isMobile=true (chromium-only flag).
      // The viewport + UA + deviceScaleFactor still give us a reasonable
      // mobile rendering target for layout/responsive coverage.
      use: {
        browserName: 'firefox',
        viewport: { width: 393, height: 851 },
        userAgent:
          'Mozilla/5.0 (Android 13; Mobile; rv:120.0) Gecko/120.0 Firefox/120.0',
        isMobile: false,
        deviceScaleFactor: 2.75,
      },
    },
  ],
  webServer: {
    command: 'npm run dev',
    url: 'http://localhost:5173',
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
  },
})
