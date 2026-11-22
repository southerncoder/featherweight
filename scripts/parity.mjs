#!/usr/bin/env node
// TS ↔ Rust parity harness.
//
// For every fixture PDF, runs the TypeScript pipeline stage by stage, then runs
// each Rust stage on the *TypeScript* output of the previous stage. A mismatch
// therefore points at the one stage that diverges, not at everything downstream.
// Finally compares the Rust public API end to end against the TypeScript one.
//
//   node scripts/parity.mjs                 # all fixtures, all stages
//   node scripts/parity.mjs --stage layout  # one stage
//   node scripts/parity.mjs --verbose       # show up to 20 diffs per failure
//   node scripts/parity.mjs test/foo.pdf    # specific PDFs
//
// Requires a current TS build (`pnpm build`); builds the Rust examples itself.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

import { diff, stageRunner, textDiff, toByteOffsets } from './parity-lib.mjs';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
// PARITY_RUST_ROOT points the harness at another copy of the Rust workspace.
const rustRoot = process.env.PARITY_RUST_ROOT ? path.resolve(process.env.PARITY_RUST_ROOT) : path.join(root, 'rust');
const args = process.argv.slice(2);
const verbose = args.includes('--verbose');
const onlyStage = args.includes('--stage') ? args[args.indexOf('--stage') + 1] : null;
const pdfArgs = args.filter((arg, i) => arg.endsWith('.pdf') && args[i - 1] !== '--stage');
const pdfs = pdfArgs.length
  ? pdfArgs.map((p) => path.resolve(p))
  : fs.readdirSync(path.join(root, 'test')).filter((f) => f.endsWith('.pdf')).sort().map((f) => path.join(root, 'test', f));

const dist = async (p) => import(pathToFileURL(path.join(root, 'dist', p)).href);
const { extractPositionedPdfText } = await dist('pdf/extraction/pdf-inspector.js');
const { inspectScreenplayPdf } = await dist('pdf/inspection.js');
const { positionedTextToPages } = await dist('screenplay/positioned-text-pages.js');
const { groupPositionedTextIntoPhysicalLines } = await dist('core/physical-lines.js');
const { normalizePhysicalText } = await dist('pdf/normalization/physical-text.js');
const { inferScreenplayLayout } = await dist('core/layout/infer-screenplay-layout.js');
const { recognizeScreenplay } = await dist('core/recognition/recognize-screenplay.js');
const { screenplayDocumentToFountain } = await dist('fountain/screenplay-document-to-fountain.js');
const { screenplayDocumentToFDX } = await dist('fdx/screenplay-document-to-fdx.js');
const api = await dist('index.js');

const rust = stageRunner(rustRoot);

function tsErrorCode(fn) {
  try {
    return fn();
  } catch (error) {
    if (error?.code) return `ERROR ${error.code}`;
    throw error;
  }
}

const results = [];
function check(name, stage, compare) {
  if (onlyStage && onlyStage !== stage) return;
  let problems;
  try {
    problems = compare();
  } catch (error) {
    problems = [`harness error: ${error.message}`];
  }
  results.push({ name, stage, problems });
}

for (const pdf of pdfs) {
  const name = path.basename(pdf, '.pdf');
  const bytes = fs.readFileSync(pdf);
  const inspection = inspectScreenplayPdf(bytes);
  const ocr = [...inspection.pagesNeedingOcr].sort((x, y) => x - y);
  const native = [...Array(inspection.pageCount).keys()].filter((p) => !ocr.includes(p));

  // Mirror createDocumentFromPdf / createScreenplayDocument step by step.
  const extracted = native.length ? positionedTextToPages(extractPositionedPdfText(bytes, native)) : [];
  const byIndex = new Map(extracted.map((page) => [page.pageIndex, page]));
  const pages = [
    ...native.map((pageIndex) => ({ pageIndex, items: byIndex.get(pageIndex)?.items ?? [], sourceMethod: 'embedded-text' })),
    ...ocr.map((pageIndex) => ({ pageIndex, items: [], sourceMethod: 'ocr' })),
  ].sort((x, y) => x.pageIndex - y.pageIndex);
  let sourceIndex = 0;
  const positioned = {
    items: pages.flatMap((page) => page.items.map((item) => ({
      sourceIndex: sourceIndex++, sourceMethod: page.sourceMethod, pageIndex: page.pageIndex,
      text: item.text, bounds: { ...item.bounds }, font: { ...item.font }, style: { ...item.style },
    }))),
  };
  const physical = groupPositionedTextIntoPhysicalLines(positioned);
  const pageIndexes = pages.map((page) => page.pageIndex);
  const normalized = normalizePhysicalText(physical, pageIndexes);
  const layout = inferScreenplayLayout(normalized);
  const document = recognizeScreenplay(normalized, layout);

  const stageCheck = (stage, rustArgs, input, expected) => check(name, stage, () => {
    const { output, crashed } = rust(rustArgs, input);
    if (crashed) return [`crashed: ${crashed}`];
    return typeof expected === 'string' ? textDiff(expected, output) : diff(expected, JSON.parse(output));
  });

  stageCheck('physical', ['physical'], positioned, toByteOffsets(physical));
  stageCheck('normalize', ['normalize'], { physicalText: toByteOffsets(physical), pageIndexes }, toByteOffsets(normalized));
  stageCheck('layout', ['layout'], toByteOffsets(normalized), layout);
  stageCheck('recognize', ['recognize'], { normalizedText: toByteOffsets(normalized), layout }, document);
  stageCheck('fountain', ['fountain'], document, screenplayDocumentToFountain(document));
  stageCheck('fdx', ['fdx'], document, tsErrorCode(() => screenplayDocumentToFDX(document)));

  for (const format of ['json', 'fountain', 'fdx']) {
    const fn = { json: api.screenplayToJSON, fountain: api.screenplayToFountain, fdx: api.screenplayToFDX }[format];
    stageCheck(`e2e-${format}`, ['convert', format, pdf, native.join(','), ocr.join(',')], undefined,
      tsErrorCode(() => fn(bytes, native, ocr)));
  }
}

const stages = [...new Set(results.map((r) => r.stage))];
const names = [...new Set(results.map((r) => r.name))];
const width = Math.max(...names.map((n) => n.length));
console.log(`\n${''.padEnd(width)}  ${stages.map((s) => s.padEnd(9)).join(' ')}`);
for (const name of names) {
  const row = stages.map((stage) => {
    const r = results.find((x) => x.name === name && x.stage === stage);
    return (r ? (r.problems.length ? 'FAIL' : 'ok') : '-').padEnd(9);
  });
  console.log(`${name.padEnd(width)}  ${row.join(' ')}`);
}
const failures = results.filter((r) => r.problems.length);
for (const f of failures) {
  console.log(`\n✗ ${f.name} / ${f.stage}`);
  for (const p of f.problems.slice(0, verbose ? 20 : 3)) console.log(`    ${p}`);
}
console.log(`\n${results.length - failures.length}/${results.length} checks pass`);
process.exitCode = failures.length ? 1 : 0;
