import { cp, mkdir, rm, stat, writeFile } from 'node:fs/promises';

if (!(await stat('dist/index.html')).isFile()) throw new Error('Build the website before preparing a Vercel deployment');
await rm('.vercel/output', { recursive: true, force: true });
await mkdir('.vercel/output', { recursive: true });
await cp('dist', '.vercel/output/static', { recursive: true });
await writeFile('.vercel/output/config.json', `${JSON.stringify({ version: 3 })}\n`);
