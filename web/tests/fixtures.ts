import { test as base, expect } from '@playwright/test';

// Extend the base `test` so that every test automatically blocks requests to
// GoatCounter. This is necessary on two counts:
//
//   1. The production build injects <script src="//gc.zgo.at/count.js">.
//      That script fires a page-view automatically when it loads, before any
//      application code runs, so a JS-level guard in analytics.ts is not enough.
//
//   2. Even if the script somehow loaded, analytics.ts now also checks
//      `hostname !== 'localhost'`, providing a second layer of defence.
//
// Blocking at the network level is the only way to prevent the script from
// recording page views and making outbound requests during tests.
export const test = base.extend<Record<string, never>>({
  page: async ({ page }, use) => {
    await page.route(/gc\.zgo\.at/, (route) => route.abort());
    await use(page);
  },
});

export { expect };
