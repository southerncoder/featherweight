import { defineConfig } from "vitest/config";

// `PARITY_CAPTURE=<file> vitest run` records every pipeline-stage call the
// tests make, for the Rust port's replay check (`scripts/parity-replay.mjs`).
const setupFiles = process.env.PARITY_CAPTURE
  ? ["./scripts/parity-capture.ts"]
  : [];

export default defineConfig({
  test: {
    coverage: {
      provider: "v8",
      reportsDirectory: "coverage",
    },
    projects: [
      {
        test: {
          name: "unit",
          environment: "node",
          setupFiles,
          include: ["test/unit/**/*.test.ts"],
        },
      },
      {
        test: {
          name: "integration",
          environment: "node",
          setupFiles,
          include: ["test/integration/**/*.test.ts"],
        },
      },
    ],
  },
});
