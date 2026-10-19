import { copyFile, mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { execFileSync } from 'node:child_process';
import { gzipSync, gunzipSync } from 'node:zlib';
import { readHistory, validateHistory, writeHistory } from './history-data.mjs';

const tag = 'benchmark-data';
const asset = 'history.json.gz';
const historyPath = 'site/data/history.json';
const archiveDirectory = '.history';
const repository = process.env.GITHUB_REPOSITORY;
const operation = process.argv[2];
if (!['download', 'upload'].includes(operation) || process.argv.length !== 3) {
  throw new Error('Usage: node scripts/history.mjs <download|upload>');
}
if (!repository) throw new Error('GITHUB_REPOSITORY must be set to OWNER/REPO');

function gh(...argumentsList) {
  return execFileSync('gh', argumentsList, { encoding: 'utf8', maxBuffer: 16 * 1024 * 1024 }).trim();
}
function findRelease() {
  const pages = JSON.parse(gh('api', '--paginate', '--slurp', `repos/${repository}/releases?per_page=100`));
  return pages.flat().find(release => release.tag_name === tag);
}
async function readOptional(path) {
  try {
    return await readFile(path);
  } catch (error) {
    if (error.code === 'ENOENT') return null;
    throw error;
  }
}
async function download() {
  const release = findRelease();
  await mkdir(archiveDirectory, { recursive: true });
  if (!release) {
    if (await readOptional(historyPath) !== null) throw new Error('Refusing to overwrite local history during bootstrap');
    await writeHistory(historyPath, { schema: 1, runs: [] });
    return;
  }
  if (!release.assets.some(entry => entry.name === asset)) {
    throw new Error('The data release exists but its history asset is missing. Restore the backup before running again');
  }
  const temporary = await mkdtemp(join(archiveDirectory, 'download-'));
  try {
    gh('release', 'download', tag, '--repo', repository, '--pattern', asset, '--dir', temporary);
    const archivePath = join(temporary, asset);
    const data = validateHistory(JSON.parse(gunzipSync(await readFile(archivePath)).toString('utf8')));
    await copyFile(archivePath, join(archiveDirectory, 'previous-history.json.gz'));
    await copyFile(archivePath, join(archiveDirectory, asset));
    await writeHistory(historyPath, data);
  } finally {
    await rm(temporary, { recursive: true, force: true });
  }
}
async function upload() {
  const history = await readHistory(historyPath);
  if (!history.runs.length) throw new Error('Refusing to publish an empty history');
  const previous = await readOptional(join(archiveDirectory, 'previous-history.json.gz'));
  if (previous) {
    const previousHistory = validateHistory(JSON.parse(gunzipSync(previous).toString('utf8')));
    const identifiers = new Set(history.runs.map(run => run.id));
    if (previousHistory.runs.some(run => !identifiers.has(run.id))) {
      throw new Error('Refusing to publish history that drops existing runs');
    }
  }
  await mkdir(archiveDirectory, { recursive: true });
  const archivePath = join(archiveDirectory, asset);
  const archive = gzipSync(await readFile(historyPath));
  await writeFile(archivePath, archive);
  if (!findRelease()) {
    if (!process.env.GITHUB_SHA) throw new Error('GITHUB_SHA must be set to create the data release');
    gh('release', 'create', tag, archivePath, '--repo', repository,
      '--target', process.env.GITHUB_SHA, '--title', 'Benchmark history',
      '--notes', 'Machine-generated layout benchmark history. The daily workflow replaces history.json.gz. Backups are kept as Actions artifacts.',
      '--prerelease', '--latest=false');
  } else {
    gh('release', 'upload', tag, archivePath, '--repo', repository, '--clobber');
  }
  const temporary = await mkdtemp(join(archiveDirectory, 'verification-'));
  try {
    gh('release', 'download', tag, '--repo', repository, '--pattern', asset, '--dir', temporary);
    if (!(await readFile(join(temporary, asset))).equals(archive)) throw new Error('Uploaded history did not match the local archive');
  } finally {
    await rm(temporary, { recursive: true, force: true });
  }
}
await (operation === 'download' ? download() : upload());
