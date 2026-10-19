import { cp, mkdir, readFile, rm, stat, writeFile } from 'node:fs/promises';
import { execFileSync } from 'node:child_process';
import assert from 'node:assert/strict';

const features = JSON.parse(await readFile('site/data/features.json', 'utf8'));
assert(features.libraries.length > 0, 'Missing library definitions');
assert.equal(new Set(features.libraries.map(library => library.id)).size, features.libraries.length);
for (const library of features.libraries) {
  for (const field of ['url', 'source']) assert.equal(new URL(library[field]).protocol, 'https:');
}
for (const feature of features.features) {
  assert.equal(feature.support.length, features.libraries.length, feature.name);
  assert(feature.support.every(status => ['yes', 'no', 'different', 'unknown'].includes(status)), feature.name);
  assert(feature.support.includes('yes'), `No library supports ${feature.name}`);
  assert(feature.note, `Missing explanation for ${feature.name}`);
}
execFileSync(process.execPath, ['--check', 'site/app.js'], { stdio: 'inherit' });
const html = await readFile('site/index.html', 'utf8');
for (const match of html.matchAll(/(?:href|src)="([^"#]+)"/g)) {
  const path = match[1];
  if (path.includes(':') || path === './' || path === 'data/history.json') continue;
  assert((await stat(`site/${path}`)).isFile(), `Missing asset ${path}`);
}
await rm('dist', { recursive: true, force: true });
await mkdir('dist', { recursive: true });
await cp('site', 'dist', { recursive: true });
const generatedAt = new Date().toISOString();
await writeFile('dist/index.html', html
  .replaceAll('{{generated-at}}', generatedAt)
  .replaceAll('{{generated-date}}', generatedAt.slice(0, 10)));

