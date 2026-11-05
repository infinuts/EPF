import { chromium } from '@playwright/test';
import { mkdir } from 'node:fs/promises';

// Run against `npm run dev`. Browser preview data never changes a real router.
await mkdir('artifacts', { recursive: true });
const browser = await chromium.launch();
try {
  const page = await browser.newPage({
    viewport: { width: 1280, height: 900 },
    reducedMotion: 'reduce',
  });
  await page.goto('http://127.0.0.1:1427');
  await page.getByText('Minecraft server', { exact: true }).waitFor();
  await page.screenshot({ path: 'docs/preview.png', fullPage: true });
  await page.getByRole('button', { name: 'Open a port', exact: true }).click();
  await page.getByLabel('Rule name').fill('Minecraft server');
  await page.getByLabel('Port number').fill('25565');
  await page.getByRole('button', { name: 'Review rule' }).click();
  await page.getByRole('heading', { name: 'Ready to open this port?' }).waitFor();
  await page.screenshot({ path: 'docs/confirmation.png' });
  await page.keyboard.press('Escape');
  await page.setViewportSize({ width: 390, height: 844 });
  await page.screenshot({ path: 'artifacts/polish-mobile.png', fullPage: true });
  await page.setViewportSize({ width: 1280, height: 900 });
  await page.getByRole('button', { name: 'Help & getting started' }).click();
  await page.screenshot({ path: 'artifacts/polish-help.png', fullPage: true });
} finally {
  await browser.close();
}
