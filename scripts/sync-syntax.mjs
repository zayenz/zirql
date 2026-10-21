import { execFileSync } from 'node:child_process';
import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const [mode, revision, ...hosts] = process.argv.slice(2);
if (!['sync', 'check'].includes(mode) || !/^[a-f0-9]{40}$/.test(revision ?? '') || hosts.length === 0) {
  throw new Error('Usage: node scripts/sync-syntax.mjs sync|check <full-Zirql-commit> <host-root> [...]');
}
const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const asset = 'assets/syntax/zirql.tmLanguage.json';
const marker = 'assets/syntax/zirql-revision.txt';
// Read committed bytes so an uncommitted grammar cannot be labelled with an older revision.
const grammar = execFileSync('git', ['show', `${revision}:${asset}`], { cwd: root });
JSON.parse(grammar.toString());
for (const host of hosts) {
  const destination = resolve(host);
  if (destination === root) throw new Error('Choose a consumer repository, not Zirql itself');
  if (mode === 'sync') {
    writeFileSync(resolve(destination, asset), grammar);
    writeFileSync(resolve(destination, marker), `${revision}\n`);
  } else if (!readFileSync(resolve(destination, asset)).equals(grammar)
    || readFileSync(resolve(destination, marker), 'utf8') !== `${revision}\n`) {
    throw new Error(`Zirql syntax differs from ${revision}: ${destination}`);
  }
  console.log(`${mode}: ${destination} (${revision})`);
}
