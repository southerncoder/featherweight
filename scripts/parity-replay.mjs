#!/usr/bin/env node
// Replays every pipeline-stage call the TypeScript test suite makes through the
// Rust port, so the Rust crate is held to all TS unit and integration cases, not
// just the fixture PDFs.
//
//   node scripts/parity-replay.mjs            # capture (runs vitest) + replay
//   node scripts/parity-replay.mjs --verbose  # more diffs per failure
//
// Capture: `PARITY_CAPTURE=<file> vitest run` (see scripts/parity-capture.ts).
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

import { diff, stageRunner, textDiff, toByteOffsets } from './parity-lib.mjs';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const rustRoot = process.env.PARITY_RUST_ROOT ? path.resolve(process.env.PARITY_RUST_ROOT) : path.join(root, 'rust');
const verbose = process.argv.includes('--verbose');
const captureFile = path.join(fs.mkdtempSync(path.join(os.tmpdir(), 'featherweight-parity-')), 'capture.jsonl');

// A single command string: pnpm is a .cmd shim on Windows and needs a shell.
const vitest = spawnSync('pnpm -s vitest run', {
  cwd: root,
  env: { ...process.env, PARITY_CAPTURE: captureFile },
  encoding: 'utf8',
  shell: true,
});
if (vitest.status !== 0) {
  console.error(vitest.stdout, vitest.stderr);
  throw new Error('TypeScript test suite failed during capture');
}

const seen = new Set();
const cases = fs.readFileSync(captureFile, 'utf8').trim().split('\n').filter((line) => {
  const key = line.replace(/"output":.*$|"error":.*$/, '');
  return !seen.has(key) && seen.add(key);
}).map((line) => JSON.parse(line));

const rust = stageRunner(rustRoot);
const toRustInput = {
  physical: (input) => input,
  normalize: (input) => ({ ...input, physicalText: toByteOffsets(input.physicalText) }),
  layout: toByteOffsets,
  recognize: (input) => ({ ...input, normalizedText: toByteOffsets(input.normalizedText) }),
  fountain: (input) => input,
  fdx: (input) => input,
};
const toRustOutput = { physical: toByteOffsets, normalize: toByteOffsets };

const tally = {};
const failures = [];
for (const [index, testCase] of cases.entries()) {
  const { stage, input } = testCase;
  const expected = testCase.error === undefined ? (toRustOutput[stage] ?? ((v) => v))(testCase.output) : `ERROR ${testCase.error}`;
  const { output, crashed } = rust([stage], toRustInput[stage](input));
  let problems;
  if (crashed) {
    // Lone UTF-16 surrogates in TS strings cannot exist in Rust strings.
    problems = /lone leading surrogate|unexpected end of hex escape|invalid unicode code point/.test(crashed)
      ? null
      : [`crashed: ${crashed}`];
  } else if (testCase.error !== undefined && stage !== 'fdx') {
    problems = [`TS threw ${testCase.error}, Rust returned output`];
  } else {
    problems = typeof expected === 'string' ? textDiff(expected, output) : diff(expected, JSON.parse(output));
  }
  const bucket = (tally[stage] ??= { pass: 0, fail: 0, skipped: 0 });
  if (problems === null) bucket.skipped++;
  else if (problems.length) {
    bucket.fail++;
    failures.push({ index, stage, problems });
  } else bucket.pass++;
}

for (const [stage, { pass, fail, skipped }] of Object.entries(tally)) {
  console.log(`${stage.padEnd(10)} ${pass} pass, ${fail} fail${skipped ? `, ${skipped} not representable in Rust` : ''}`);
}
for (const f of failures.slice(0, verbose ? 50 : 10)) {
  console.log(`\n✗ case ${f.index} / ${f.stage}`);
  for (const p of f.problems.slice(0, verbose ? 20 : 3)) console.log(`    ${p}`);
}
const total = Object.values(tally).reduce((sum, t) => sum + t.pass + t.fail, 0);
console.log(`\n${total - failures.length}/${total} replayed TS test cases match`);
process.exitCode = failures.length ? 1 : 0;
