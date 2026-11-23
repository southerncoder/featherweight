// Stage 0: raw extraction items — npm @firecrawl/pdf-inspector vs the pdf-inspector crate.
import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const require = createRequire(path.join(root, 'package.json'));
const inspector = require('@firecrawl/pdf-inspector');
let failed = 0;
for (const arg of process.argv.slice(2)) {
  const pdf = path.resolve(arg);
  const bytes = fs.readFileSync(pdf);
  const ts = inspector.extractTextWithPositions(bytes);
  const rs = JSON.parse(execFileSync('cargo', ['run', '-q', '--example', 'items', '--', pdf], { cwd: path.join(root, 'rust'), encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'] }));
  const keys = ['text', 'x', 'y', 'width', 'height', 'font', 'fontSize', 'page', 'isBold', 'isItalic', 'isUnderline', 'isStrikeout', 'itemType'];
  let bad = 0;
  for (let i = 0; i < Math.max(ts.length, rs.length); i++) {
    const diff = keys.filter((k) => {
      const a = ts[i]?.[k], b = rs[i]?.[k];
      return typeof a === 'number' ? Math.abs(a - b) > 1e-3 : String(a) !== String(b);
    });
    if (diff.length && bad++ < 5) console.log(`  #${i} ${diff.join(',')}:`, JSON.stringify(ts[i]), '\n    vs', JSON.stringify(rs[i]));
  }
  console.log(`${path.basename(pdf)}: items TS=${ts.length} RS=${rs.length} mismatches=${bad}`);
  failed += bad;
}
process.exitCode = failed ? 1 : 0;
