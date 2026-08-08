import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

import { afterEach, describe, expect, it } from "vitest";

import { runCli } from "../../../src/cli/run-cli.js";

const temporaryRoots: string[] = [];
const fixturePath = fileURLToPath(
  new URL("../../brick-and-steel.pdf", import.meta.url),
);

afterEach(async () => {
  await Promise.all(
    temporaryRoots.splice(0).map((path) =>
      rm(path, { recursive: true, force: true })
    ),
  );
});

describe("runCli", () => {
  it.each([
    ["help", ["--help"], "Usage: featherweight"],
    ["version", ["--version"], "1.2.3-test\n"],
  ])("writes %s output and exits successfully", async (_name, arguments_, output) => {
    const stdout: string[] = [];
    const stderr: string[] = [];

    const exitCode = await runCli(arguments_, {
      cwd: process.cwd(),
      version: "1.2.3-test",
      writeStdout: async (value) => {
        stdout.push(value);
      },
      writeStderr: async (value) => {
        stderr.push(value);
      },
    });

    expect(exitCode).toBe(0);
    expect(stdout).toHaveLength(1);
    expect(stdout[0]).toContain(output);
    expect(stderr).toEqual([]);
  });

  it("returns exit 2 for invalid arguments", async () => {
    const stderr: string[] = [];

    const exitCode = await runCli([], {
      cwd: process.cwd(),
      version: "1.2.3-test",
      writeStdout: async () => undefined,
      writeStderr: async (value) => {
        stderr.push(value);
      },
    });

    expect(exitCode).toBe(2);
    expect(stderr).toEqual([
      "featherweight: Invalid arguments. Run 'featherweight --help' for usage.\n",
    ]);
  });

  it("returns exit 1 when the input cannot be read", async () => {
    const temporaryRoot = await createTemporaryRoot();
    const stderr: string[] = [];

    const exitCode = await runCli(["missing.pdf"], {
      cwd: temporaryRoot,
      version: "1.2.3-test",
      writeStdout: async () => undefined,
      writeStderr: async (value) => {
        stderr.push(value);
      },
    });

    expect(exitCode).toBe(1);
    expect(stderr).toEqual(["featherweight: Unable to read input PDF.\n"]);
  });

  it("returns exit 1 when stdout rejects the artifact", async () => {
    const stderr: string[] = [];

    const exitCode = await runCli([fixturePath], {
      cwd: process.cwd(),
      version: "1.2.3-test",
      writeStdout: async () => {
        throw new Error("stdout unavailable");
      },
      writeStderr: async (value) => {
        stderr.push(value);
      },
    });

    expect(exitCode).toBe(1);
    expect(stderr).toEqual(["featherweight: Unable to write output.\n"]);
  });

  it("returns the intended exit code when stderr rejects an error", async () => {
    const exitCode = await runCli([], {
      cwd: process.cwd(),
      version: "1.2.3-test",
      writeStdout: async () => undefined,
      writeStderr: async () => {
        throw new Error("stderr unavailable");
      },
    });

    expect(exitCode).toBe(2);
  });
});

async function createTemporaryRoot(): Promise<string> {
  const temporaryRoot = await mkdtemp(join(tmpdir(), "featherweight-run-cli-"));
  temporaryRoots.push(temporaryRoot);
  return temporaryRoot;
}
