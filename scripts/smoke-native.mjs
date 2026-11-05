// Read-only smoke check for an EPF WebView2 instance launched with remote debugging.
import assert from 'node:assert/strict';
import { log } from 'node:console';
import process from 'node:process';
import { chromium } from '@playwright/test';

const browser = await chromium.connectOverCDP(process.env.EPF_CDP_URL ?? 'http://127.0.0.1:9227');
try {
  const page = browser.contexts()[0].pages()[0];
  assert.ok(page, 'EPF must have an open WebView2 page.');
  const errors = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await page.getByRole('heading', { name: 'Open a port. Get connected.' }).waitFor();
  assert.equal(await page.getByText('Browser preview', { exact: true }).count(), 0);
  await page.getByRole('button', { name: 'Refresh rules' }).waitFor({ timeout: 45000 });
  const refused = await page.evaluate(async () => {
    try {
      await globalThis.__TAURI_INTERNALS__.invoke('open_ports', {
        request: {
          name: 'Consent boundary check',
          externalPort: 25565,
          internalPort: 25565,
          protocol: 'tcp',
          destinationIp: null,
          leaseSeconds: 3600,
        },
        confirmed: false,
      });
      return null;
    } catch (error) {
      return String(error);
    }
  });
  assert.match(refused ?? '', /Confirm this operation/);
  await page.getByRole('button', { name: 'Network diagnostics', exact: true }).click();
  await page.getByRole('button', { name: 'Run diagnostics' }).click();
  await page.getByRole('heading', { name: 'Here’s what we found' }).waitFor({ timeout: 45000 });
  assert.equal(await page.getByText('Not requested', { exact: true }).count(), 1);
  assert.deepEqual(errors, []);
  await page.screenshot({ path: 'artifacts/native-diagnostics.png', fullPage: true });
  log(
    'Native UI → Tauri IPC → Rust diagnostics passed. Unconfirmed opening was refused. No router rules were changed.',
  );
} finally {
  await browser.close();
}
