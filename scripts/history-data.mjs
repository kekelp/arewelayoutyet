import { mkdir, readFile, rename, rm, writeFile } from 'node:fs/promises';
import { dirname } from 'node:path';
import { randomUUID } from 'node:crypto';

export function validateHistory(data) {
  if (data?.schema !== 1 || !Array.isArray(data.runs)) throw new Error('Unsupported or invalid benchmark history');
  const identifiers = new Set();
  for (const run of data.runs) {
    if (typeof run?.id !== 'string' || identifiers.has(run.id)) throw new Error('Invalid or duplicate run ID');
    identifiers.add(run.id);
    if (typeof run.date !== 'string' || !Number.isFinite(Date.parse(run.date)) || !Array.isArray(run.results) || !run.results.length) {
      throw new Error('Invalid benchmark run');
    }
  }
  return data;
}

export async function readHistory(path, initialize = false) {
  let content;
  try {
    content = await readFile(path, 'utf8');
  } catch (error) {
    if (initialize && error.code === 'ENOENT') return { schema: 1, runs: [] };
    throw error;
  }
  return validateHistory(JSON.parse(content));
}

export async function writeHistory(path, history) {
  validateHistory(history);
  await mkdir(dirname(path), { recursive: true });
  const temporary = `${path}.${randomUUID()}.tmp`;
  try {
    await writeFile(temporary, `${JSON.stringify(history, null, 2)}\n`);
    await rename(temporary, path);
  } finally {
    await rm(temporary, { force: true });
  }
}
