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

export async function getFreePort(): Promise<number> {
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
    if (childHasExited(child)) {
      throw new Error(
        `web server exited early (code=${String(child.exitCode)}, signal=${String(child.signalCode)})`,
      );
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

const childHasExited = (child: ChildProcess): boolean =>
  child.exitCode !== null || child.signalCode !== null;

async function stopChild(
  child: ChildProcess,
  timeoutMs = 10_000,
): Promise<void> {
  if (childHasExited(child)) return;

  child.kill("SIGTERM");
  const gracefulDeadline = Date.now() + timeoutMs;
  while (!childHasExited(child) && Date.now() < gracefulDeadline) {
    await new Promise((resolve) => setTimeout(resolve, 100));
  }

  if (!childHasExited(child)) {
    child.kill("SIGKILL");
    const killDeadline = Date.now() + 2_000;
    while (!childHasExited(child) && Date.now() < killDeadline) {
      await new Promise((resolve) => setTimeout(resolve, 50));
    }
  }

  if (!childHasExited(child)) {
    throw new Error("web server did not exit after SIGKILL");
  }
}

const errorMessage = (error: unknown): string =>
  error instanceof Error ? error.message : String(error);

async function stopAndRemoveTestState(
  child: ChildProcess,
  dataDir: string,
  homeDir: string,
): Promise<void> {
  const failures: string[] = [];
  try {
    await stopChild(child);
  } catch (error) {
    failures.push(`child shutdown: ${errorMessage(error)}`);
  } finally {
    const cleanup = await Promise.allSettled(
      [dataDir, homeDir].map((directory) =>
        fs.rm(directory, { recursive: true, force: true }),
      ),
    );
    cleanup.forEach((result, index) => {
      if (result.status === "rejected") {
        const directory = index === 0 ? dataDir : homeDir;
        failures.push(
          `remove ${directory}: ${errorMessage(result.reason as unknown)}`,
        );
      }
    });
  }

  if (failures.length > 0) {
    throw new Error(
      `failed to stop test Web server cleanly: ${failures.join("; ")}`,
    );
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
        ...(options.env ?? {}),
        HOST: host,
        PORT: String(port),
        ENABLE_HSTS: "false",
        RUSTFLAGS: process.env.RUSTFLAGS || "-Awarnings",
        // HOME is isolated below, so keep rustup/cargo pointed at the host
        // toolchain instead of making the shim search the empty test home.
        CARGO_HOME:
          options.env?.CARGO_HOME ??
          process.env.CARGO_HOME ??
          path.join(os.homedir(), ".cargo"),
        RUSTUP_HOME:
          options.env?.RUSTUP_HOME ??
          process.env.RUSTUP_HOME ??
          path.join(os.homedir(), ".rustup"),
        // Isolate every conventional home/config lookup used by Rust crates or
        // platform-specific code. CC_SWITCH_* remains the application-level
        // override; HOME/USERPROFILE and XDG prevent accidental fallback into
        // the developer/CI account when a subsystem calls dirs directly.
        HOME: homeDir,
        USERPROFILE: homeDir,
        XDG_CONFIG_HOME: path.join(homeDir, ".config"),
        XDG_DATA_HOME: path.join(homeDir, ".local", "share"),
        XDG_STATE_HOME: path.join(homeDir, ".local", "state"),
        XDG_CACHE_HOME: path.join(homeDir, ".cache"),
        CC_SWITCH_DATA_DIR: dataDir,
        CC_SWITCH_TEST_HOME: homeDir,
        CC_SWITCH_WEB_DIST_DIR: distWebDir,
        // The integration suites stand up mock upstreams (model-fetch endpoints,
        // endpoint-speed-test targets, etc.) on 127.0.0.1, which the production
        // SSRF guard (validate_outbound_url) blocks as an internal address. This
        // env var is the guard's existing operator allow-list bypass; setting it
        // here is TEST-INFRA ONLY so the mocked localhost upstreams are reachable
        // in tests. Production behavior is unchanged — the guard still blocks
        // loopback/private targets unless an operator opts in.
        CC_SWITCH_WEB_SSRF_ALLOW: "127.0.0.1,localhost,[::1]",
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

  try {
    await waitForServer(baseUrl, child);
  } catch (error) {
    try {
      await stopAndRemoveTestState(child, dataDir, homeDir);
    } catch (cleanupError) {
      throw new Error(
        `${errorMessage(error)}; cleanup also failed: ${errorMessage(cleanupError)}`,
      );
    }
    throw error;
  }

  let stopPromise: Promise<void> | undefined;
  return {
    baseUrl,
    dataDir,
    homeDir,
    stop: () => {
      stopPromise ??= stopAndRemoveTestState(child, dataDir, homeDir);
      return stopPromise;
    },
  };
}
