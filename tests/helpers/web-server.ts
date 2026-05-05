import fs from "node:fs/promises";
import net from "node:net";
import os from "node:os";
import path from "node:path";
import { spawn, type ChildProcess } from "node:child_process";
import { fileURLToPath } from "node:url";

const repoRoot = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
  "..",
);

const distWebDir = path.join(repoRoot, "dist-web");

export type TestWebServer = {
  baseUrl: string;
  dataDir: string;
  homeDir: string;
  stop: () => Promise<void>;
};

type StartTestWebServerOptions = {
  env?: Record<string, string>;
};

async function ensureDistWeb(): Promise<void> {
  await fs.access(path.join(distWebDir, "index.html"));
}

async function getFreePort(): Promise<number> {
  return await new Promise((resolve, reject) => {
    const server = net.createServer();
    server.unref();
    server.on("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      if (!address || typeof address === "string") {
        reject(new Error("Failed to allocate free port"));
        return;
      }
      const { port } = address;
      server.close((error) => {
        if (error) {
          reject(error);
          return;
        }
        resolve(port);
      });
    });
  });
}

async function waitForServer(
  baseUrl: string,
  child: ChildProcess,
  timeoutMs = 300_000,
): Promise<void> {
  const deadline = Date.now() + timeoutMs;

  while (Date.now() < deadline) {
    if (child.exitCode !== null) {
      throw new Error(`web server exited early with code ${child.exitCode}`);
    }

    try {
      const response = await fetch(new URL("/api/health", baseUrl));
      if (response.ok) {
        return;
      }
    } catch {
      // Retry until timeout.
    }

    await new Promise((resolve) => setTimeout(resolve, 500));
  }

  throw new Error(`Timed out waiting for web server at ${baseUrl}`);
}

async function stopChild(
  child: ChildProcess,
  timeoutMs = 10_000,
): Promise<void> {
  if (child.exitCode !== null) {
    return;
  }

  child.kill("SIGTERM");
  const start = Date.now();

  while (child.exitCode === null && Date.now() - start < timeoutMs) {
    await new Promise((resolve) => setTimeout(resolve, 100));
  }

  if (child.exitCode === null) {
    child.kill("SIGKILL");
  }
}

export async function startTestWebServer(
  options: StartTestWebServerOptions = {},
): Promise<TestWebServer> {
  await ensureDistWeb();

  const port = await getFreePort();
  const host = "127.0.0.1";
  const dataDir = await fs.mkdtemp(
    path.join(os.tmpdir(), "cc-switch-web-page-data-"),
  );
  const homeDir = await fs.mkdtemp(
    path.join(os.tmpdir(), "cc-switch-web-page-home-"),
  );
  const baseUrl = `http://${host}:${port}`;

  const child = spawn(
    "cargo",
    [
      "run",
      "--quiet",
      "--manifest-path",
      "src-tauri/Cargo.toml",
      "--no-default-features",
      "--features",
      "web-server",
      "--example",
      "server",
    ],
    {
      cwd: repoRoot,
      env: {
        ...process.env,
        HOST: host,
        PORT: String(port),
        ENABLE_HSTS: "false",
        RUSTFLAGS: process.env.RUSTFLAGS || "-Awarnings",
        CC_SWITCH_DATA_DIR: dataDir,
        CC_SWITCH_TEST_HOME: homeDir,
        CC_SWITCH_WEB_DIST_DIR: distWebDir,
        ...(options.env ?? {}),
      },
      stdio: ["ignore", "pipe", "pipe"],
    },
  );

  child.stdout.on("data", (chunk) => {
    process.stderr.write(`[page-web-server] ${chunk}`);
  });
  child.stderr.on("data", (chunk) => {
    process.stderr.write(`[page-web-server] ${chunk}`);
  });

  await waitForServer(baseUrl, child);

  return {
    baseUrl,
    dataDir,
    homeDir,
    stop: () => stopChild(child),
  };
}
