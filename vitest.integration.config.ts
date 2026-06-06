import path from "node:path";
import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
  test: {
    environment: "jsdom",
    setupFiles: ["./tests/setupGlobals.ts", "./tests/setupTests.ts"],
    globals: true,
    include: ["tests/integration/**/*.web-server.test.tsx"],
    // Each suite spawns a REAL `cargo run` web-server (plus mock upstream /
    // WebDAV servers) and drives heavy disk + HTTP I/O. Running all ~20 suites
    // in parallel saturates CPU/IO on constrained CI runners, so individual
    // `waitFor`s (e.g. the WebDAV upload-success toast) flakily exceed their
    // timeout even though they pass reliably in isolation. Run suites
    // sequentially (one live server at a time) for a deterministic gate; the
    // tests within a file already use `describe.sequential`.
    fileParallelism: false,
  },
});
