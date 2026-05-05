import { createHash } from "node:crypto";
import type { AddressInfo } from "node:net";
import http, {
  type IncomingMessage,
  type Server,
  type ServerResponse,
} from "node:http";

type QueuedFailure = {
  method: string;
  path: string;
  status: number;
};

type StoredFile = {
  body: Buffer;
  etag: string;
};

export type TestWebDavServer = {
  baseUrl: string;
  rootPath: string;
  stop: () => Promise<void>;
  reset: () => void;
  readFile: (path: string) => Buffer | undefined;
  failNext: (method: string, path: string, status: number) => void;
};

type StartTestWebDavServerOptions = {
  rootPath?: string;
  username?: string;
  password?: string;
};

function normalizePath(pathname: string): string {
  const decoded = decodeURIComponent(pathname);
  if (decoded === "/") {
    return "/";
  }
  return decoded.replace(/\/+$/, "") || "/";
}

function parentPath(pathname: string): string {
  const normalized = normalizePath(pathname);
  if (normalized === "/") {
    return "/";
  }
  const index = normalized.lastIndexOf("/");
  if (index <= 0) {
    return "/";
  }
  return normalized.slice(0, index);
}

function buildEtag(body: Buffer): string {
  return `"${createHash("sha1").update(body).digest("hex")}"`;
}

async function readBody(request: IncomingMessage): Promise<Buffer> {
  const chunks: Buffer[] = [];
  for await (const chunk of request) {
    chunks.push(Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk));
  }
  return Buffer.concat(chunks);
}

function end(response: ServerResponse, status: number, body?: Buffer | string) {
  response.statusCode = status;
  if (body === undefined) {
    response.end();
    return;
  }

  if (typeof body === "string") {
    response.setHeader("Content-Length", Buffer.byteLength(body));
    response.end(body);
    return;
  }

  response.setHeader("Content-Length", body.length);
  response.end(body);
}

async function listen(server: Server): Promise<AddressInfo> {
  await new Promise<void>((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => {
      server.off("error", reject);
      resolve();
    });
  });

  const address = server.address();
  if (!address || typeof address === "string") {
    throw new Error("failed to start test WebDAV server");
  }

  return address;
}

export async function startTestWebDavServer(
  options: StartTestWebDavServerOptions = {},
): Promise<TestWebDavServer> {
  const rootPath = normalizePath(options.rootPath ?? "/dav");
  const username = options.username ?? "alice";
  const password = options.password ?? "secret";
  const expectedAuth = `Basic ${Buffer.from(`${username}:${password}`).toString("base64")}`;
  const directories = new Set<string>(["/", rootPath]);
  const files = new Map<string, StoredFile>();
  const failures: QueuedFailure[] = [];

  const reset = () => {
    directories.clear();
    directories.add("/");
    directories.add(rootPath);
    files.clear();
    failures.length = 0;
  };

  const server = http.createServer(async (request, response) => {
    const method = (request.method ?? "GET").toUpperCase();
    const pathname = normalizePath(new URL(request.url ?? "/", "http://127.0.0.1").pathname);

    if (!pathname.startsWith(rootPath)) {
      end(response, 404);
      return;
    }

    if (request.headers.authorization !== expectedAuth) {
      response.setHeader("WWW-Authenticate", 'Basic realm="cc-switch-test-webdav"');
      end(response, 401);
      return;
    }

    const failureIndex = failures.findIndex(
      (failure) =>
        failure.method === method && failure.path === pathname,
    );
    if (failureIndex >= 0) {
      const [failure] = failures.splice(failureIndex, 1);
      end(response, failure.status);
      return;
    }

    if (method === "PROPFIND") {
      if (directories.has(pathname)) {
        response.setHeader("Content-Type", "application/xml; charset=utf-8");
        end(
          response,
          207,
          `<?xml version="1.0" encoding="utf-8"?><multistatus><response><href>${pathname}</href></response></multistatus>`,
        );
        return;
      }
      end(response, 404);
      return;
    }

    if (method === "MKCOL") {
      if (directories.has(pathname)) {
        end(response, 405);
        return;
      }

      if (!directories.has(parentPath(pathname))) {
        end(response, 409);
        return;
      }

      directories.add(pathname);
      end(response, 201);
      return;
    }

    if (method === "PUT") {
      if (!directories.has(parentPath(pathname))) {
        end(response, 409);
        return;
      }

      const body = await readBody(request);
      files.set(pathname, {
        body,
        etag: buildEtag(body),
      });
      end(response, 201);
      return;
    }

    if (method === "GET" || method === "HEAD") {
      const file = files.get(pathname);
      if (!file) {
        end(response, 404);
        return;
      }

      response.setHeader("ETag", file.etag);
      response.setHeader("Content-Length", file.body.length);

      if (method === "HEAD") {
        end(response, 200);
        return;
      }

      end(response, 200, file.body);
      return;
    }

    end(response, 405);
  });

  const address = await listen(server);

  return {
    baseUrl: `http://127.0.0.1:${address.port}${rootPath}/`,
    rootPath,
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
    reset,
    readFile: (path: string) => files.get(normalizePath(path))?.body,
    failNext: (method: string, path: string, status: number) => {
      failures.push({
        method: method.toUpperCase(),
        path: normalizePath(path),
        status,
      });
    },
  };
}
