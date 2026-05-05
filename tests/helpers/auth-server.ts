import http from "node:http";
import type { AddressInfo } from "node:net";

type CopilotFlowMode = "pending" | "success" | "access_denied" | "error";
type CodexFlowMode = "pending" | "success" | "access_denied" | "error";

type CopilotFlow = {
  mode: CopilotFlowMode;
  userCode?: string;
  verificationUri?: string;
  account?: {
    id: number;
    login: string;
    avatarUrl?: string | null;
  };
};

type CodexFlow = {
  mode: CodexFlowMode;
  userCode?: string;
  verificationUri?: string;
  account?: {
    accountId: string;
    email: string;
  };
};

type InternalCopilotSession = Required<Omit<CopilotFlow, "verificationUri">> & {
  domain: string;
  deviceCode: string;
  verificationUri: string;
  accessToken: string;
};

type InternalCodexSession = Required<Omit<CodexFlow, "verificationUri">> & {
  deviceCode: string;
  verificationUri: string;
  authorizationCode: string;
  codeVerifier: string;
};

export type TestAuthServer = {
  baseUrl: string;
  queueCopilotFlow: (flow: CopilotFlow) => void;
  queueCodexFlow: (flow: CodexFlow) => void;
  reset: () => void;
  stop: () => Promise<void>;
};

function json(response: http.ServerResponse, status: number, payload: unknown) {
  response.statusCode = status;
  response.setHeader("Content-Type", "application/json; charset=utf-8");
  response.end(JSON.stringify(payload));
}

function text(response: http.ServerResponse, status: number, body: string) {
  response.statusCode = status;
  response.setHeader("Content-Type", "text/plain; charset=utf-8");
  response.end(body);
}

async function readBody(request: http.IncomingMessage): Promise<string> {
  const chunks: Buffer[] = [];
  for await (const chunk of request) {
    chunks.push(Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk));
  }
  return Buffer.concat(chunks).toString("utf8");
}

function makeJwt(payload: Record<string, unknown>): string {
  const header = Buffer.from(
    JSON.stringify({ alg: "none", typ: "JWT" }),
    "utf8",
  ).toString("base64url");
  const body = Buffer.from(JSON.stringify(payload), "utf8").toString(
    "base64url",
  );
  return `${header}.${body}.signature`;
}

export async function startTestAuthServer(): Promise<TestAuthServer> {
  let nextCopilotId = 1;
  let nextCodexId = 1;

  const queuedCopilotFlows: CopilotFlow[] = [];
  const queuedCodexFlows: CodexFlow[] = [];
  const copilotSessions = new Map<string, InternalCopilotSession>();
  const copilotAccessTokens = new Map<
    string,
    { id: number; login: string; avatarUrl: string | null }
  >();
  const codexSessions = new Map<string, InternalCodexSession>();
  const codexAuthCodes = new Map<string, InternalCodexSession>();

  const server = http.createServer(async (request, response) => {
    const requestUrl = new URL(request.url ?? "/", "http://127.0.0.1");
    const pathParts = requestUrl.pathname.split("/").filter(Boolean);

    if (
      request.method === "POST" &&
      pathParts.length === 5 &&
      pathParts[0] === "copilot" &&
      pathParts[2] === "login" &&
      pathParts[3] === "device" &&
      pathParts[4] === "code"
    ) {
      const domain = decodeURIComponent(pathParts[1]);
      const deviceNumber = nextCopilotId++;
      const defaultAccountId = 10_000 + deviceNumber;
      const flow = queuedCopilotFlows.shift() ?? {
        mode: "success" as const,
        account: {
          id: defaultAccountId,
          login: `copilot-user-${deviceNumber}`,
          avatarUrl: null,
        },
      };
      const deviceCode = `copilot-device-${deviceNumber}`;
      const accessToken = `copilot-access-${deviceNumber}`;
      const verificationUri =
        flow.verificationUri ??
        `${requestUrl.origin}/copilot/${encodeURIComponent(domain)}/verify/${deviceCode}`;
      const session: InternalCopilotSession = {
        mode: flow.mode,
        userCode: flow.userCode ?? `GH-${String(deviceNumber).padStart(4, "0")}`,
        verificationUri,
        account: flow.account ?? {
          id: defaultAccountId,
          login: `copilot-user-${deviceNumber}`,
          avatarUrl: null,
        },
        domain,
        deviceCode,
        accessToken,
      };
      copilotSessions.set(deviceCode, session);
      json(response, 200, {
        device_code: session.deviceCode,
        user_code: session.userCode,
        verification_uri: session.verificationUri,
        expires_in: 900,
        interval: 1,
      });
      return;
    }

    if (
      request.method === "POST" &&
      pathParts.length === 5 &&
      pathParts[0] === "copilot" &&
      pathParts[2] === "login" &&
      pathParts[3] === "oauth" &&
      pathParts[4] === "access_token"
    ) {
      const body = await readBody(request);
      const params = new URLSearchParams(body);
      const deviceCode = params.get("device_code") ?? "";
      const session = copilotSessions.get(deviceCode);
      if (!session) {
        json(response, 404, { error: "unknown_device_code" });
        return;
      }

      if (session.mode === "pending") {
        json(response, 200, {
          error: "authorization_pending",
          error_description: "waiting for device authorization",
        });
        return;
      }

      if (session.mode === "access_denied") {
        json(response, 200, {
          error: "access_denied",
          error_description: "denied by fake auth server",
        });
        return;
      }

      if (session.mode === "error") {
        text(response, 500, "copilot device poll failed");
        return;
      }

      copilotAccessTokens.set(session.accessToken, {
        id: session.account.id,
        login: session.account.login,
        avatarUrl: session.account.avatarUrl ?? null,
      });
      json(response, 200, {
        access_token: session.accessToken,
        token_type: "bearer",
      });
      return;
    }

    if (
      request.method === "GET" &&
      pathParts.length === 4 &&
      pathParts[0] === "copilot" &&
      pathParts[2] === "api" &&
      pathParts[3] === "user"
    ) {
      const token = request.headers.authorization?.replace(/^token\s+/i, "");
      const account = token ? copilotAccessTokens.get(token) : null;
      if (!account) {
        json(response, 401, { message: "missing or invalid token" });
        return;
      }
      json(response, 200, {
        id: account.id,
        login: account.login,
        avatar_url: account.avatarUrl,
      });
      return;
    }

    if (
      request.method === "GET" &&
      pathParts.length === 6 &&
      pathParts[0] === "copilot" &&
      pathParts[2] === "api" &&
      pathParts[3] === "copilot_internal" &&
      pathParts[4] === "v2" &&
      pathParts[5] === "token"
    ) {
      const token = request.headers.authorization?.replace(/^token\s+/i, "");
      const account = token ? copilotAccessTokens.get(token) : null;
      if (!account) {
        json(response, 401, { message: "missing or invalid token" });
        return;
      }
      json(response, 200, {
        token: `copilot-api-token-${account.id}`,
        expires_at: 4_102_444_800,
      });
      return;
    }

    if (
      request.method === "POST" &&
      requestUrl.pathname === "/codex/deviceauth/usercode"
    ) {
      const deviceNumber = nextCodexId++;
      const flow = queuedCodexFlows.shift() ?? {
        mode: "success" as const,
        account: {
          accountId: `codex-acc-${deviceNumber}`,
          email: `codex-${deviceNumber}@example.com`,
        },
      };
      const deviceCode = `codex-device-${deviceNumber}`;
      const verificationUri =
        flow.verificationUri ?? `${requestUrl.origin}/codex/verify/${deviceCode}`;
      const session: InternalCodexSession = {
        mode: flow.mode,
        userCode: flow.userCode ?? `OA-${String(deviceNumber).padStart(4, "0")}`,
        verificationUri,
        account: flow.account ?? {
          accountId: `codex-acc-${deviceNumber}`,
          email: `codex-${deviceNumber}@example.com`,
        },
        deviceCode,
        authorizationCode: `codex-auth-${deviceNumber}`,
        codeVerifier: `codex-verifier-${deviceNumber}`,
      };
      codexSessions.set(deviceCode, session);
      json(response, 200, {
        device_auth_id: session.deviceCode,
        user_code: session.userCode,
        interval: 1,
        expires_in: 900,
      });
      return;
    }

    if (
      request.method === "POST" &&
      requestUrl.pathname === "/codex/deviceauth/token"
    ) {
      const body = await readBody(request);
      const parsed = JSON.parse(body || "{}") as {
        device_auth_id?: string;
      };
      const session = parsed.device_auth_id
        ? codexSessions.get(parsed.device_auth_id)
        : null;
      if (!session) {
        text(response, 404, "unknown device_auth_id");
        return;
      }

      if (session.mode === "pending") {
        text(response, 403, "authorization pending");
        return;
      }

      if (session.mode === "access_denied") {
        text(response, 400, "access denied");
        return;
      }

      if (session.mode === "error") {
        text(response, 500, "codex device poll failed");
        return;
      }

      codexAuthCodes.set(session.authorizationCode, session);
      json(response, 200, {
        authorization_code: session.authorizationCode,
        code_verifier: session.codeVerifier,
      });
      return;
    }

    if (request.method === "POST" && requestUrl.pathname === "/codex/oauth/token") {
      const body = await readBody(request);
      const params = new URLSearchParams(body);
      const grantType = params.get("grant_type");

      if (grantType === "authorization_code") {
        const session = codexAuthCodes.get(params.get("code") ?? "");
        if (!session) {
          text(response, 404, "unknown authorization code");
          return;
        }

        const tokenPayload = {
          chatgpt_account_id: session.account.accountId,
          email: session.account.email,
        };
        json(response, 200, {
          access_token: makeJwt(tokenPayload),
          refresh_token: `refresh-${session.account.accountId}`,
          id_token: makeJwt(tokenPayload),
          expires_in: 3600,
        });
        return;
      }

      if (grantType === "refresh_token") {
        const refreshToken = params.get("refresh_token") ?? "";
        const accountId = refreshToken.replace(/^refresh-/, "");
        const tokenPayload = {
          chatgpt_account_id: accountId,
          email: `${accountId}@example.com`,
        };
        json(response, 200, {
          access_token: makeJwt(tokenPayload),
          refresh_token: refreshToken,
          id_token: makeJwt(tokenPayload),
          expires_in: 3600,
        });
        return;
      }

      text(response, 400, "unsupported grant_type");
      return;
    }

    text(response, 404, "not found");
  });

  await new Promise<void>((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => resolve());
  });

  const address = server.address();
  if (!address || typeof address === "string") {
    throw new Error("failed to start auth server");
  }

  const reset = () => {
    queuedCopilotFlows.length = 0;
    queuedCodexFlows.length = 0;
    copilotSessions.clear();
    copilotAccessTokens.clear();
    codexSessions.clear();
    codexAuthCodes.clear();
    nextCopilotId = 1;
    nextCodexId = 1;
  };

  return {
    baseUrl: `http://127.0.0.1:${(address as AddressInfo).port}`,
    queueCopilotFlow: (flow) => {
      queuedCopilotFlows.push(flow);
    },
    queueCodexFlow: (flow) => {
      queuedCodexFlows.push(flow);
    },
    reset,
    stop: async () => {
      await new Promise<void>((resolve, reject) => {
        server.close((error) => {
          if (error) {
            reject(error);
            return;
          }
          resolve();
        });
      });
    },
  };
}
