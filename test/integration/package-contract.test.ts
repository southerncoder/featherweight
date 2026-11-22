import { exec, execFile } from "node:child_process";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";

import { expect, test } from "vitest";

const executeFile = promisify(execFile);
const executeCommand = promisify(exec);
const repositoryRoot = fileURLToPath(new URL("../../", import.meta.url));

test("packs only the supported distribution and project documentation", async () => {
  const npmCache = await mkdtemp(join(tmpdir(), "featherweight-npm-cache-"));

  try {
    // Node refuses to spawn Windows .cmd shims without a shell (CVE-2024-27980),
    // so on Windows run npm through one, as a single quoted command string.
    const { stdout } =
      process.platform === "win32"
        ? await executeCommand(`npm pack --dry-run --json --cache "${npmCache}"`, {
            cwd: repositoryRoot,
            encoding: "utf8",
          })
        : await executeFile("npm", ["pack", "--dry-run", "--json", "--cache", npmCache], {
            cwd: repositoryRoot,
            encoding: "utf8",
          });
    const packResults = JSON.parse(stdout) as readonly PackResult[];
    const packResult = packResults[0];

    expect(packResult).toBeDefined();

    const packedFiles = new Set(
      packResult?.files.map((file) => file.path) ?? [],
    );
    const requiredFiles = [
      "dist/index.js",
      "dist/index.d.ts",
      "dist/cli.js",
      "CHANGELOG.md",
      "LICENSE",
      "README.md",
      "THIRD_PARTY_NOTICES.md",
    ];

    for (const requiredFile of requiredFiles) {
      expect(packedFiles.has(requiredFile), requiredFile).toBe(true);
    }

    for (const packedFile of packedFiles) {
      expect(
        packedFile === "package.json" ||
          requiredFiles.includes(packedFile) ||
          packedFile.startsWith("dist/"),
        packedFile,
      ).toBe(true);
      expect(packedFile).not.toMatch(/^(?:src|test|corpus)\//u);
    }

    const packageManifest = JSON.parse(
      await readFile(join(repositoryRoot, "package.json"), "utf8"),
    ) as PackageManifest;

    expect(packageManifest.name).toBe("@bantam-hq/featherweight");
    expect(packageManifest.type).toBe("module");
    expect(packageManifest.engines.node).toBe(">=24");
    expect(packageManifest.packageManager).toBe("pnpm@11.18.0");

    const declaredTargets = [
      packageManifest.main,
      packageManifest.types,
      ...collectStringValues(packageManifest.exports),
      ...collectStringValues(packageManifest.bin),
    ];

    for (const declaredTarget of declaredTargets) {
      expect(packedFiles.has(removeLeadingDotSlash(declaredTarget))).toBe(true);
    }
  } finally {
    await rm(npmCache, { recursive: true, force: true });
  }
});

interface PackResult {
  readonly files: readonly { readonly path: string }[];
}

interface PackageManifest {
  readonly name: string;
  readonly type: string;
  readonly engines: { readonly node: string };
  readonly packageManager: string;
  readonly main: string;
  readonly types: string;
  readonly exports: unknown;
  readonly bin: unknown;
}

function collectStringValues(value: unknown): string[] {
  if (typeof value === "string") {
    return [value];
  }

  if (typeof value !== "object" || value === null) {
    return [];
  }

  return Object.values(value).flatMap(collectStringValues);
}

function removeLeadingDotSlash(path: string): string {
  return path.startsWith("./") ? path.slice(2) : path;
}
