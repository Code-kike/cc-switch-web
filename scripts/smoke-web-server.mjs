#!/usr/bin/env node
import fs from "node:fs/promises";
import net from "node:net";
import os from "node:os";
import path from "node:path";
import process from "node:process";
import { spawn } from "node:child_process";

const repoRoot = process.cwd();
const distWebDir = path.join(repoRoot, "dist-web");
const startupTimeoutMs = Number(
  process.env.CC_SWITCH_SMOKE_STARTUP_TIMEOUT_MS || "300000",
);

const seededProviderIds = {
  opencode: "smoke-opencode",
  openclaw: "smoke-openclaw",
  hermes: "smoke-hermes",
};

const updatedProviderValues = {
  claudeBaseUrl: "https://claude-updated.example.com",
  opencodeBaseUrl: "https://opencode-updated.example.com/v1",
};

const smokeProviderIds = {
  claudeAlt: "smoke-claude-alt",
};

const smokeWorkspace = {
  filename: "AGENTS.md",
  initialContent: "# Smoke Workspace\n\nSeeded AGENTS content for web smoke.\n",
  updatedContent: "# Smoke Workspace\n\nUpdated AGENTS content for web smoke.\n",
};

const smokeDailyMemory = {
  filename: "2026-03-04.md",
  query: "nebula-marker",
  initialContent:
    "# 2026-03-04\n\nInitial daily memory entry with nebula-marker.\n",
  updatedContent:
    "# 2026-03-04\n\nUpdated daily memory entry with nebula-marker and revised content.\n",
};

const smokeBackup = {
  renamedName: "smoke-renamed-backup",
};

const smokeAdvancedConfig = {
  streamCheck: {
    timeoutSecs: 77,
    maxRetries: 4,
    degradedThresholdMs: 9000,
    claudeModel: "smoke-claude-model",
    codexModel: "smoke-codex-model",
    geminiModel: "smoke-gemini-model",
    testPrompt: "Return the smoke-marker.",
  },
  logConfig: {
    enabled: false,
    level: "debug",
  },
};

const smokeOpenClaw = {
  env: {
    vars: {
      OPENCLAW_TOKEN: "smoke-openclaw-env",
    },
    shellEnv: {
      PATH: "/opt/smoke/bin",
    },
  },
  envUpdated: {
    vars: {
      OPENCLAW_TOKEN: "smoke-openclaw-env-updated",
      EXTRA_FLAG: "enabled",
    },
    shellEnv: {
      PATH: "/opt/smoke/bin",
      HOME: "/srv/smoke",
    },
  },
  tools: {
    profile: "legacy-profile",
    allow: ["allow:read"],
    deny: ["deny:write"],
    passthrough: true,
  },
  toolsUpdated: {
    profile: "full",
    allow: ["allow:read", "allow:new"],
    deny: ["deny:new"],
    passthrough: false,
  },
  agents: {
    model: {
      primary: "smoke-openclaw/claude-sonnet-4",
      fallbacks: ["smoke-openclaw/claude-haiku-4"],
    },
    workspace: "~/projects",
    timeout: 300,
    contextTokens: 200000,
    maxConcurrent: 4,
    unknownFlag: true,
  },
  agentsUpdated: {
    model: {
      primary: "smoke-openclaw/claude-sonnet-4",
    },
    workspace: "~/projects/web",
    timeoutSeconds: 480,
    contextTokens: 250000,
    maxConcurrent: 6,
    unknownFlag: true,
  },
};

const smokeHermesMemory = {
  memoryInitial: "# Hermes MEMORY\n\nRemember the smoke-memory marker.\n",
  memoryUpdated: "# Hermes MEMORY\n\nUpdated memory content from web smoke.\n",
  userInitial: "# Hermes USER\n\nSeeded user profile for web smoke.\n",
};

const smokeDeepLink = {
  url: "ccswitch://v1/import?resource=provider&app=openclaw&name=Smoke%20DeepLink&endpoint=https%3A%2F%2Fdeeplink-smoke.example.com%2Fv1&apiKey=sk-deeplink",
  expectedEndpoint: "https://deeplink-smoke.example.com/v1",
  expectedHomepage: "https://deeplink-smoke.example.com",
  configBase64: Buffer.from(
    JSON.stringify({
      baseUrl: "https://deeplink-merged.example.com/v1",
      apiKey: "sk-merged",
    }),
    "utf8",
  ).toString("base64"),
};

const smokeSkill = {
  fileName: "smoke-skill.zip",
  installName: "smoke-skill",
  id: "local:smoke-skill",
  zipBase64:
    "UEsDBBQAAAAAADqMpFyqS+y0KAAAACgAAAAIAAAAU0tJTEwubWQjIFNtb2tlIFNraWxsCgpBIHNtb2tlLWluc3RhbGxlZCBza2lsbC4KUEsBAhQDFAAAAAAAOoykXKpL7LQoAAAAKAAAAAgAAAAAAAAAAAAAAIABAAAAAFNLSUxMLm1kUEsFBgAAAAABAAEANgAAAE4AAAAAAA==",
};

const smokeMcpIds = {
  shared: "smoke-shared",
  claudeOnly: "smoke-claude-only",
  geminiOnly: "smoke-gemini-only",
  opencodeOnly: "smoke-opencode-only",
  hermesOnly: "smoke-hermes-only",
  unified: "smoke-unified-server",
  legacyClaude: "smoke-legacy-claude",
  legacyConfig: "smoke-legacy-config",
};

const smokeSessions = [
  {
    sessionId: "019cc369-bd7c-7891-b371-7b20b4fe0b18",
    subdir: "project-alpha",
    fileName: "alpha-session.jsonl",
    projectDir: "/tmp/smoke-alpha",
    userMessage: "Alpha smoke request",
    assistantMessage: "Alpha smoke response",
    metaTimestamp: "2026-03-06T21:50:12Z",
    userTimestamp: "2026-03-06T21:50:13Z",
    assistantTimestamp: "2026-03-06T21:50:14Z",
  },
  {
    sessionId: "019cc36a-bd7c-7891-b371-7b20b4fe0b19",
    subdir: "project-beta",
    fileName: "beta-session.jsonl",
    projectDir: "/tmp/smoke-beta",
    userMessage: "Beta smoke request",
    assistantMessage: "Beta smoke response",
    metaTimestamp: "2026-03-06T21:51:12Z",
    userTimestamp: "2026-03-06T21:51:13Z",
    assistantTimestamp: "2026-03-06T21:51:14Z",
  },
  {
    sessionId: "019cc36b-bd7c-7891-b371-7b20b4fe0b20",
    subdir: "project-gamma",
    fileName: "gamma-session.jsonl",
    projectDir: "/tmp/smoke-gamma",
    userMessage: "Gamma smoke request",
    assistantMessage: "Gamma smoke response",
    metaTimestamp: "2026-03-06T21:52:12Z",
    userTimestamp: "2026-03-06T21:52:13Z",
    assistantTimestamp: "2026-03-06T21:52:14Z",
  },
];

const smokeUsage = {
  codexSessionId: "019cc36c-bd7c-7891-b371-7b20b4fe0b21",
  archivedFileName: "smoke-usage-session.jsonl",
  model: "openai/gpt-5.4",
  normalizedModel: "gpt-5.4",
  timestamp: "2026-03-07T12:15:02Z",
  inputTokens: 1200,
  cachedInputTokens: 300,
  outputTokens: 450,
  pricingModelId: "smoke-web-model-pricing",
};

async function ensureDistWeb() {
  const indexPath = path.join(distWebDir, "index.html");
  try {
    await fs.access(indexPath);
  } catch {
    throw new Error(
      `Missing ${indexPath}. Run "pnpm build:web" before smoke testing.`,
    );
  }
}

async function writeFixtureFile(filePath, content) {
  await fs.mkdir(path.dirname(filePath), { recursive: true });
  await fs.writeFile(filePath, content, "utf8");
}

async function writeJsonFixture(filePath, value) {
  await writeFixtureFile(filePath, JSON.stringify(value, null, 2));
}

async function readJsonFixture(filePath) {
  return JSON.parse(await fs.readFile(filePath, "utf8"));
}

function getSmokeSessionPath(homeDir, session) {
  return path.join(homeDir, ".codex", "sessions", session.subdir, session.fileName);
}

function getArchivedCodexUsagePath(homeDir) {
  return path.join(homeDir, ".codex", "archived_sessions", smokeUsage.archivedFileName);
}

function getOpenClawConfigPath(homeDir) {
  return path.join(homeDir, ".openclaw", "openclaw.json");
}

function getWorkspaceFilePath(homeDir, filename = smokeWorkspace.filename) {
  return path.join(homeDir, ".openclaw", "workspace", filename);
}

function getDailyMemoryPath(homeDir, filename = smokeDailyMemory.filename) {
  return path.join(homeDir, ".openclaw", "workspace", "memory", filename);
}

function getHermesConfigPath(homeDir) {
  return path.join(homeDir, ".hermes", "config.yaml");
}

function getHermesMemoryPath(homeDir, kind) {
  return path.join(
    homeDir,
    ".hermes",
    "memories",
    kind === "user" ? "USER.md" : "MEMORY.md",
  );
}

async function seedLiveProviderFixtures(homeDir) {
  await writeJsonFixture(path.join(homeDir, ".claude", "settings.json"), {
    env: {
      ANTHROPIC_AUTH_TOKEN: "claude-smoke-key",
      ANTHROPIC_BASE_URL: "https://claude-smoke.example.com",
    },
    ui: {
      displayName: "Smoke Claude",
    },
  });
  await writeJsonFixture(path.join(homeDir, ".claude.json"), {
    mcpServers: {
      [smokeMcpIds.shared]: {
        type: "stdio",
        command: "sh",
        args: ["-lc", "echo shared"],
      },
      [smokeMcpIds.claudeOnly]: {
        type: "stdio",
        command: "echo",
        args: ["claude-only"],
      },
    },
  });

  await writeJsonFixture(path.join(homeDir, ".codex", "auth.json"), {
    OPENAI_API_KEY: "codex-smoke-key",
  });
  await writeFixtureFile(
    path.join(homeDir, ".codex", "config.toml"),
    [
      'base_url = "https://codex-smoke.example.com/v1"',
      'model = "gpt-4.1"',
      "",
      `[mcp_servers.${smokeMcpIds.shared}]`,
      'type = "stdio"',
      'command = "sh"',
      'args = ["-lc", "echo shared"]',
      "",
    ].join("\n"),
  );

  await writeFixtureFile(
    path.join(homeDir, ".gemini", ".env"),
    "GEMINI_API_KEY=gemini-smoke-key\nGEMINI_MODEL=gemini-2.5-pro\n",
  );
  await writeJsonFixture(path.join(homeDir, ".gemini", "settings.json"), {
    mcpServers: {
      [smokeMcpIds.geminiOnly]: {
        url: "https://gemini-mcp.example.com/sse",
      },
    },
    selectedAuthType: "api_key",
  });

  await writeJsonFixture(path.join(homeDir, ".config", "opencode", "opencode.json"), {
    $schema: "https://opencode.ai/config.json",
    provider: {
      [seededProviderIds.opencode]: {
        npm: "@ai-sdk/openai-compatible",
        name: "Smoke OpenCode",
        options: {
          baseURL: "https://opencode-smoke.example.com/v1",
          apiKey: "opencode-smoke-key",
        },
        models: {
          "gpt-4o": {
            name: "GPT-4o",
          },
        },
      },
    },
    mcp: {
      [smokeMcpIds.opencodeOnly]: {
        type: "local",
        command: ["sh", "-lc", "echo opencode"],
        enabled: true,
      },
    },
  });

  await writeJsonFixture(getOpenClawConfigPath(homeDir), {
    models: {
      mode: "merge",
      providers: {
        [seededProviderIds.openclaw]: {
          baseUrl: "https://openclaw-smoke.example.com/v1",
          apiKey: "openclaw-smoke-key",
          api: "openai-completions",
          models: [
            {
              id: "claude-sonnet-4",
              name: "Claude Sonnet 4",
            },
          ],
        },
      },
    },
    env: smokeOpenClaw.env,
    tools: smokeOpenClaw.tools,
    agents: {
      defaults: smokeOpenClaw.agents,
    },
  });

  await writeFixtureFile(
    getHermesConfigPath(homeDir),
    [
      "custom_providers:",
      `  - name: ${seededProviderIds.hermes}`,
      "    base_url: https://hermes-smoke.example.com/v1",
      "    api_key: hermes-smoke-key",
      "    model: anthropic/claude-sonnet-4",
      "mcp_servers:",
      `  ${smokeMcpIds.hermesOnly}:`,
      "    command: sh",
      '    args: ["-lc", "echo hermes"]',
      "    enabled: true",
      "memory:",
      "  memory_char_limit: 2200",
      "  user_char_limit: 1375",
      "  memory_enabled: true",
      "  user_profile_enabled: false",
      "",
    ].join("\n"),
  );

  await writeFixtureFile(
    getWorkspaceFilePath(homeDir),
    smokeWorkspace.initialContent,
  );
  await writeFixtureFile(
    getDailyMemoryPath(homeDir),
    smokeDailyMemory.initialContent,
  );
  await writeFixtureFile(
    getHermesMemoryPath(homeDir, "memory"),
    smokeHermesMemory.memoryInitial,
  );
  await writeFixtureFile(
    getHermesMemoryPath(homeDir, "user"),
    smokeHermesMemory.userInitial,
  );
}

async function seedSessionFixtures(homeDir) {
  for (const session of smokeSessions) {
    const lines = [
      JSON.stringify({
        timestamp: session.metaTimestamp,
        type: "session_meta",
        payload: {
          id: session.sessionId,
          cwd: session.projectDir,
        },
      }),
      JSON.stringify({
        timestamp: session.userTimestamp,
        type: "response_item",
        payload: {
          type: "message",
          role: "user",
          content: session.userMessage,
        },
      }),
      JSON.stringify({
        timestamp: session.assistantTimestamp,
        type: "response_item",
        payload: {
          type: "message",
          role: "assistant",
          content: session.assistantMessage,
        },
      }),
    ];
    await writeFixtureFile(
      getSmokeSessionPath(homeDir, session),
      `${lines.join("\n")}\n`,
    );
  }

  const usageLines = [
    JSON.stringify({
      timestamp: "2026-03-07T12:15:00Z",
      type: "session_meta",
      payload: {
        session_id: smokeUsage.codexSessionId,
      },
    }),
    JSON.stringify({
      timestamp: "2026-03-07T12:15:01Z",
      type: "turn_context",
      payload: {
        model: smokeUsage.model,
      },
    }),
    JSON.stringify({
      timestamp: smokeUsage.timestamp,
      type: "event_msg",
      payload: {
        type: "token_count",
        info: {
          model: smokeUsage.model,
          total_token_usage: {
            input_tokens: smokeUsage.inputTokens,
            cached_input_tokens: smokeUsage.cachedInputTokens,
            output_tokens: smokeUsage.outputTokens,
          },
        },
      },
    }),
  ];
  await writeFixtureFile(
    getArchivedCodexUsagePath(homeDir),
    `${usageLines.join("\n")}\n`,
  );
}

function getFreePort() {
  return new Promise((resolve, reject) => {
    const server = net.createServer();
    server.on("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      if (!address || typeof address === "string") {
        server.close(() => reject(new Error("Failed to allocate port")));
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

async function fetchJson(baseUrl, probe) {
  if (typeof probe.send === "function") {
    return probe.send(baseUrl, smokeArtifacts);
  }

  const response = await fetch(new URL(probe.path, baseUrl), {
    method: probe.method,
    headers: probe.body
      ? {
          "content-type": "application/json",
        }
      : undefined,
    body: probe.body ? JSON.stringify(probe.body) : undefined,
  });

  const contentType = response.headers.get("content-type") ?? "";
  const payload = contentType.includes("application/json")
    ? await response.json()
    : await response.text();

  return { response, payload };
}

const smokeArtifacts = {
  createdBackupFilename: null,
  renamedBackupFilename: null,
  restoreSafetyBackupId: null,
  exportedSql: null,
  homeDir: null,
  importedPromptId: null,
  importedDeeplinkProviderId: null,
  claudeFailoverProviderId: null,
  syncedUsageRequestId: null,
};

async function waitForServer(baseUrl, child, timeoutMs = startupTimeoutMs) {
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
    } catch {}
    await new Promise((resolve) => setTimeout(resolve, 1000));
  }
  throw new Error(
    `Timed out waiting for ${baseUrl}/api/health after ${timeoutMs}ms`,
  );
}

async function shutdown(child) {
  if (!child || child.killed) return;
  child.kill("SIGINT");
  await new Promise((resolve) => {
    const timer = setTimeout(() => {
      child.kill("SIGKILL");
      resolve();
    }, 5000);
    child.once("exit", () => {
      clearTimeout(timer);
      resolve();
    });
  });
}

const probes = [
  {
    name: "health",
    method: "GET",
    path: "/api/health",
    validate(response, payload) {
      if (!response.ok || payload?.status !== "ok") {
        throw new Error(`expected status ok, got ${response.status}`);
      }
    },
  },
  {
    name: "spa-root",
    method: "GET",
    path: "/",
    validate(response, payload) {
      const contentType = response.headers.get("content-type") ?? "";
      if (!response.ok || !contentType.includes("text/html")) {
        throw new Error(`expected HTML 200, got ${response.status} ${contentType}`);
      }
      if (typeof payload !== "string" || !payload.toLowerCase().includes("<!doctype html")) {
        throw new Error("expected SPA index.html payload");
      }
    },
  },
  {
    name: "settings",
    method: "GET",
    path: "/api/settings/get-settings",
    validate(response, payload) {
      if (!response.ok || typeof payload !== "object" || payload === null) {
        throw new Error(`expected settings JSON, got ${response.status}`);
      }
    },
  },
  {
    name: "stream-check-config",
    method: "GET",
    path: "/api/config/get-stream-check-config",
    validate(response, payload) {
      if (
        !response.ok ||
        typeof payload !== "object" ||
        payload === null ||
        typeof payload?.timeoutSecs !== "number" ||
        typeof payload?.maxRetries !== "number" ||
        typeof payload?.degradedThresholdMs !== "number" ||
        typeof payload?.claudeModel !== "string" ||
        typeof payload?.codexModel !== "string" ||
        typeof payload?.geminiModel !== "string"
      ) {
        throw new Error(`expected stream-check config object, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "stream-check-config-save",
    method: "PUT",
    path: "/api/config/save-stream-check-config",
    body: {
      config: smokeAdvancedConfig.streamCheck,
    },
    validate(response, payload) {
      if (!response.ok || payload !== null) {
        throw new Error(`expected stream-check save null payload, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "stream-check-config-after-save",
    method: "GET",
    path: "/api/config/get-stream-check-config",
    validate(response, payload) {
      if (
        !response.ok ||
        payload?.timeoutSecs !== smokeAdvancedConfig.streamCheck.timeoutSecs ||
        payload?.maxRetries !== smokeAdvancedConfig.streamCheck.maxRetries ||
        payload?.degradedThresholdMs !== smokeAdvancedConfig.streamCheck.degradedThresholdMs ||
        payload?.claudeModel !== smokeAdvancedConfig.streamCheck.claudeModel ||
        payload?.codexModel !== smokeAdvancedConfig.streamCheck.codexModel ||
        payload?.geminiModel !== smokeAdvancedConfig.streamCheck.geminiModel ||
        payload?.testPrompt !== smokeAdvancedConfig.streamCheck.testPrompt
      ) {
        throw new Error(`expected persisted stream-check config, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "log-config",
    method: "GET",
    path: "/api/config/get-log-config",
    validate(response, payload) {
      if (
        !response.ok ||
        typeof payload !== "object" ||
        payload === null ||
        typeof payload?.enabled !== "boolean" ||
        typeof payload?.level !== "string"
      ) {
        throw new Error(`expected log-config object, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "log-config-set",
    method: "PUT",
    path: "/api/config/set-log-config",
    body: {
      config: smokeAdvancedConfig.logConfig,
    },
    validate(response, payload) {
      if (!response.ok || payload !== true) {
        throw new Error(`expected log-config set true payload, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "log-config-after-save",
    method: "GET",
    path: "/api/config/get-log-config",
    validate(response, payload) {
      if (
        !response.ok ||
        payload?.enabled !== smokeAdvancedConfig.logConfig.enabled ||
        payload?.level !== smokeAdvancedConfig.logConfig.level
      ) {
        throw new Error(`expected persisted log-config, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "proxy-status",
    method: "GET",
    path: "/api/proxy/get-proxy-status",
    validate(response, payload) {
      if (!response.ok || typeof payload?.running !== "boolean") {
        throw new Error(`expected proxy status JSON, got ${response.status}`);
      }
    },
  },
  {
    name: "backups",
    method: "GET",
    path: "/api/backups/list-db-backups",
    validate(response, payload) {
      if (!response.ok || !Array.isArray(payload)) {
        throw new Error(`expected backup array, got ${response.status}`);
      }
    },
  },
  {
    name: "backup-create",
    method: "POST",
    path: "/api/backups/create-db-backup",
    validate(response, payload, artifacts) {
      if (!response.ok || typeof payload !== "string" || !payload.endsWith(".db")) {
        throw new Error(`expected created backup filename, got ${response.status} ${JSON.stringify(payload)}`);
      }
      artifacts.createdBackupFilename = payload;
    },
  },
  {
    name: "backups-after-create",
    method: "GET",
    path: "/api/backups/list-db-backups",
    validate(response, payload, artifacts) {
      if (
        !response.ok ||
        !Array.isArray(payload) ||
        !payload.some((entry) => entry?.filename === artifacts.createdBackupFilename)
      ) {
        throw new Error(`expected created backup to appear in list, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "backup-rename",
    method: "POST",
    path: "/api/backups/rename-db-backup",
    async send(baseUrl, artifacts) {
      if (!artifacts.createdBackupFilename) {
        throw new Error("missing created backup filename artifact");
      }
      const response = await fetch(new URL("/api/backups/rename-db-backup", baseUrl), {
        method: "POST",
        headers: {
          "content-type": "application/json",
        },
        body: JSON.stringify({
          oldFilename: artifacts.createdBackupFilename,
          newName: smokeBackup.renamedName,
        }),
      });
      const payload = await response.json();
      return { response, payload };
    },
    validate(response, payload, artifacts) {
      const expectedFilename = `${smokeBackup.renamedName}.db`;
      if (!response.ok || payload !== expectedFilename) {
        throw new Error(`expected renamed backup filename ${expectedFilename}, got ${response.status} ${JSON.stringify(payload)}`);
      }
      artifacts.renamedBackupFilename = payload;
    },
  },
  {
    name: "backups-after-rename",
    method: "GET",
    path: "/api/backups/list-db-backups",
    validate(response, payload, artifacts) {
      if (
        !response.ok ||
        !Array.isArray(payload) ||
        !payload.some((entry) => entry?.filename === artifacts.renamedBackupFilename) ||
        payload.some((entry) => entry?.filename === artifacts.createdBackupFilename)
      ) {
        throw new Error(`expected renamed backup to replace original list entry, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "backup-restore",
    method: "POST",
    path: "/api/backups/restore-db-backup",
    async send(baseUrl, artifacts) {
      if (!artifacts.renamedBackupFilename) {
        throw new Error("missing renamed backup filename artifact");
      }
      const response = await fetch(new URL("/api/backups/restore-db-backup", baseUrl), {
        method: "POST",
        headers: {
          "content-type": "application/json",
        },
        body: JSON.stringify({
          filename: artifacts.renamedBackupFilename,
        }),
      });
      const payload = await response.json();
      return { response, payload };
    },
    validate(response, payload, artifacts) {
      if (!response.ok || typeof payload !== "string" || payload.length === 0) {
        throw new Error(`expected restore safety backup id, got ${response.status} ${JSON.stringify(payload)}`);
      }
      artifacts.restoreSafetyBackupId = payload;
    },
  },
  {
    name: "backups-after-restore",
    method: "GET",
    path: "/api/backups/list-db-backups",
    validate(response, payload, artifacts) {
      const safetyFilename = `${artifacts.restoreSafetyBackupId}.db`;
      if (
        !response.ok ||
        !Array.isArray(payload) ||
        !payload.some((entry) => entry?.filename === artifacts.renamedBackupFilename) ||
        !payload.some((entry) => entry?.filename === safetyFilename)
      ) {
        throw new Error(`expected restore to keep target backup and create safety backup, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "backup-delete",
    method: "DELETE",
    path: "/api/backups/delete-db-backup",
    async send(baseUrl, artifacts) {
      if (!artifacts.renamedBackupFilename) {
        throw new Error("missing renamed backup filename artifact");
      }
      const response = await fetch(
        new URL(
          `/api/backups/delete-db-backup?filename=${encodeURIComponent(artifacts.renamedBackupFilename)}`,
          baseUrl,
        ),
        {
          method: "DELETE",
        },
      );
      const payload = await response.json();
      return { response, payload };
    },
    validate(response, payload) {
      if (!response.ok || payload !== null) {
        throw new Error(`expected backup delete null payload, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "backups-after-delete",
    method: "GET",
    path: "/api/backups/list-db-backups",
    validate(response, payload, artifacts) {
      const safetyFilename = `${artifacts.restoreSafetyBackupId}.db`;
      if (
        !response.ok ||
        !Array.isArray(payload) ||
        payload.some((entry) => entry?.filename === artifacts.renamedBackupFilename) ||
        !payload.some((entry) => entry?.filename === safetyFilename)
      ) {
        throw new Error(`expected deleted backup to disappear while safety backup remains, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "mcp-servers",
    method: "GET",
    path: "/api/mcp/get-mcp-servers",
    validate(response, payload) {
      if (!response.ok || typeof payload !== "object" || payload === null) {
        throw new Error(`expected MCP object, got ${response.status}`);
      }
    },
  },
  {
    name: "claude-mcp-status",
    method: "GET",
    path: "/api/mcp/get-claude-mcp-status",
    validate(response, payload) {
      if (!response.ok || typeof payload !== "object" || payload === null) {
        throw new Error(`expected Claude MCP status object, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "validate-mcp-command",
    method: "POST",
    path: "/api/mcp/validate-mcp-command",
    body: { cmd: "sh" },
    validate(response, payload) {
      if (!response.ok || payload !== true) {
        throw new Error(`expected MCP command validation to succeed, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "import-mcp-from-apps",
    method: "POST",
    path: "/api/mcp/import-mcp-from-apps",
    body: {},
    validate(response, payload) {
      if (!response.ok || typeof payload !== "number" || payload < 5) {
        throw new Error(`expected MCP import count >= 5, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "mcp-servers-after-import",
    method: "GET",
    path: "/api/mcp/get-mcp-servers",
    validate(response, payload) {
      if (!response.ok || typeof payload !== "object" || payload === null) {
        throw new Error(`expected imported MCP object, got ${response.status} ${JSON.stringify(payload)}`);
      }
      if (
        Object.keys(payload).length !== 5 ||
        !payload[smokeMcpIds.shared]?.apps?.claude ||
        !payload[smokeMcpIds.shared]?.apps?.codex ||
        !payload[smokeMcpIds.claudeOnly]?.apps?.claude ||
        !payload[smokeMcpIds.geminiOnly]?.apps?.gemini ||
        !payload[smokeMcpIds.opencodeOnly]?.apps?.opencode ||
        !payload[smokeMcpIds.hermesOnly]?.apps?.hermes ||
        payload[smokeMcpIds.opencodeOnly]?.server?.type !== "stdio" ||
        payload[smokeMcpIds.geminiOnly]?.server?.type !== "sse"
      ) {
        throw new Error(`expected imported MCP servers to merge and normalize correctly, got ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "read-claude-mcp-config",
    method: "GET",
    path: "/api/config/read-claude-mcp-config",
    validate(response, payload) {
      if (
        !response.ok ||
        typeof payload !== "string" ||
        !payload.includes(smokeMcpIds.shared) ||
        !payload.includes(smokeMcpIds.claudeOnly)
      ) {
        throw new Error(`expected Claude MCP config text, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "get-mcp-config-claude",
    method: "GET",
    path: "/api/config/get-mcp-config?app=claude",
    validate(response, payload) {
      if (
        !response.ok ||
        typeof payload !== "object" ||
        payload === null ||
        typeof payload?.configPath !== "string" ||
        typeof payload?.servers !== "object" ||
        payload.servers === null ||
        !payload.servers[smokeMcpIds.shared] ||
        !payload.servers[smokeMcpIds.claudeOnly] ||
        payload.servers[smokeMcpIds.geminiOnly]
      ) {
        throw new Error(`expected Claude legacy MCP projection, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "toggle-mcp-app-disable-codex-live",
    method: "POST",
    path: "/api/mcp/toggle-mcp-app",
    async send(baseUrl, artifacts) {
      const response = await fetch(new URL("/api/mcp/toggle-mcp-app", baseUrl), {
        method: "POST",
        headers: {
          "content-type": "application/json",
        },
        body: JSON.stringify({
          serverId: smokeMcpIds.shared,
          app: "codex",
          enabled: false,
        }),
      });
      const result = await response.json();
      const codexConfig = await fs.readFile(
        path.join(artifacts.homeDir, ".codex", "config.toml"),
        "utf8",
      );
      const serversResponse = await fetch(new URL("/api/mcp/get-mcp-servers", baseUrl));
      const servers = await serversResponse.json();
      return { response, payload: { result, codexConfig, servers } };
    },
    validate(response, payload) {
      if (
        !response.ok ||
        payload?.result !== null ||
        payload?.codexConfig?.includes(`mcp_servers.${smokeMcpIds.shared}`) ||
        payload?.servers?.[smokeMcpIds.shared]?.apps?.codex !== false
      ) {
        throw new Error(`expected disabling Codex MCP to remove live config, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "set-mcp-enabled-restore-codex-live",
    method: "PUT",
    path: "/api/mcp/set-mcp-enabled",
    async send(baseUrl, artifacts) {
      const response = await fetch(new URL("/api/mcp/set-mcp-enabled", baseUrl), {
        method: "PUT",
        headers: {
          "content-type": "application/json",
        },
        body: JSON.stringify({
          app: "codex",
          id: smokeMcpIds.shared,
          enabled: true,
        }),
      });
      const result = await response.json();
      const codexConfig = await fs.readFile(
        path.join(artifacts.homeDir, ".codex", "config.toml"),
        "utf8",
      );
      const serversResponse = await fetch(new URL("/api/mcp/get-mcp-servers", baseUrl));
      const servers = await serversResponse.json();
      return { response, payload: { result, codexConfig, servers } };
    },
    validate(response, payload) {
      if (
        !response.ok ||
        payload?.result !== true ||
        !payload?.codexConfig?.includes(`mcp_servers.${smokeMcpIds.shared}`) ||
        payload?.servers?.[smokeMcpIds.shared]?.apps?.codex !== true
      ) {
        throw new Error(`expected legacy set_mcp_enabled to restore Codex live config, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "upsert-mcp-server-writes-live",
    method: "POST",
    path: "/api/mcp/upsert-mcp-server",
    async send(baseUrl, artifacts) {
      const response = await fetch(new URL("/api/mcp/upsert-mcp-server", baseUrl), {
        method: "POST",
        headers: {
          "content-type": "application/json",
        },
        body: JSON.stringify({
          server: {
            id: smokeMcpIds.unified,
            name: "Smoke Unified Server",
            description: "Unified smoke MCP server",
            server: {
              type: "stdio",
              command: "sh",
              args: ["-lc", "echo unified"],
            },
            apps: {
              claude: true,
              codex: false,
              gemini: true,
              opencode: false,
              openclaw: false,
              hermes: false,
            },
            tags: ["smoke"],
          },
        }),
      });
      const result = await response.json();
      const claudeConfig = await readJsonFixture(
        path.join(artifacts.homeDir, ".claude.json"),
      );
      const geminiConfig = await readJsonFixture(
        path.join(artifacts.homeDir, ".gemini", "settings.json"),
      );
      const serversResponse = await fetch(new URL("/api/mcp/get-mcp-servers", baseUrl));
      const servers = await serversResponse.json();
      return { response, payload: { result, claudeConfig, geminiConfig, servers } };
    },
    validate(response, payload) {
      if (
        !response.ok ||
        payload?.result !== null ||
        !payload?.claudeConfig?.mcpServers?.[smokeMcpIds.unified] ||
        !payload?.geminiConfig?.mcpServers?.[smokeMcpIds.unified] ||
        !payload?.servers?.[smokeMcpIds.unified]?.apps?.claude ||
        !payload?.servers?.[smokeMcpIds.unified]?.apps?.gemini
      ) {
        throw new Error(`expected unified MCP upsert to sync live config, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "delete-mcp-server-removes-live",
    method: "DELETE",
    path: `/api/mcp/delete-mcp-server?id=${encodeURIComponent(smokeMcpIds.unified)}`,
    async send(baseUrl, artifacts) {
      const response = await fetch(
        new URL(`/api/mcp/delete-mcp-server?id=${encodeURIComponent(smokeMcpIds.unified)}`, baseUrl),
        {
          method: "DELETE",
        },
      );
      const result = await response.json();
      const claudeConfig = await readJsonFixture(
        path.join(artifacts.homeDir, ".claude.json"),
      );
      const geminiConfig = await readJsonFixture(
        path.join(artifacts.homeDir, ".gemini", "settings.json"),
      );
      const serversResponse = await fetch(new URL("/api/mcp/get-mcp-servers", baseUrl));
      const servers = await serversResponse.json();
      return { response, payload: { result, claudeConfig, geminiConfig, servers } };
    },
    validate(response, payload) {
      if (
        !response.ok ||
        payload?.result !== true ||
        payload?.claudeConfig?.mcpServers?.[smokeMcpIds.unified] ||
        payload?.geminiConfig?.mcpServers?.[smokeMcpIds.unified] ||
        payload?.servers?.[smokeMcpIds.unified]
      ) {
        throw new Error(`expected unified MCP delete to remove live config, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "upsert-claude-mcp-server-live",
    method: "POST",
    path: "/api/mcp/upsert-claude-mcp-server",
    async send(baseUrl, artifacts) {
      const response = await fetch(new URL("/api/mcp/upsert-claude-mcp-server", baseUrl), {
        method: "POST",
        headers: {
          "content-type": "application/json",
        },
        body: JSON.stringify({
          id: smokeMcpIds.legacyClaude,
          spec: {
            type: "stdio",
            command: "echo",
            args: ["legacy-claude"],
          },
        }),
      });
      const result = await response.json();
      const claudeConfig = await readJsonFixture(
        path.join(artifacts.homeDir, ".claude.json"),
      );
      return { response, payload: { result, claudeConfig } };
    },
    validate(response, payload) {
      if (
        !response.ok ||
        payload?.result !== true ||
        !payload?.claudeConfig?.mcpServers?.[smokeMcpIds.legacyClaude]
      ) {
        throw new Error(`expected legacy Claude MCP upsert to update live config, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "delete-claude-mcp-server-live",
    method: "DELETE",
    path: `/api/mcp/delete-claude-mcp-server?id=${encodeURIComponent(smokeMcpIds.legacyClaude)}`,
    async send(baseUrl, artifacts) {
      const response = await fetch(
        new URL(
          `/api/mcp/delete-claude-mcp-server?id=${encodeURIComponent(smokeMcpIds.legacyClaude)}`,
          baseUrl,
        ),
        {
          method: "DELETE",
        },
      );
      const result = await response.json();
      const claudeConfig = await readJsonFixture(
        path.join(artifacts.homeDir, ".claude.json"),
      );
      return { response, payload: { result, claudeConfig } };
    },
    validate(response, payload) {
      if (
        !response.ok ||
        payload?.result !== true ||
        payload?.claudeConfig?.mcpServers?.[smokeMcpIds.legacyClaude]
      ) {
        throw new Error(`expected legacy Claude MCP delete to update live config, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "upsert-mcp-server-in-config-live",
    method: "POST",
    path: "/api/config/upsert-mcp-server-in-config",
    async send(baseUrl, artifacts) {
      const response = await fetch(
        new URL("/api/config/upsert-mcp-server-in-config", baseUrl),
        {
          method: "POST",
          headers: {
            "content-type": "application/json",
          },
          body: JSON.stringify({
            app: "hermes",
            id: smokeMcpIds.legacyConfig,
            spec: {
              type: "stdio",
              command: "echo",
              args: ["legacy-config"],
            },
          }),
        },
      );
      const result = await response.json();
      const hermesConfig = await fs.readFile(
        path.join(artifacts.homeDir, ".hermes", "config.yaml"),
        "utf8",
      );
      const serversResponse = await fetch(new URL("/api/mcp/get-mcp-servers", baseUrl));
      const servers = await serversResponse.json();
      return { response, payload: { result, hermesConfig, servers } };
    },
    validate(response, payload) {
      if (
        !response.ok ||
        payload?.result !== true ||
        !payload?.hermesConfig?.includes(`${smokeMcpIds.legacyConfig}:`) ||
        !payload?.servers?.[smokeMcpIds.legacyConfig]?.apps?.hermes
      ) {
        throw new Error(`expected legacy config MCP upsert to update Hermes live config, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "delete-mcp-server-in-config-live",
    method: "DELETE",
    path: `/api/config/delete-mcp-server-in-config?app=hermes&id=${encodeURIComponent(smokeMcpIds.legacyConfig)}`,
    async send(baseUrl, artifacts) {
      const response = await fetch(
        new URL(
          `/api/config/delete-mcp-server-in-config?app=hermes&id=${encodeURIComponent(smokeMcpIds.legacyConfig)}`,
          baseUrl,
        ),
        {
          method: "DELETE",
        },
      );
      const result = await response.json();
      const hermesConfig = await fs.readFile(
        path.join(artifacts.homeDir, ".hermes", "config.yaml"),
        "utf8",
      );
      const serversResponse = await fetch(new URL("/api/mcp/get-mcp-servers", baseUrl));
      const servers = await serversResponse.json();
      return { response, payload: { result, hermesConfig, servers } };
    },
    validate(response, payload) {
      if (
        !response.ok ||
        payload?.result !== true ||
        payload?.hermesConfig?.includes(`${smokeMcpIds.legacyConfig}:`) ||
        payload?.servers?.[smokeMcpIds.legacyConfig]
      ) {
        throw new Error(`expected legacy config MCP delete to update Hermes live config, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "sessions-list",
    method: "GET",
    path: "/api/sessions/list-sessions",
    validate(response, payload) {
      const sessionIds = Array.isArray(payload)
        ? payload.map((item) => item?.sessionId)
        : [];
      if (
        !response.ok ||
        !Array.isArray(payload) ||
        payload.length !== smokeSessions.length ||
        !smokeSessions.every((session) => sessionIds.includes(session.sessionId)) ||
        !payload.every(
          (session) =>
            session?.providerId === "codex" &&
            typeof session?.resumeCommand === "string" &&
            session.resumeCommand.includes(session.sessionId),
        )
      ) {
        throw new Error(`expected seeded session list, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "session-messages",
    method: "GET",
    path: "/api/sessions/get-session-messages",
    async send(baseUrl, artifacts) {
      const session = smokeSessions[0];
      const query = new URLSearchParams({
        providerId: "codex",
        sourcePath: getSmokeSessionPath(artifacts.homeDir, session),
      });
      const response = await fetch(
        new URL(`/api/sessions/get-session-messages?${query.toString()}`, baseUrl),
      );
      const payload = await response.json();
      return { response, payload };
    },
    validate(response, payload) {
      if (
        !response.ok ||
        !Array.isArray(payload) ||
        payload.length !== 2 ||
        payload[0]?.role !== "user" ||
        payload[0]?.content !== smokeSessions[0].userMessage ||
        payload[1]?.role !== "assistant" ||
        payload[1]?.content !== smokeSessions[0].assistantMessage
      ) {
        throw new Error(`expected session messages payload, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "delete-session",
    method: "DELETE",
    path: "/api/sessions/delete-session",
    async send(baseUrl, artifacts) {
      const session = smokeSessions[0];
      const sourcePath = getSmokeSessionPath(artifacts.homeDir, session);
      const query = new URLSearchParams({
        providerId: "codex",
        sessionId: session.sessionId,
        sourcePath,
      });
      const response = await fetch(
        new URL(`/api/sessions/delete-session?${query.toString()}`, baseUrl),
        {
          method: "DELETE",
        },
      );
      const deleted = await response.json();
      const listResponse = await fetch(new URL("/api/sessions/list-sessions", baseUrl));
      const remainingSessions = await listResponse.json();
      let fileExists = true;
      try {
        await fs.access(sourcePath);
      } catch {
        fileExists = false;
      }
      return { response, payload: { deleted, remainingSessions, fileExists } };
    },
    validate(response, payload) {
      if (
        !response.ok ||
        payload?.deleted !== true ||
        payload?.fileExists !== false ||
        !Array.isArray(payload?.remainingSessions) ||
        payload.remainingSessions.length !== smokeSessions.length - 1 ||
        payload.remainingSessions.some(
          (session) => session?.sessionId === smokeSessions[0].sessionId,
        )
      ) {
        throw new Error(`expected single session delete to succeed, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "delete-sessions-batch",
    method: "DELETE",
    path: "/api/sessions/delete-sessions",
    async send(baseUrl, artifacts) {
      const items = smokeSessions.slice(1).map((session) => ({
        providerId: "codex",
        sessionId: session.sessionId,
        sourcePath: getSmokeSessionPath(artifacts.homeDir, session),
      }));
      const response = await fetch(new URL("/api/sessions/delete-sessions", baseUrl), {
        method: "DELETE",
        headers: {
          "content-type": "application/json",
        },
        body: JSON.stringify({ items }),
      });
      const outcomes = await response.json();
      const listResponse = await fetch(new URL("/api/sessions/list-sessions", baseUrl));
      const remainingSessions = await listResponse.json();
      const remainingFiles = await Promise.all(
        smokeSessions.slice(1).map(async (session) => {
          try {
            await fs.access(getSmokeSessionPath(artifacts.homeDir, session));
            return true;
          } catch {
            return false;
          }
        }),
      );
      return { response, payload: { outcomes, remainingSessions, remainingFiles } };
    },
    validate(response, payload) {
      if (
        !response.ok ||
        !Array.isArray(payload?.outcomes) ||
        payload.outcomes.length !== 2 ||
        !payload.outcomes.every((item) => item?.success === true) ||
        !Array.isArray(payload?.remainingSessions) ||
        payload.remainingSessions.length !== 0 ||
        !Array.isArray(payload?.remainingFiles) ||
        payload.remainingFiles.some(Boolean)
      ) {
        throw new Error(`expected batch session delete to succeed, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "session-launch-terminal-desktop-only",
    method: "POST",
    path: "/api/sessions/launch-session-terminal",
    body: { command: `codex resume ${smokeSessions[0].sessionId}` },
    validate(response, payload) {
      if (response.status !== 501 || payload?.code !== "WEB_DESKTOP_ONLY") {
        throw new Error(`expected session terminal route to be desktop-only, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "sync-session-usage",
    method: "POST",
    path: "/api/sessions/sync-session-usage",
    validate(response, payload, artifacts) {
      if (
        !response.ok ||
        payload?.imported !== 1 ||
        payload?.skipped !== 0 ||
        typeof payload?.filesScanned !== "number" ||
        payload.filesScanned < 1 ||
        !Array.isArray(payload?.errors) ||
        payload.errors.length !== 0
      ) {
        throw new Error(`expected session usage sync to import one archived Codex session, got ${response.status} ${JSON.stringify(payload)}`);
      }
      artifacts.syncedUsageRequestId = `codex_session:${smokeUsage.codexSessionId}:1`;
    },
  },
  {
    name: "usage-data-sources-after-session-sync",
    method: "GET",
    path: "/api/usage/get-usage-data-sources",
    validate(response, payload) {
      const codexSessionSource = Array.isArray(payload)
        ? payload.find((item) => item?.dataSource === "codex_session")
        : null;
      if (
        !response.ok ||
        !Array.isArray(payload) ||
        !codexSessionSource ||
        codexSessionSource.requestCount !== 1 ||
        Number(codexSessionSource.totalCostUsd) <= 0
      ) {
        throw new Error(`expected codex session data source after sync, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "usage-summary-after-session-sync",
    method: "GET",
    path: "/api/usage/get-usage-summary?appType=codex",
    validate(response, payload) {
      if (
        !response.ok ||
        payload?.totalRequests !== 1 ||
        payload?.totalInputTokens !== smokeUsage.inputTokens ||
        payload?.totalOutputTokens !== smokeUsage.outputTokens ||
        payload?.totalCacheReadTokens !== smokeUsage.cachedInputTokens ||
        payload?.totalCacheCreationTokens !== 0 ||
        Math.abs(Number(payload?.successRate) - 100) > 0.001 ||
        Number(payload?.totalCost) <= 0
      ) {
        throw new Error(`expected codex usage summary after sync, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "usage-trends-after-session-sync",
    method: "GET",
    path: "/api/usage/get-usage-trends",
    async send(baseUrl) {
      const centerTs = Math.floor(Date.parse(smokeUsage.timestamp) / 1000);
      const query = new URLSearchParams({
        appType: "codex",
        startDate: String(centerTs - 3600),
        endDate: String(centerTs + 3600),
      });
      const response = await fetch(
        new URL(`/api/usage/get-usage-trends?${query.toString()}`, baseUrl),
      );
      const payload = await response.json();
      return { response, payload };
    },
    validate(response, payload) {
      if (
        !response.ok ||
        !Array.isArray(payload) ||
        !payload.some(
          (bucket) =>
            bucket?.requestCount === 1 &&
            bucket?.totalInputTokens === smokeUsage.inputTokens &&
            bucket?.totalOutputTokens === smokeUsage.outputTokens &&
            bucket?.totalCacheReadTokens === smokeUsage.cachedInputTokens &&
            Number(bucket?.totalCost) > 0,
        )
      ) {
        throw new Error(`expected daily usage trends after sync, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "request-logs-after-session-sync",
    method: "POST",
    path: "/api/system/get_request_logs",
    body: {
      filters: {
        appType: "codex",
      },
      page: 0,
      pageSize: 10,
    },
    validate(response, payload, artifacts) {
      const [firstLog] = Array.isArray(payload?.data) ? payload.data : [];
      if (
        !response.ok ||
        payload?.total !== 1 ||
        payload?.page !== 0 ||
        payload?.pageSize !== 10 ||
        !firstLog ||
        firstLog.requestId !== `codex_session:${smokeUsage.codexSessionId}:1` ||
        firstLog.providerId !== "_codex_session" ||
        firstLog.providerName !== "Codex (Session)" ||
        firstLog.appType !== "codex" ||
        firstLog.model !== smokeUsage.normalizedModel ||
        firstLog.dataSource !== "codex_session" ||
        Number(firstLog.totalCostUsd) <= 0
      ) {
        throw new Error(`expected request logs populated from synced session usage, got ${response.status} ${JSON.stringify(payload)}`);
      }
      artifacts.syncedUsageRequestId = firstLog.requestId;
    },
  },
  {
    name: "request-detail-after-session-sync",
    method: "POST",
    path: "/api/system/get_request_detail",
    async send(baseUrl, artifacts) {
      const requestId = artifacts.syncedUsageRequestId;
      const response = await fetch(new URL("/api/system/get_request_detail", baseUrl), {
        method: "POST",
        headers: {
          "content-type": "application/json",
        },
        body: JSON.stringify({ requestId }),
      });
      const payload = await response.json();
      return { response, payload };
    },
    validate(response, payload) {
      if (
        !response.ok ||
        payload?.requestId !== `codex_session:${smokeUsage.codexSessionId}:1` ||
        payload?.providerId !== "_codex_session" ||
        payload?.providerName !== "Codex (Session)" ||
        payload?.model !== smokeUsage.normalizedModel ||
        payload?.inputTokens !== smokeUsage.inputTokens ||
        payload?.outputTokens !== smokeUsage.outputTokens ||
        payload?.cacheReadTokens !== smokeUsage.cachedInputTokens ||
        payload?.dataSource !== "codex_session" ||
        Number(payload?.totalCostUsd) <= 0
      ) {
        throw new Error(`expected request detail for synced session usage, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "request-detail-not-found",
    method: "POST",
    path: "/api/system/get_request_detail",
    body: { requestId: "smoke-missing-request" },
    validate(response, payload) {
      if (!response.ok || payload !== null) {
        throw new Error(`expected null request detail for missing request id, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "model-pricing-list",
    method: "POST",
    path: "/api/system/get_model_pricing",
    validate(response, payload) {
      if (
        !response.ok ||
        !Array.isArray(payload) ||
        !payload.some((item) => item?.modelId === smokeUsage.normalizedModel)
      ) {
        throw new Error(`expected seeded model pricing list, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "model-pricing-upsert",
    method: "POST",
    path: "/api/system/update_model_pricing",
    async send(baseUrl) {
      const response = await fetch(new URL("/api/system/update_model_pricing", baseUrl), {
        method: "POST",
        headers: {
          "content-type": "application/json",
        },
        body: JSON.stringify({
          modelId: smokeUsage.pricingModelId,
          displayName: "Smoke Web Model",
          inputCost: "1.25",
          outputCost: "2.50",
          cacheReadCost: "0.10",
          cacheCreationCost: "0.05",
        }),
      });
      const result = await response.json();
      const pricingResponse = await fetch(new URL("/api/system/get_model_pricing", baseUrl), {
        method: "POST",
      });
      const pricing = await pricingResponse.json();
      return { response, payload: { result, pricing } };
    },
    validate(response, payload) {
      const smokeEntry = Array.isArray(payload?.pricing)
        ? payload.pricing.find((item) => item?.modelId === smokeUsage.pricingModelId)
        : null;
      if (
        !response.ok ||
        payload?.result !== null ||
        !smokeEntry ||
        smokeEntry.displayName !== "Smoke Web Model" ||
        smokeEntry.inputCostPerMillion !== "1.25" ||
        smokeEntry.outputCostPerMillion !== "2.50" ||
        smokeEntry.cacheReadCostPerMillion !== "0.10" ||
        smokeEntry.cacheCreationCostPerMillion !== "0.05"
      ) {
        throw new Error(`expected model pricing upsert to persist, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "model-pricing-delete",
    method: "POST",
    path: "/api/system/delete_model_pricing",
    async send(baseUrl) {
      const response = await fetch(new URL("/api/system/delete_model_pricing", baseUrl), {
        method: "POST",
        headers: {
          "content-type": "application/json",
        },
        body: JSON.stringify({
          modelId: smokeUsage.pricingModelId,
        }),
      });
      const result = await response.json();
      const pricingResponse = await fetch(new URL("/api/system/get_model_pricing", baseUrl), {
        method: "POST",
      });
      const pricing = await pricingResponse.json();
      return { response, payload: { result, pricing } };
    },
    validate(response, payload) {
      if (
        !response.ok ||
        payload?.result !== null ||
        !Array.isArray(payload?.pricing) ||
        payload.pricing.some((item) => item?.modelId === smokeUsage.pricingModelId)
      ) {
        throw new Error(`expected model pricing delete to remove smoke entry, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "subscription-quota-claude-not-found",
    method: "GET",
    path: "/api/subscription/get-subscription-quota?tool=claude",
    validate(response, payload) {
      if (
        !response.ok ||
        payload?.tool !== "claude" ||
        payload?.credentialStatus !== "not_found" ||
        payload?.success !== false ||
        !Array.isArray(payload?.tiers) ||
        payload.tiers.length !== 0
      ) {
        throw new Error(`expected claude subscription quota not-found state, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "subscription-quota-gemini-parse-error",
    method: "GET",
    path: "/api/subscription/get-subscription-quota?tool=gemini",
    async send(baseUrl, artifacts) {
      await writeFixtureFile(
        path.join(artifacts.homeDir, ".gemini", "oauth_creds.json"),
        "{invalid-json",
      );
      const response = await fetch(
        new URL("/api/subscription/get-subscription-quota?tool=gemini", baseUrl),
      );
      const payload = await response.json();
      return { response, payload };
    },
    validate(response, payload) {
      if (
        !response.ok ||
        payload?.tool !== "gemini" ||
        payload?.credentialStatus !== "parse_error" ||
        payload?.success !== false ||
        typeof payload?.error !== "string" ||
        !payload.error.includes("Failed to parse Gemini credentials")
      ) {
        throw new Error(`expected gemini subscription quota parse-error state, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "balance-unknown-provider",
    method: "GET",
    path: `/api/usage/get-balance?baseUrl=${encodeURIComponent("https://unknown-balance.example.com")}&apiKey=smoke`,
    validate(response, payload) {
      if (
        !response.ok ||
        payload?.success !== false ||
        payload?.error !== "Unknown balance provider"
      ) {
        throw new Error(`expected deterministic unknown-provider balance state, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "coding-plan-unknown-provider",
    method: "GET",
    path: `/api/usage/get-coding-plan-quota?baseUrl=${encodeURIComponent("https://unknown-plan.example.com")}&apiKey=smoke`,
    validate(response, payload) {
      if (
        !response.ok ||
        payload?.tool !== "coding_plan" ||
        payload?.credentialStatus !== "not_found" ||
        payload?.success !== false ||
        !Array.isArray(payload?.tiers) ||
        payload.tiers.length !== 0
      ) {
        throw new Error(`expected deterministic unknown-provider coding plan state, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "usage-script-invalid-app",
    method: "POST",
    path: "/api/usage/testusagescript",
    body: {
      providerId: "smoke-provider",
      app: "invalid",
      scriptCode: "return [];",
    },
    validate(response, payload) {
      if (response.status !== 400 || payload?.code !== "BAD_REQUEST") {
        throw new Error(`expected invalid app usage script probe to fail with BAD_REQUEST, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "workspace-read-file",
    method: "GET",
    path: `/api/workspace/read-workspace-file?filename=${encodeURIComponent(smokeWorkspace.filename)}`,
    validate(response, payload) {
      if (!response.ok || payload !== smokeWorkspace.initialContent) {
        throw new Error(`expected seeded workspace file content, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "workspace-write-file",
    method: "PUT",
    path: "/api/workspace/write-workspace-file",
    async send(baseUrl, artifacts) {
      const response = await fetch(new URL("/api/workspace/write-workspace-file", baseUrl), {
        method: "PUT",
        headers: {
          "content-type": "application/json",
        },
        body: JSON.stringify({
          filename: smokeWorkspace.filename,
          content: smokeWorkspace.updatedContent,
        }),
      });
      const result = await response.json();
      const fileContent = await fs.readFile(
        getWorkspaceFilePath(artifacts.homeDir),
        "utf8",
      );
      return { response, payload: { result, fileContent } };
    },
    validate(response, payload) {
      if (
        !response.ok ||
        payload?.result !== null ||
        payload?.fileContent !== smokeWorkspace.updatedContent
      ) {
        throw new Error(`expected workspace write to update live file, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "workspace-open-directory-desktop-only",
    method: "POST",
    path: "/api/workspace/open-workspace-directory",
    body: { subdir: "workspace" },
    validate(response, payload) {
      if (response.status !== 501 || payload?.code !== "WEB_DESKTOP_ONLY") {
        throw new Error(`expected workspace open-directory route to be desktop-only, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "daily-memory-list",
    method: "POST",
    path: "/api/system/list_daily_memory_files",
    validate(response, payload) {
      if (
        !response.ok ||
        !Array.isArray(payload) ||
        payload.length !== 1 ||
        payload[0]?.filename !== smokeDailyMemory.filename ||
        !String(payload[0]?.preview ?? "").includes(smokeDailyMemory.query)
      ) {
        throw new Error(`expected seeded daily memory list entry, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "daily-memory-read",
    method: "POST",
    path: "/api/system/read_daily_memory_file",
    body: { filename: smokeDailyMemory.filename },
    validate(response, payload) {
      if (!response.ok || payload !== smokeDailyMemory.initialContent) {
        throw new Error(`expected seeded daily memory content, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "daily-memory-write",
    method: "POST",
    path: "/api/system/write_daily_memory_file",
    async send(baseUrl, artifacts) {
      const response = await fetch(new URL("/api/system/write_daily_memory_file", baseUrl), {
        method: "POST",
        headers: {
          "content-type": "application/json",
        },
        body: JSON.stringify({
          filename: smokeDailyMemory.filename,
          content: smokeDailyMemory.updatedContent,
        }),
      });
      const result = await response.json();
      const fileContent = await fs.readFile(
        getDailyMemoryPath(artifacts.homeDir),
        "utf8",
      );
      return { response, payload: { result, fileContent } };
    },
    validate(response, payload) {
      if (
        !response.ok ||
        payload?.result !== null ||
        payload?.fileContent !== smokeDailyMemory.updatedContent
      ) {
        throw new Error(`expected daily memory write to update live file, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "daily-memory-search",
    method: "POST",
    path: "/api/system/search_daily_memory_files",
    body: { query: smokeDailyMemory.query },
    validate(response, payload) {
      if (
        !response.ok ||
        !Array.isArray(payload) ||
        payload.length !== 1 ||
        payload[0]?.filename !== smokeDailyMemory.filename ||
        !String(payload[0]?.snippet ?? "").toLowerCase().includes(smokeDailyMemory.query)
      ) {
        throw new Error(`expected daily memory search result, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "daily-memory-delete",
    method: "POST",
    path: "/api/system/delete_daily_memory_file",
    async send(baseUrl, artifacts) {
      const response = await fetch(new URL("/api/system/delete_daily_memory_file", baseUrl), {
        method: "POST",
        headers: {
          "content-type": "application/json",
        },
        body: JSON.stringify({
          filename: smokeDailyMemory.filename,
        }),
      });
      const result = await response.json();
      const listResponse = await fetch(new URL("/api/system/list_daily_memory_files", baseUrl), {
        method: "POST",
      });
      const remaining = await listResponse.json();
      let fileExists = true;
      try {
        await fs.access(getDailyMemoryPath(artifacts.homeDir));
      } catch {
        fileExists = false;
      }
      return { response, payload: { result, remaining, fileExists } };
    },
    validate(response, payload) {
      if (
        !response.ok ||
        payload?.result !== null ||
        payload?.fileExists !== false ||
        !Array.isArray(payload?.remaining) ||
        payload.remaining.length !== 0
      ) {
        throw new Error(`expected daily memory delete to remove live file, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "openclaw-get-env",
    method: "GET",
    path: "/api/env/get-openclaw-env",
    validate(response, payload) {
      if (
        !response.ok ||
        payload?.vars?.OPENCLAW_TOKEN !== smokeOpenClaw.env.vars.OPENCLAW_TOKEN ||
        payload?.shellEnv?.PATH !== smokeOpenClaw.env.shellEnv.PATH
      ) {
        throw new Error(`expected seeded OpenClaw env config, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "openclaw-set-env",
    method: "PUT",
    path: "/api/env/set-openclaw-env",
    async send(baseUrl, artifacts) {
      const response = await fetch(new URL("/api/env/set-openclaw-env", baseUrl), {
        method: "PUT",
        headers: {
          "content-type": "application/json",
        },
        body: JSON.stringify({
          env: smokeOpenClaw.envUpdated,
        }),
      });
      const result = await response.json();
      const configText = await fs.readFile(
        getOpenClawConfigPath(artifacts.homeDir),
        "utf8",
      );
      const liveEnvResponse = await fetch(new URL("/api/env/get-openclaw-env", baseUrl));
      const liveEnv = await liveEnvResponse.json();
      return { response, payload: { result, configText, liveEnv } };
    },
    validate(response, payload) {
      if (
        !response.ok ||
        payload?.liveEnv?.vars?.OPENCLAW_TOKEN !==
          smokeOpenClaw.envUpdated.vars.OPENCLAW_TOKEN ||
        payload?.liveEnv?.shellEnv?.HOME !== smokeOpenClaw.envUpdated.shellEnv.HOME ||
        !String(payload?.configText ?? "").includes("smoke-openclaw-env-updated") ||
        !String(payload?.configText ?? "").includes("/srv/smoke")
      ) {
        throw new Error(`expected OpenClaw env writeback to update live config, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "openclaw-get-tools",
    method: "GET",
    path: "/api/openclaw/get-openclaw-tools",
    validate(response, payload) {
      if (
        !response.ok ||
        payload?.profile !== smokeOpenClaw.tools.profile ||
        !Array.isArray(payload?.allow) ||
        payload.allow[0] !== smokeOpenClaw.tools.allow[0] ||
        !Array.isArray(payload?.deny) ||
        payload.deny[0] !== smokeOpenClaw.tools.deny[0] ||
        payload?.passthrough !== smokeOpenClaw.tools.passthrough
      ) {
        throw new Error(`expected seeded OpenClaw tools config, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "openclaw-scan-health-initial",
    method: "GET",
    path: "/api/config/scan-openclaw-config-health",
    validate(response, payload) {
      const warningCodes = Array.isArray(payload)
        ? payload.map((warning) => warning?.code)
        : [];
      if (
        !response.ok ||
        !warningCodes.includes("invalid_tools_profile") ||
        !warningCodes.includes("legacy_agents_timeout")
      ) {
        throw new Error(`expected OpenClaw health warnings for legacy profile/timeout, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "openclaw-set-tools",
    method: "PUT",
    path: "/api/openclaw/set-openclaw-tools",
    async send(baseUrl, artifacts) {
      const response = await fetch(new URL("/api/openclaw/set-openclaw-tools", baseUrl), {
        method: "PUT",
        headers: {
          "content-type": "application/json",
        },
        body: JSON.stringify({
          tools: smokeOpenClaw.toolsUpdated,
        }),
      });
      const result = await response.json();
      const configText = await fs.readFile(
        getOpenClawConfigPath(artifacts.homeDir),
        "utf8",
      );
      const liveToolsResponse = await fetch(
        new URL("/api/openclaw/get-openclaw-tools", baseUrl),
      );
      const liveTools = await liveToolsResponse.json();
      return { response, payload: { result, configText, liveTools } };
    },
    validate(response, payload) {
      const configText = String(payload?.configText ?? "");
      if (
        !response.ok ||
        payload?.liveTools?.profile !== smokeOpenClaw.toolsUpdated.profile ||
        payload?.liveTools?.passthrough !== smokeOpenClaw.toolsUpdated.passthrough ||
        !Array.isArray(payload?.liveTools?.allow) ||
        payload.liveTools.allow.length !== smokeOpenClaw.toolsUpdated.allow.length ||
        !Array.isArray(payload?.liveTools?.deny) ||
        payload.liveTools.deny.length !== smokeOpenClaw.toolsUpdated.deny.length ||
        !/profile\s*:\s*"full"/.test(configText) &&
          !/"profile"\s*:\s*"full"/.test(configText)
      ) {
        throw new Error(`expected OpenClaw tools writeback to update live config, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "openclaw-get-agents-defaults",
    method: "GET",
    path: "/api/openclaw/get-openclaw-agents-defaults",
    validate(response, payload) {
      if (
        !response.ok ||
        payload?.workspace !== smokeOpenClaw.agents.workspace ||
        payload?.timeout !== smokeOpenClaw.agents.timeout ||
        payload?.contextTokens !== smokeOpenClaw.agents.contextTokens ||
        payload?.maxConcurrent !== smokeOpenClaw.agents.maxConcurrent ||
        payload?.unknownFlag !== true ||
        payload?.model?.primary !== smokeOpenClaw.agents.model.primary
      ) {
        throw new Error(`expected seeded OpenClaw agents.defaults config, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "openclaw-set-agents-defaults",
    method: "PUT",
    path: "/api/openclaw/set-openclaw-agents-defaults",
    async send(baseUrl, artifacts) {
      const response = await fetch(
        new URL("/api/openclaw/set-openclaw-agents-defaults", baseUrl),
        {
          method: "PUT",
          headers: {
            "content-type": "application/json",
          },
          body: JSON.stringify({
            defaults: smokeOpenClaw.agentsUpdated,
          }),
        },
      );
      const result = await response.json();
      const configText = await fs.readFile(
        getOpenClawConfigPath(artifacts.homeDir),
        "utf8",
      );
      const defaultsResponse = await fetch(
        new URL("/api/openclaw/get-openclaw-agents-defaults", baseUrl),
      );
      const liveDefaults = await defaultsResponse.json();
      const warningsResponse = await fetch(
        new URL("/api/config/scan-openclaw-config-health", baseUrl),
      );
      const warnings = await warningsResponse.json();
      return { response, payload: { result, configText, liveDefaults, warnings } };
    },
    validate(response, payload) {
      const warningCodes = Array.isArray(payload?.warnings)
        ? payload.warnings.map((warning) => warning?.code)
        : [];
      const configText = String(payload?.configText ?? "");
      if (
        !response.ok ||
        payload?.liveDefaults?.timeoutSeconds !==
          smokeOpenClaw.agentsUpdated.timeoutSeconds ||
        payload?.liveDefaults?.timeout !== undefined ||
        payload?.liveDefaults?.workspace !== smokeOpenClaw.agentsUpdated.workspace ||
        payload?.liveDefaults?.contextTokens !==
          smokeOpenClaw.agentsUpdated.contextTokens ||
        payload?.liveDefaults?.maxConcurrent !==
          smokeOpenClaw.agentsUpdated.maxConcurrent ||
        payload?.liveDefaults?.unknownFlag !== true ||
        /"?timeout"?\s*:/.test(configText) ||
        !/"?timeoutSeconds"?\s*:\s*480/.test(configText) ||
        warningCodes.includes("legacy_agents_timeout")
      ) {
        throw new Error(`expected OpenClaw agents.defaults writeback to migrate timeout and clear warning, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "hermes-memory-limits",
    method: "GET",
    path: "/api/hermes/get-hermes-memory-limits",
    validate(response, payload) {
      if (
        !response.ok ||
        payload?.memory !== 2200 ||
        payload?.user !== 1375 ||
        payload?.memoryEnabled !== true ||
        payload?.userEnabled !== false
      ) {
        throw new Error(`expected seeded Hermes memory limits, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "hermes-memory-read",
    method: "GET",
    path: "/api/hermes/get-hermes-memory?kind=memory",
    validate(response, payload) {
      if (!response.ok || payload !== smokeHermesMemory.memoryInitial) {
        throw new Error(`expected seeded Hermes memory content, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "hermes-memory-write",
    method: "PUT",
    path: "/api/hermes/set-hermes-memory",
    async send(baseUrl, artifacts) {
      const response = await fetch(new URL("/api/hermes/set-hermes-memory", baseUrl), {
        method: "PUT",
        headers: {
          "content-type": "application/json",
        },
        body: JSON.stringify({
          kind: "memory",
          content: smokeHermesMemory.memoryUpdated,
        }),
      });
      const result = await response.json();
      const fileContent = await fs.readFile(
        getHermesMemoryPath(artifacts.homeDir, "memory"),
        "utf8",
      );
      const readResponse = await fetch(
        new URL("/api/hermes/get-hermes-memory?kind=memory", baseUrl),
      );
      const liveContent = await readResponse.json();
      return { response, payload: { result, fileContent, liveContent } };
    },
    validate(response, payload) {
      if (
        !response.ok ||
        payload?.result !== null ||
        payload?.fileContent !== smokeHermesMemory.memoryUpdated ||
        payload?.liveContent !== smokeHermesMemory.memoryUpdated
      ) {
        throw new Error(`expected Hermes memory write to update live file, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "hermes-memory-disable",
    method: "PUT",
    path: "/api/hermes/set-hermes-memory-enabled",
    async send(baseUrl, artifacts) {
      const response = await fetch(
        new URL("/api/hermes/set-hermes-memory-enabled", baseUrl),
        {
          method: "PUT",
          headers: {
            "content-type": "application/json",
          },
          body: JSON.stringify({
            kind: "memory",
            enabled: false,
          }),
        },
      );
      const result = await response.json();
      const configText = await fs.readFile(
        getHermesConfigPath(artifacts.homeDir),
        "utf8",
      );
      const limitsResponse = await fetch(
        new URL("/api/hermes/get-hermes-memory-limits", baseUrl),
      );
      const liveLimits = await limitsResponse.json();
      return { response, payload: { result, configText, liveLimits } };
    },
    validate(response, payload) {
      if (
        !response.ok ||
        payload?.liveLimits?.memoryEnabled !== false ||
        payload?.liveLimits?.userEnabled !== false ||
        !String(payload?.configText ?? "").includes("memory_enabled: false") ||
        !String(payload?.configText ?? "").includes("user_profile_enabled: false")
      ) {
        throw new Error(`expected Hermes memory toggle to update config.yaml, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "skills-installed-initial",
    method: "GET",
    path: "/api/skills/get-installed-skills",
    validate(response, payload) {
      if (!response.ok || !Array.isArray(payload) || payload.length !== 0) {
        throw new Error(`expected empty installed skills list, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "import-default-claude",
    method: "POST",
    path: "/api/config/import-default-config",
    body: { app: "claude" },
    validate(response, payload) {
      if (!response.ok || payload !== true) {
        throw new Error(`expected Claude default import to succeed, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "providers-claude-after-import",
    method: "GET",
    path: "/api/providers/get-providers?app=claude",
    validate(response, payload) {
      if (!response.ok || typeof payload !== "object" || payload === null || !payload.default) {
        throw new Error(`expected imported Claude provider, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "update-claude-current-provider-writes-live",
    method: "PUT",
    path: "/api/providers/update-provider",
    async send(baseUrl, artifacts) {
      const listResponse = await fetch(
        new URL("/api/providers/get-providers?app=claude", baseUrl),
      );
      const providers = await listResponse.json();
      const provider = providers.default;
      provider.settingsConfig.env.ANTHROPIC_BASE_URL =
        updatedProviderValues.claudeBaseUrl;

      const response = await fetch(new URL("/api/providers/update-provider", baseUrl), {
        method: "PUT",
        headers: {
          "content-type": "application/json",
        },
        body: JSON.stringify({
          app: "claude",
          provider,
        }),
      });
      const result = await response.json();
      const liveSettings = JSON.parse(
        await fs.readFile(
          path.join(artifacts.homeDir, ".claude", "settings.json"),
          "utf8",
        ),
      );
      return { response, payload: { result, liveSettings } };
    },
    validate(response, payload) {
      if (
        !response.ok ||
        payload?.result !== true ||
        payload?.liveSettings?.env?.ANTHROPIC_BASE_URL !==
          updatedProviderValues.claudeBaseUrl
      ) {
        throw new Error(`expected Claude provider update to succeed, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "switch-claude-provider-writes-live",
    method: "POST",
    path: "/api/providers/switch-provider",
    async send(baseUrl, artifacts) {
      const listResponse = await fetch(
        new URL("/api/providers/get-providers?app=claude", baseUrl),
      );
      const providers = await listResponse.json();
      const altProvider = structuredClone(providers.default);
      altProvider.id = smokeProviderIds.claudeAlt;
      altProvider.name = "Smoke Claude Alt";
      altProvider.settingsConfig.env.ANTHROPIC_BASE_URL =
        "https://claude-switch.example.com";
      altProvider.settingsConfig.env.ANTHROPIC_AUTH_TOKEN =
        "claude-switch-key";

      const addResponse = await fetch(new URL("/api/providers/add-provider", baseUrl), {
        method: "POST",
        headers: {
          "content-type": "application/json",
        },
        body: JSON.stringify({
          app: "claude",
          provider: altProvider,
          addToLive: false,
        }),
      });
      const addPayload = await addResponse.json();
      if (!addResponse.ok || addPayload !== true) {
        return { response: addResponse, payload: { step: "add", addPayload } };
      }

      const response = await fetch(new URL("/api/providers/switch-provider", baseUrl), {
        method: "POST",
        headers: {
          "content-type": "application/json",
        },
        body: JSON.stringify({
          app: "claude",
          id: smokeProviderIds.claudeAlt,
        }),
      });
      const switchResult = await response.json();
      const currentProviderResponse = await fetch(
        new URL("/api/providers/get-current-provider?app=claude", baseUrl),
      );
      const currentProvider = await currentProviderResponse.json();
      const liveSettings = JSON.parse(
        await fs.readFile(
          path.join(artifacts.homeDir, ".claude", "settings.json"),
          "utf8",
        ),
      );
      return {
        response,
        payload: {
          currentProvider,
          currentProviderStatus: currentProviderResponse.status,
          liveSettings,
          switchResult,
        },
      };
    },
    validate(response, payload) {
      if (
        !response.ok ||
        !Array.isArray(payload?.switchResult?.warnings) ||
        payload?.currentProviderStatus !== 200 ||
        payload?.currentProvider !== smokeProviderIds.claudeAlt ||
        payload?.liveSettings?.env?.ANTHROPIC_BASE_URL !==
          "https://claude-switch.example.com"
      ) {
        throw new Error(`expected Claude switch to write live settings, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "proxy-takeover-status",
    method: "GET",
    path: "/api/proxy/get-proxy-takeover-status",
    validate(response, payload) {
      const appKeys = ["claude", "codex", "gemini", "opencode", "openclaw", "hermes"];
      if (
        !response.ok ||
        typeof payload !== "object" ||
        payload === null ||
        !appKeys.every((key) => typeof payload?.[key] === "boolean")
      ) {
        throw new Error(`expected proxy takeover status object, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "proxy-config-claude-initial",
    method: "GET",
    path: "/api/config/get-proxy-config-for-app?appType=claude",
    validate(response, payload) {
      if (
        !response.ok ||
        payload?.appType !== "claude" ||
        typeof payload?.enabled !== "boolean" ||
        typeof payload?.autoFailoverEnabled !== "boolean" ||
        typeof payload?.maxRetries !== "number" ||
        typeof payload?.circuitFailureThreshold !== "number"
      ) {
        throw new Error(`expected Claude app proxy config, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "failover-queue-claude-initial",
    method: "GET",
    path: "/api/failover/get-failover-queue?appType=claude",
    validate(response, payload) {
      if (!response.ok || !Array.isArray(payload) || payload.length !== 0) {
        throw new Error(`expected empty initial Claude failover queue, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "failover-enable-codex-without-queue-blocked",
    method: "PUT",
    path: "/api/failover/set-auto-failover-enabled",
    body: { appType: "codex", enabled: true },
    validate(response, payload) {
      if (response.status !== 400 || payload?.code !== "BAD_REQUEST") {
        throw new Error(`expected enabling failover without queue to fail, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "failover-available-providers-claude",
    method: "GET",
    path: "/api/failover/get-available-providers-for-failover?appType=claude",
    validate(response, payload, artifacts) {
      if (
        !response.ok ||
        !Array.isArray(payload) ||
        payload.length === 0 ||
        typeof payload[0]?.id !== "string"
      ) {
        throw new Error(`expected available Claude failover providers, got ${response.status} ${JSON.stringify(payload)}`);
      }
      artifacts.claudeFailoverProviderId = payload[0].id;
    },
  },
  {
    name: "failover-add-claude-provider",
    method: "POST",
    path: "/api/failover/add-to-failover-queue",
    async send(baseUrl, artifacts) {
      if (!artifacts.claudeFailoverProviderId) {
        throw new Error("missing selected Claude failover provider id");
      }
      const response = await fetch(new URL("/api/failover/add-to-failover-queue", baseUrl), {
        method: "POST",
        headers: {
          "content-type": "application/json",
        },
        body: JSON.stringify({
          appType: "claude",
          providerId: artifacts.claudeFailoverProviderId,
        }),
      });
      const result = await response.json();
      const queueResponse = await fetch(
        new URL("/api/failover/get-failover-queue?appType=claude", baseUrl),
      );
      const queue = await queueResponse.json();
      const availableResponse = await fetch(
        new URL("/api/failover/get-available-providers-for-failover?appType=claude", baseUrl),
      );
      const availableProviders = await availableResponse.json();
      return { response, payload: { result, queue, availableProviders } };
    },
    validate(response, payload, artifacts) {
      if (
        !response.ok ||
        payload?.result !== null ||
        !Array.isArray(payload?.queue) ||
        !payload.queue.some((item) => item?.providerId === artifacts.claudeFailoverProviderId) ||
        !Array.isArray(payload?.availableProviders) ||
        payload.availableProviders.some(
          (provider) => provider?.id === artifacts.claudeFailoverProviderId,
        )
      ) {
        throw new Error(`expected adding Claude provider to failover queue to succeed, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "proxy-update-config-for-claude",
    method: "PUT",
    path: "/api/config/update-proxy-config-for-app",
    async send(baseUrl) {
      const configResponse = await fetch(
        new URL("/api/config/get-proxy-config-for-app?appType=claude", baseUrl),
      );
      const config = await configResponse.json();
      const nextConfig = {
        ...config,
        maxRetries: 4,
        streamingFirstByteTimeout: 45,
        streamingIdleTimeout: 90,
        nonStreamingTimeout: 540,
        circuitFailureThreshold: 6,
        circuitSuccessThreshold: 3,
        circuitTimeoutSeconds: 75,
        circuitErrorRateThreshold: 0.4,
        circuitMinRequests: 12,
      };
      const response = await fetch(
        new URL("/api/config/update-proxy-config-for-app", baseUrl),
        {
          method: "PUT",
          headers: {
            "content-type": "application/json",
          },
          body: JSON.stringify({
            config: nextConfig,
          }),
        },
      );
      const result = await response.json();
      const updatedResponse = await fetch(
        new URL("/api/config/get-proxy-config-for-app?appType=claude", baseUrl),
      );
      const updatedConfig = await updatedResponse.json();
      return { response, payload: { result, updatedConfig } };
    },
    validate(response, payload) {
      if (
        !response.ok ||
        payload?.result !== null ||
        payload?.updatedConfig?.maxRetries !== 4 ||
        payload?.updatedConfig?.streamingFirstByteTimeout !== 45 ||
        payload?.updatedConfig?.streamingIdleTimeout !== 90 ||
        payload?.updatedConfig?.nonStreamingTimeout !== 540 ||
        payload?.updatedConfig?.circuitFailureThreshold !== 6 ||
        payload?.updatedConfig?.circuitSuccessThreshold !== 3 ||
        payload?.updatedConfig?.circuitTimeoutSeconds !== 75 ||
        payload?.updatedConfig?.circuitErrorRateThreshold !== 0.4 ||
        payload?.updatedConfig?.circuitMinRequests !== 12
      ) {
        throw new Error(`expected Claude app proxy config update to persist, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "failover-enable-claude",
    method: "PUT",
    path: "/api/failover/set-auto-failover-enabled",
    async send(baseUrl) {
      const response = await fetch(new URL("/api/failover/set-auto-failover-enabled", baseUrl), {
        method: "PUT",
        headers: {
          "content-type": "application/json",
        },
        body: JSON.stringify({
          appType: "claude",
          enabled: true,
        }),
      });
      const result = await response.json();
      const enabledResponse = await fetch(
        new URL("/api/failover/get-auto-failover-enabled?appType=claude", baseUrl),
      );
      const enabled = await enabledResponse.json();
      const configResponse = await fetch(
        new URL("/api/config/get-proxy-config-for-app?appType=claude", baseUrl),
      );
      const config = await configResponse.json();
      return { response, payload: { result, enabled, config } };
    },
    validate(response, payload) {
      if (
        !response.ok ||
        payload?.result !== null ||
        payload?.enabled !== true ||
        payload?.config?.autoFailoverEnabled !== true
      ) {
        throw new Error(`expected enabling Claude auto failover to persist, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "failover-runtime-stats-web-not-supported",
    method: "POST",
    path: "/api/system/get_circuit_breaker_stats",
    async send(baseUrl, artifacts) {
      if (!artifacts.claudeFailoverProviderId) {
        throw new Error("missing Claude failover provider id for runtime-stats probe");
      }
      const response = await fetch(
        new URL("/api/system/get_circuit_breaker_stats", baseUrl),
        {
          method: "POST",
          headers: {
            "content-type": "application/json",
          },
          body: JSON.stringify({
            providerId: artifacts.claudeFailoverProviderId,
            appType: "claude",
          }),
        },
      );
      const payload = await response.json();
      return { response, payload };
    },
    validate(response, payload) {
      if (response.status !== 501 || payload?.code !== "WEB_NOT_SUPPORTED") {
        throw new Error(`expected circuit breaker runtime stats to be unavailable in web mode, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "failover-disable-and-remove-claude",
    method: "DELETE",
    path: "/api/failover/remove-from-failover-queue",
    async send(baseUrl, artifacts) {
      if (!artifacts.claudeFailoverProviderId) {
        throw new Error("missing Claude failover provider id for remove probe");
      }
      const disableResponse = await fetch(
        new URL("/api/failover/set-auto-failover-enabled", baseUrl),
        {
          method: "PUT",
          headers: {
            "content-type": "application/json",
          },
          body: JSON.stringify({
            appType: "claude",
            enabled: false,
          }),
        },
      );
      const disableResult = await disableResponse.json();
      const response = await fetch(
        new URL(
          `/api/failover/remove-from-failover-queue?appType=claude&providerId=${encodeURIComponent(artifacts.claudeFailoverProviderId)}`,
          baseUrl,
        ),
        {
          method: "DELETE",
        },
      );
      const result = await response.json();
      const queueResponse = await fetch(
        new URL("/api/failover/get-failover-queue?appType=claude", baseUrl),
      );
      const queue = await queueResponse.json();
      const enabledResponse = await fetch(
        new URL("/api/failover/get-auto-failover-enabled?appType=claude", baseUrl),
      );
      const enabled = await enabledResponse.json();
      return { response, payload: { disableResult, result, queue, enabled } };
    },
    validate(response, payload) {
      if (
        !response.ok ||
        payload?.disableResult !== null ||
        payload?.result !== null ||
        !Array.isArray(payload?.queue) ||
        payload.queue.length !== 0 ||
        payload?.enabled !== false
      ) {
        throw new Error(`expected disabling and removing Claude failover queue item to succeed, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "install-skills-upload",
    method: "POST",
    path: "/api/skills/install-skills-upload?app=claude",
    async send(baseUrl, artifacts) {
      const zipBytes = Buffer.from(smokeSkill.zipBase64, "base64");
      const formData = new FormData();
      formData.set(
        "file",
        new Blob([zipBytes], { type: "application/zip" }),
        smokeSkill.fileName,
      );
      const response = await fetch(
        new URL("/api/skills/install-skills-upload?app=claude", baseUrl),
        {
          method: "POST",
          body: formData,
        },
      );
      const installed = await response.json();
      const skillPath = path.join(
        artifacts.homeDir,
        ".claude",
        "skills",
        smokeSkill.installName,
        "SKILL.md",
      );
      const installedSkillMd = await fs.readFile(skillPath, "utf8");
      return {
        response,
        payload: { installed, installedSkillMd },
      };
    },
    validate(response, payload) {
      if (
        !response.ok ||
        !Array.isArray(payload?.installed) ||
        payload.installed.length !== 1 ||
        payload.installed[0]?.id !== smokeSkill.id ||
        payload.installed[0]?.directory !== smokeSkill.installName ||
        typeof payload.installedSkillMd !== "string" ||
        !payload.installedSkillMd.includes("# Smoke Skill")
      ) {
        throw new Error(`expected successful skills ZIP install, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "skills-installed-after-upload",
    method: "GET",
    path: "/api/skills/get-installed-skills",
    validate(response, payload) {
      if (
        !response.ok ||
        !Array.isArray(payload) ||
        !payload.some((skill) => skill?.id === smokeSkill.id)
      ) {
        throw new Error(`expected uploaded skill in installed list, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "import-default-codex",
    method: "POST",
    path: "/api/config/import-default-config",
    body: { app: "codex" },
    validate(response, payload) {
      if (!response.ok || payload !== true) {
        throw new Error(`expected Codex default import to succeed, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "providers-codex-after-import",
    method: "GET",
    path: "/api/providers/get-providers?app=codex",
    validate(response, payload) {
      if (!response.ok || typeof payload !== "object" || payload === null || !payload.default) {
        throw new Error(`expected imported Codex provider, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "import-default-gemini",
    method: "POST",
    path: "/api/config/import-default-config",
    body: { app: "gemini" },
    validate(response, payload) {
      if (!response.ok || payload !== true) {
        throw new Error(`expected Gemini default import to succeed, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "providers-gemini-after-import",
    method: "GET",
    path: "/api/providers/get-providers?app=gemini",
    validate(response, payload) {
      if (!response.ok || typeof payload !== "object" || payload === null || !payload.default) {
        throw new Error(`expected imported Gemini provider, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "import-opencode-from-live",
    method: "POST",
    path: "/api/providers/import-opencode-providers-from-live",
    body: {},
    validate(response, payload) {
      if (!response.ok || payload !== 1) {
        throw new Error(`expected OpenCode live import count 1, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "providers-opencode-after-import",
    method: "GET",
    path: "/api/providers/get-providers?app=opencode",
    validate(response, payload) {
      if (
        !response.ok ||
        typeof payload !== "object" ||
        payload === null ||
        !payload[seededProviderIds.opencode]
      ) {
        throw new Error(`expected imported OpenCode provider, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "update-opencode-live-managed-provider-writes-live",
    method: "PUT",
    path: "/api/providers/update-provider",
    async send(baseUrl, artifacts) {
      const listResponse = await fetch(
        new URL("/api/providers/get-providers?app=opencode", baseUrl),
      );
      const providers = await listResponse.json();
      const provider = providers[seededProviderIds.opencode];
      provider.settingsConfig.options.baseURL =
        updatedProviderValues.opencodeBaseUrl;

      const response = await fetch(new URL("/api/providers/update-provider", baseUrl), {
        method: "PUT",
        headers: {
          "content-type": "application/json",
        },
        body: JSON.stringify({
          app: "opencode",
          provider,
        }),
      });
      const result = await response.json();
      const liveConfig = JSON.parse(
        await fs.readFile(
          path.join(artifacts.homeDir, ".config", "opencode", "opencode.json"),
          "utf8",
        ),
      );
      return { response, payload: { result, liveConfig } };
    },
    validate(response, payload) {
      if (
        !response.ok ||
        payload?.result !== true ||
        payload?.liveConfig?.provider?.[seededProviderIds.opencode]?.options?.baseURL !==
          updatedProviderValues.opencodeBaseUrl
      ) {
        throw new Error(`expected OpenCode provider update to succeed, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "import-openclaw-from-live",
    method: "POST",
    path: "/api/openclaw/import-openclaw-providers-from-live",
    body: {},
    validate(response, payload) {
      if (!response.ok || payload !== 1) {
        throw new Error(`expected OpenClaw live import count 1, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "providers-openclaw-after-import",
    method: "GET",
    path: "/api/providers/get-providers?app=openclaw",
    validate(response, payload) {
      if (
        !response.ok ||
        typeof payload !== "object" ||
        payload === null ||
        !payload[seededProviderIds.openclaw]
      ) {
        throw new Error(`expected imported OpenClaw provider, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "import-hermes-from-live",
    method: "POST",
    path: "/api/hermes/import-hermes-providers-from-live",
    body: {},
    validate(response, payload) {
      if (!response.ok || payload !== 1) {
        throw new Error(`expected Hermes live import count 1, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "providers-hermes-after-import",
    method: "GET",
    path: "/api/providers/get-providers?app=hermes",
    validate(response, payload) {
      if (
        !response.ok ||
        typeof payload !== "object" ||
        payload === null ||
        !payload[seededProviderIds.hermes]
      ) {
        throw new Error(`expected imported Hermes provider, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "deeplink-parse-provider",
    method: "POST",
    path: "/api/deeplink/parse-deeplink",
    body: { url: smokeDeepLink.url },
    validate(response, payload) {
      if (
        !response.ok ||
        payload?.resource !== "provider" ||
        payload?.app !== "openclaw" ||
        payload?.name !== "Smoke DeepLink" ||
        payload?.endpoint !== smokeDeepLink.expectedEndpoint ||
        payload?.apiKey !== "sk-deeplink"
      ) {
        throw new Error(`expected deep link parse result, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "deeplink-merge-provider-config",
    method: "POST",
    path: "/api/config/merge-deeplink-config",
    body: {
      request: {
        version: "v1",
        resource: "provider",
        app: "openclaw",
        name: "Smoke DeepLink Merge",
        config: smokeDeepLink.configBase64,
        configFormat: "json",
      },
    },
    validate(response, payload) {
      if (
        !response.ok ||
        payload?.endpoint !== "https://deeplink-merged.example.com/v1" ||
        payload?.apiKey !== "sk-merged" ||
        payload?.homepage !== "https://deeplink-merged.example.com"
      ) {
        throw new Error(`expected deep link config merge result, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "deeplink-import-provider-unified",
    method: "POST",
    path: "/api/deeplink/import-from-deeplink-unified",
    async send(baseUrl, artifacts) {
      const response = await fetch(
        new URL("/api/deeplink/import-from-deeplink-unified", baseUrl),
        {
          method: "POST",
          headers: {
            "content-type": "application/json",
          },
          body: JSON.stringify({
            request: {
              version: "v1",
              resource: "provider",
              app: "openclaw",
              name: "Smoke DeepLink",
              endpoint: smokeDeepLink.expectedEndpoint,
              apiKey: "sk-deeplink",
            },
          }),
        },
      );
      const payload = await response.json();
      artifacts.importedDeeplinkProviderId = payload?.id ?? null;
      const configText = await fs.readFile(
        getOpenClawConfigPath(artifacts.homeDir),
        "utf8",
      );
      return { response, payload: { result: payload, configText } };
    },
    validate(response, payload) {
      if (
        !response.ok ||
        payload?.result?.type !== "provider" ||
        typeof payload?.result?.id !== "string" ||
        !String(payload?.configText ?? "").includes(payload.result.id) ||
        !String(payload?.configText ?? "").includes(smokeDeepLink.expectedEndpoint)
      ) {
        throw new Error(`expected deep link provider import to update live config, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "providers-openclaw-after-deeplink-import",
    method: "GET",
    path: "/api/providers/get-providers?app=openclaw",
    validate(response, payload, artifacts) {
      const provider = artifacts.importedDeeplinkProviderId
        ? payload?.[artifacts.importedDeeplinkProviderId]
        : null;
      if (
        !response.ok ||
        !artifacts.importedDeeplinkProviderId ||
        typeof payload !== "object" ||
        payload === null ||
        !provider ||
        provider?.name !== "Smoke DeepLink" ||
        provider?.settingsConfig?.baseUrl !== smokeDeepLink.expectedEndpoint
      ) {
        throw new Error(`expected deep link imported provider in OpenClaw provider list, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "export-config-download",
    method: "GET",
    path: "/api/config/export-config-download",
    async send(baseUrl, artifacts) {
      const response = await fetch(new URL("/api/config/export-config-download", baseUrl));
      const payload = await response.text();
      artifacts.exportedSql = payload;
      return { response, payload };
    },
    validate(response, payload) {
      const contentType = response.headers.get("content-type") ?? "";
      const disposition = response.headers.get("content-disposition") ?? "";
      if (!response.ok || !contentType.includes("application/sql")) {
        throw new Error(`expected SQL download, got ${response.status} ${contentType}`);
      }
      if (!disposition.includes("attachment;")) {
        throw new Error(`expected attachment disposition, got ${disposition}`);
      }
      if (typeof payload !== "string" || payload.trim().length === 0) {
        throw new Error("expected non-empty exported SQL payload");
      }
    },
  },
  {
    name: "import-config-upload",
    method: "POST",
    path: "/api/config/import-config-upload",
    async send(baseUrl, artifacts) {
      if (typeof artifacts.exportedSql !== "string" || artifacts.exportedSql.length === 0) {
        throw new Error("missing exported SQL artifact for import smoke");
      }
      const formData = new FormData();
      formData.set(
        "file",
        new Blob([artifacts.exportedSql], { type: "application/sql" }),
        "smoke-export.sql",
      );
      const response = await fetch(new URL("/api/config/import-config-upload", baseUrl), {
        method: "POST",
        body: formData,
      });
      const payload = await response.json();
      return { response, payload };
    },
    validate(response, payload) {
      if (!response.ok || payload?.success !== true || typeof payload?.backupId !== "string") {
        throw new Error(`expected successful SQL upload import, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "uninstall-skill-unified",
    method: "POST",
    path: "/api/skills/uninstall-skill-unified",
    body: { id: smokeSkill.id },
    validate(response, payload) {
      if (!response.ok || typeof payload !== "object" || payload === null) {
        throw new Error(`expected skill uninstall result, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "skills-installed-after-uninstall",
    method: "GET",
    path: "/api/skills/get-installed-skills",
    validate(response, payload) {
      if (
        !response.ok ||
        !Array.isArray(payload) ||
        payload.some((skill) => skill?.id === smokeSkill.id)
      ) {
        throw new Error(`expected uploaded skill to be removed, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "prompts-claude",
    method: "GET",
    path: "/api/prompts/get-prompts?app=claude",
    validate(response, payload) {
      if (!response.ok || typeof payload !== "object" || payload === null) {
        throw new Error(`expected prompts object, got ${response.status}`);
      }
    },
  },
  {
    name: "import-prompt-upload",
    method: "POST",
    path: "/api/prompts/import-prompt-upload?app=claude",
    async send(baseUrl, artifacts) {
      const formData = new FormData();
      formData.set(
        "file",
        new Blob(["# Smoke Prompt\n\nThis prompt was uploaded by smoke-web-server.\n"], {
          type: "text/markdown",
        }),
        "SMOKE_PROMPT.md",
      );
      const response = await fetch(
        new URL("/api/prompts/import-prompt-upload?app=claude", baseUrl),
        {
          method: "POST",
          body: formData,
        },
      );
      const payload = await response.json();
      artifacts.importedPromptId = payload;
      return { response, payload };
    },
    validate(response, payload) {
      if (!response.ok || typeof payload !== "string" || !payload.startsWith("imported-")) {
        throw new Error(`expected imported prompt id, got ${response.status} ${JSON.stringify(payload)}`);
      }
    },
  },
  {
    name: "prompts-claude-after-upload",
    method: "GET",
    path: "/api/prompts/get-prompts?app=claude",
    validate(response, payload, artifacts) {
      if (!response.ok || typeof payload !== "object" || payload === null) {
        throw new Error(`expected prompts object after upload, got ${response.status}`);
      }
      if (
        typeof artifacts.importedPromptId !== "string" ||
        !(artifacts.importedPromptId in payload)
      ) {
        throw new Error(
          `expected uploaded prompt ${String(artifacts.importedPromptId)} to be visible in prompt list`,
        );
      }
    },
  },
  {
    name: "usage-summary",
    method: "GET",
    path: "/api/usage/get-usage-summary",
    validate(response, payload) {
      if (!response.ok || typeof payload !== "object" || payload === null) {
        throw new Error(`expected usage summary JSON, got ${response.status}`);
      }
    },
  },
  {
    name: "usage-data-sources",
    method: "GET",
    path: "/api/usage/get-usage-data-sources",
    validate(response, payload) {
      if (!response.ok || !Array.isArray(payload)) {
        throw new Error(`expected usage data source array, got ${response.status}`);
      }
    },
  },
  {
    name: "check-for-updates",
    method: "POST",
    path: "/api/system/check_for_updates",
    body: {},
    validate(response, payload) {
      if (!response.ok || typeof payload !== "boolean") {
        throw new Error(`expected update bool, got ${response.status}`);
      }
    },
  },
  {
    name: "tool-versions",
    method: "POST",
    path: "/api/system/get_tool_versions",
    body: { tools: ["claude"] },
    validate(response, payload) {
      if (!response.ok || !Array.isArray(payload)) {
        throw new Error(`expected tool version array, got ${response.status}`);
      }
    },
  },
  {
    name: "auth-status-copilot",
    method: "POST",
    path: "/api/auth/auth-get-status",
    body: { authProvider: "github_copilot" },
    validate(response, payload) {
      if (!response.ok || payload?.provider !== "github_copilot" || !Array.isArray(payload?.accounts)) {
        throw new Error(`expected copilot auth status, got ${response.status}`);
      }
    },
  },
  {
    name: "auth-status-codex",
    method: "POST",
    path: "/api/auth/auth-get-status",
    body: { authProvider: "codex_oauth" },
    validate(response, payload) {
      if (!response.ok || payload?.provider !== "codex_oauth" || !Array.isArray(payload?.accounts)) {
        throw new Error(`expected codex auth status, got ${response.status}`);
      }
    },
  },
  {
    name: "desktop-only-open-app-config-folder",
    method: "POST",
    path: "/api/config/open-app-config-folder",
    body: {},
    validate(response, payload) {
      if (response.status !== 501 || payload?.code !== "WEB_DESKTOP_ONLY") {
        throw new Error(
          `expected WEB_DESKTOP_ONLY 501, got ${response.status} ${JSON.stringify(payload)}`,
        );
      }
    },
  },
  {
    name: "desktop-only-open-config-folder",
    method: "POST",
    path: "/api/config/open-config-folder",
    body: { app: "claude" },
    validate(response, payload) {
      if (response.status !== 501 || payload?.code !== "WEB_DESKTOP_ONLY") {
        throw new Error(
          `expected WEB_DESKTOP_ONLY 501 for open-config-folder, got ${response.status} ${JSON.stringify(payload)}`,
        );
      }
    },
  },
  {
    name: "desktop-only-open-provider-terminal",
    method: "POST",
    path: "/api/providers/open-provider-terminal",
    body: { providerId: "default", app: "claude" },
    validate(response, payload) {
      if (response.status !== 501 || payload?.code !== "WEB_DESKTOP_ONLY") {
        throw new Error(
          `expected WEB_DESKTOP_ONLY 501 for open-provider-terminal, got ${response.status} ${JSON.stringify(payload)}`,
        );
      }
    },
  },
  {
    name: "desktop-only-pick-directory",
    method: "POST",
    path: "/api/system/pick_directory",
    body: { defaultPath: "/tmp" },
    validate(response, payload) {
      if (response.status !== 501 || payload?.code !== "WEB_DESKTOP_ONLY") {
        throw new Error(
          `expected WEB_DESKTOP_ONLY 501 for pick_directory, got ${response.status} ${JSON.stringify(payload)}`,
        );
      }
    },
  },
  {
    name: "upload-required-export-config",
    method: "POST",
    path: "/api/config/export-config-to-file",
    body: {},
    validate(response, payload) {
      if (response.status !== 400 || payload?.code !== "WEB_UPLOAD_REQUIRED") {
        throw new Error(
          `expected WEB_UPLOAD_REQUIRED 400, got ${response.status} ${JSON.stringify(payload)}`,
        );
      }
    },
  },
];

async function main() {
  await ensureDistWeb();

  const port = Number(process.env.PORT || (await getFreePort()));
  const host = process.env.HOST || "127.0.0.1";
  const dataDir = await fs.mkdtemp(path.join(os.tmpdir(), "cc-switch-web-smoke-"));
  const homeDir = await fs.mkdtemp(path.join(os.tmpdir(), "cc-switch-web-home-"));
  const baseUrl = `http://${host}:${port}`;

  await seedLiveProviderFixtures(homeDir);
  await seedSessionFixtures(homeDir);
  smokeArtifacts.homeDir = homeDir;

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
      },
      stdio: ["ignore", "pipe", "pipe"],
    },
  );

  child.stdout.on("data", (chunk) => {
    process.stderr.write(`[server] ${chunk}`);
  });
  child.stderr.on("data", (chunk) => {
    process.stderr.write(`[server] ${chunk}`);
  });

  let exitCode = null;
  child.on("exit", (code) => {
    exitCode = code;
  });

  try {
    await waitForServer(baseUrl, child);
    const results = [];

    for (const probe of probes) {
      const { response, payload } = await fetchJson(baseUrl, probe);
      probe.validate(response, payload, smokeArtifacts);
      results.push({
        name: probe.name,
        status: response.status,
      });
    }

    console.log(
      JSON.stringify(
        {
          baseUrl,
          dataDir,
          homeDir,
          probes: results,
        },
        null,
        2,
      ),
    );
  } catch (error) {
    if (exitCode !== null) {
      throw new Error(`web server exited early with code ${exitCode}`);
    }
    throw error;
  } finally {
    await shutdown(child);
  }
}

main().catch((error) => {
  console.error(
    `[smoke-web-server] ${error instanceof Error ? error.message : String(error)}`,
  );
  process.exit(1);
});
