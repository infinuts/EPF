import { readFileSync } from 'node:fs';
import { log } from 'node:console';
import process from 'node:process';
import assert from 'node:assert/strict';

const read = (path) => readFileSync(new URL(`../${path}`, import.meta.url), 'utf8');
const pkg = JSON.parse(read('package.json'));
const lock = JSON.parse(read('package-lock.json'));
const tauri = JSON.parse(read('src-tauri/tauri.conf.json'));
const workspace = read('Cargo.toml').match(/\[workspace.package\]([\s\S]*?)(?=\n\[|$)/)?.[1];
const rustVersion = workspace?.match(/^version\s*=\s*"([^"]+)"/m)?.[1];

for (const [name, version] of Object.entries({
  'Cargo workspace': rustVersion,
  'Tauri config': tauri.version,
  'npm lockfile': lock.version,
  'npm root package': lock.packages[''].version,
})) {
  assert.equal(version, pkg.version, `${name} must match package.json (${pkg.version}).`);
}
if (process.env.GITHUB_REF_TYPE === 'tag') {
  assert.equal(
    process.env.GITHUB_REF_NAME,
    `v${pkg.version}`,
    'Release tag must match the app version.',
  );
}
log(`EPF version ${pkg.version} is consistent across the workspace.`);
