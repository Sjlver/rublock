// Renders scripts/og-image.html to public/og-image.png (1200×630, the size
// OpenGraph / Twitter cards expect). Needs network access for the web font.
import { chromium } from '@playwright/test';
import { fileURLToPath } from 'node:url';

const src = new URL('./og-image.html', import.meta.url);
const out = fileURLToPath(new URL('../public/og-image.png', import.meta.url));

const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1200, height: 630 } });
await page.goto(src.href, { waitUntil: 'networkidle' });
await page.evaluate(() => document.fonts.ready);
await page.screenshot({ path: out });
await browser.close();
console.log(`wrote ${out}`);
