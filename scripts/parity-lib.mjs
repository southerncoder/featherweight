// Shared helpers for the TS <-> Rust parity scripts.
import path from 'node:path';
import { execFileSync, spawnSync } from 'node:child_process';

/** Builds the Rust `stage` example and returns a runner for it. */
export function stageRunner(rustRoot) {
  execFileSync('cargo', ['build', '-q', '--example', 'stage'], { cwd: rustRoot, stdio: 'inherit' });
  const stageBin = path.join(rustRoot, 'target', 'debug', 'examples', process.platform === 'win32' ? 'stage.exe' : 'stage');

  return function rust(stageArgs, input) {
    const result = spawnSync(stageBin, stageArgs, {
      input: input === undefined ? '' : JSON.stringify(input),
      encoding: 'utf8',
      maxBuffer: 1 << 28,
    });
    if (result.status !== 0) {
      return { crashed: (result.stderr || `exit ${result.status}`).trim().split(/\r?\n/).slice(-3).join(' | ') };
    }
    return { output: result.stdout };
  };
}

// Rust span offsets are UTF-8 byte offsets; TS uses UTF-16. Convert TS values.
export function toByteOffsets(value) {
  if (Array.isArray(value)) return value.map(toByteOffsets);
  if (value === null || typeof value !== 'object') return value;
  const out = {};
  for (const [key, child] of Object.entries(value)) out[key] = toByteOffsets(child);
  if (typeof value.text === 'string' && Array.isArray(value.spans)) {
    out.spans = value.spans.map((span) => ({
      ...toByteOffsets(span),
      start: Buffer.byteLength(value.text.slice(0, span.start)),
      end: Buffer.byteLength(value.text.slice(0, span.end)),
    }));
  }
  return out;
}

export function diff(a, b, at = '$', out = []) {
  if (out.length >= 20) return out;
  if (typeof a === 'number' && typeof b === 'number') {
    if (!(a === b || Math.abs(a - b) <= 1e-9 * Math.max(1, Math.abs(a)))) out.push(`${at}: ${a} != ${b}`);
  } else if (Array.isArray(a) || Array.isArray(b)) {
    if (!Array.isArray(a) || !Array.isArray(b)) out.push(`${at}: array vs ${typeof b}`);
    else {
      if (a.length !== b.length) out.push(`${at}.length: ${a.length} != ${b.length}`);
      for (let i = 0; i < Math.min(a.length, b.length); i++) diff(a[i], b[i], `${at}[${i}]`, out);
    }
  } else if (a && b && typeof a === 'object' && typeof b === 'object') {
    for (const key of new Set([...Object.keys(a), ...Object.keys(b)])) {
      if (!(key in b)) out.push(`${at}.${key}: missing in Rust`);
      else if (!(key in a)) out.push(`${at}.${key}: unexpected in Rust`);
      else diff(a[key], b[key], `${at}.${key}`, out);
    }
  } else if (a !== b) {
    out.push(`${at}: ${JSON.stringify(a)?.slice(0, 120)} != ${JSON.stringify(b)?.slice(0, 120)}`);
  }
  return out;
}

export function textDiff(a, b) {
  if (a === b) return [];
  const la = a.split('\n'), lb = b.split('\n');
  for (let i = 0; i < Math.max(la.length, lb.length); i++) {
    if (la[i] !== lb[i]) return [`line ${i + 1}:\n      TS  ${JSON.stringify(la[i])}\n      RS  ${JSON.stringify(lb[i])}`];
  }
  return ['(trailing difference)'];
}

