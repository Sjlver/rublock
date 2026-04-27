import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
  testDir: './tests',
  // WASM loading and puzzle generation can take several seconds.
  timeout: 30_000,
  retries: process.env.CI ? 1 : 0,
  reporter: process.env.CI ? 'github' : 'list',

  // Tests run against the production preview build, not the dev server.
  // Build the app (mise run web / npm run build) before running tests.
  webServer: {
    command: 'npm run preview',
    url: 'http://localhost:4173',
    reuseExistingServer: !process.env.CI,
  },

  use: {
    baseURL: 'http://localhost:4173',
  },

  projects: [
    {
      name: 'chromium',
      use: {
        ...devices['Desktop Chrome'],
        // Grant clipboard access so the "Share this puzzle" copy-to-clipboard
        // test can verify the written URL.
        permissions: ['clipboard-read', 'clipboard-write'],
      },
    },
  ],
});
