import { test, expect } from '@playwright/test';

test('opening and closing a port requires review and explicit confirmation', async ({ page }) => {
  const errors: string[] = [];
  page.on('pageerror', (e) => errors.push(e.message));
  await page.goto('/');
  await expect(page.getByText('Browser preview', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Open a port', exact: true }).click();
  await page.getByLabel('Rule name').fill('My test server');
  await page.getByLabel('Port number').fill('3000');
  await page.getByRole('button', { name: 'Both', exact: true }).click();
  await page.getByRole('button', { name: 'Advanced options' }).click();
  await page.getByLabel('Internal port').fill('3001');
  await page.getByRole('button', { name: 'Review rule' }).click();
  await expect(page.getByRole('heading', { name: 'Ready to open this port?' })).toBeVisible();
  expect(await page.evaluate(() => localStorage.getItem('epf-preview-v1'))).toBeNull();
  await page.getByRole('button', { name: 'Open port', exact: true }).click();
  await expect(page.getByRole('dialog')).not.toBeVisible();
  await expect(page.getByRole('row').filter({ hasText: 'My test server' })).toHaveCount(2);
  const row = page
    .getByRole('row')
    .filter({ hasText: 'My test server' })
    .filter({ hasText: 'TCP' });
  await row.getByRole('button', { name: 'Close My test server port', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Close this port?' })).toBeVisible();
  await page.getByRole('button', { name: 'Keep open' }).click();
  await expect(row.getByText('Active', { exact: true })).toBeVisible();
  await row.getByRole('button', { name: 'Close My test server port', exact: true }).click();
  await page.getByRole('button', { name: 'Close port', exact: true }).click();
  await expect(row.getByText('Inactive', { exact: true })).toBeVisible();
  await page.reload();
  await expect(page.getByRole('row').filter({ hasText: 'My test server' })).toHaveCount(2);
  expect(errors).toEqual([]);
});

test('invalid ports are rejected and profiles never open a mapping', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Open a port', exact: true }).click();
  await page.getByLabel('Port number').fill('65536');
  await page.getByRole('button', { name: 'Review rule' }).click();
  await expect(page.getByRole('alert')).toContainText('between 1 and 65535');
  await page.getByLabel('Rule name').fill('Saved game');
  await page.getByLabel('Port number').fill('27015');
  await page.getByRole('button', { name: 'Save as profile' }).click();
  await page.getByRole('button', { name: 'Saved profiles' }).click();
  await expect(page.getByRole('heading', { name: 'Saved game' })).toBeVisible();
  const saved = await page.evaluate(() => JSON.parse(localStorage.getItem('epf-preview-v1')!));
  expect(saved.rules).toHaveLength(2);
  expect(saved.profiles).toHaveLength(1);
  await page.getByRole('button', { name: 'Use profile' }).click();
  await expect(page.getByLabel('Port number')).toHaveValue('27015');
  await page.getByRole('button', { name: 'Close dialog' }).click();
  await page.getByRole('button', { name: 'Delete Saved game profile' }).click();
  await expect(
    page.getByRole('heading', { name: 'Keep your favorite setups handy.' }),
  ).toBeVisible();
});

test('search, filters and diagnostics work without router changes', async ({ page }) => {
  await page.goto('/');
  await page.getByLabel('Search rules').fill('valheim');
  await expect(page.getByRole('row').filter({ hasText: 'Valheim' })).toHaveCount(1);
  await expect(page.getByRole('row').filter({ hasText: 'Minecraft server' })).toHaveCount(0);
  await page.getByLabel('Search rules').clear();
  await page.getByRole('button', { name: 'Active', exact: true }).click();
  await expect(page.getByRole('row').filter({ hasText: 'Valheim' })).toHaveCount(0);
  await page.getByRole('button', { name: 'Network diagnostics', exact: true }).click();
  await page.getByRole('button', { name: 'Run diagnostics' }).click();
  await expect(page.getByText('Not requested', { exact: true })).toBeVisible();
  await page.getByRole('checkbox').check();
  await page.getByRole('button', { name: 'Run diagnostics' }).click();
  await expect(page.getByText('Sample result:', { exact: false })).toBeVisible();
  await expect(
    page.getByText('Not tested. No external port-testing service is configured in this release.'),
  ).toBeVisible();
  expect(await page.evaluate(() => localStorage.getItem('epf-preview-v1'))).toBeNull();
});

test('unsupported routers show a useful explanation and allow saving a profile', async ({
  page,
}) => {
  await page.goto('/?scenario=offline');
  await expect(page.getByText('UPnP unavailable', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Open a port', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Review rule' })).toBeDisabled();
  await page.getByLabel('Port number').fill('25565');
  await page.getByRole('button', { name: 'Save as profile' }).click();
  await expect(page.getByRole('dialog')).not.toBeVisible();
});

test('layout fits a small viewport and dialogs support keyboard dismissal', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto('/');
  await expect(page.getByRole('heading', { name: 'Open a port. Get connected.' })).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(
    true,
  );
  await page.getByRole('button', { name: 'Open a port', exact: true }).click();
  await expect(page.getByRole('dialog')).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(page.getByRole('dialog')).not.toBeVisible();
});
