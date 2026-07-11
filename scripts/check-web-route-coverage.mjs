#!/usr/bin/env node
//
// Web route coverage / command-parity gate.
//
// SSOT (deep-read finding M41): `src/lib/api/web-commands.ts` is the
// authoritative source of truth for the command <-> HTTP-route parity surface.
// It is the runtime route table the web adapter resolves command specs from
// (by command name). `commands.manifest.json` at the repo root is only
// advisory/heuristic — it is regenerated from the Tauri `#[tauri::command]`
// surface to help author web-commands.ts, but this check intentionally reads
// web-commands.ts (not the manifest) so the gate validates the surface the app
// actually uses. Do not switch this script to the manifest without resolving
// the two-SSOT split first.
//
// LIMITATION (deep-read finding L10): route discovery below scrapes the Rust
// handlers with a naive `.route("<literal>")` regex. It therefore only sees
// string-literal route paths. Routes that are built from a macro, a variable,
// a `const`, or string concatenation/formatting are invisible to this gate and
// would show up as false "missing" entries (or, for wildcard `/*path` mounts,
// be matched only by the prefix heuristic below). If a route ever fails to be
// detected, prefer making its `.route("...")` argument a string literal over
// loosening this matcher.
//
import fs from "node:fs";
import path from "node:path";
import ts from "typescript";

const root = process.cwd();
const webCommandsPath = path.join(root, "src/lib/api/web-commands.ts");
const handlersDir = path.join(root, "src-tauri/src/web_api/handlers");

const webCommands = fs.readFileSync(webCommandsPath, "utf8");

function unwrapExpression(expr) {
  let current = expr;
  while (
    ts.isAsExpression(current) ||
    ts.isTypeAssertionExpression(current) ||
    ts.isParenthesizedExpression(current) ||
    ts.isSatisfiesExpression(current)
  ) {
    current = current.expression;
  }
  return current;
}

function propertyNameText(name) {
  if (
    ts.isIdentifier(name) ||
    ts.isStringLiteral(name) ||
    ts.isNumericLiteral(name)
  ) {
    return name.text;
  }
  return undefined;
}

function findObjectProperty(objectNode, propertyName) {
  return objectNode.properties.find(
    (property) =>
      ts.isPropertyAssignment(property) &&
      propertyNameText(property.name) === propertyName,
  );
}

function readStringProperty(objectNode, propertyName) {
  const property = findObjectProperty(objectNode, propertyName);
  if (!property) return undefined;
  const value = unwrapExpression(property.initializer);
  if (ts.isStringLiteral(value) || ts.isNoSubstitutionTemplateLiteral(value)) {
    return value.text;
  }
  return undefined;
}

function readBooleanProperty(objectNode, propertyName) {
  const property = findObjectProperty(objectNode, propertyName);
  if (!property) return false;
  const value = unwrapExpression(property.initializer);
  return value.kind === ts.SyntaxKind.TrueKeyword;
}

function findCommandMap(sourceFile) {
  let commandMap;
  function visit(node) {
    if (
      ts.isCallExpression(node) &&
      ts.isIdentifier(node.expression) &&
      node.expression.text === "defineCommands" &&
      node.arguments.length > 0 &&
      ts.isObjectLiteralExpression(node.arguments[0])
    ) {
      commandMap = node.arguments[0];
      return;
    }
    ts.forEachChild(node, visit);
  }
  visit(sourceFile);
  return commandMap;
}

const sourceFile = ts.createSourceFile(
  webCommandsPath,
  webCommands,
  ts.ScriptTarget.Latest,
  true,
  ts.ScriptKind.TS,
);
const commandMap = findCommandMap(sourceFile);
if (!commandMap) {
  console.error(`Could not find defineCommands({...}) in ${webCommandsPath}`);
  process.exit(2);
}

const commands = [];
for (const property of commandMap.properties) {
  if (!ts.isPropertyAssignment(property)) continue;
  const name = propertyNameText(property.name);
  const spec = unwrapExpression(property.initializer);
  if (!name || !ts.isObjectLiteralExpression(spec)) continue;

  const method = readStringProperty(spec, "method");
  const routePath = readStringProperty(spec, "path");
  if (!method || !routePath) {
    console.error(
      `Command ${name} is missing a literal method/path in web-commands.ts`,
    );
    process.exit(2);
  }

  commands.push({
    name,
    method,
    path: routePath,
    unsupported: readBooleanProperty(spec, "unsupported"),
    webReplacement: readBooleanProperty(spec, "webReplacement"),
  });
}

// Extract each `.route("path", <args>)` call's argument string using a balanced
// paren scan, so multi-line route registrations and chained method routers
// (`get(h).post(h2)`) are captured. Route path literals never contain parens.
function extractRouteArgStrings(source) {
  const args = [];
  const marker = ".route(";
  let i = 0;
  while ((i = source.indexOf(marker, i)) !== -1) {
    let depth = 0;
    let j = i + marker.length - 1; // position of the opening '('
    const start = j;
    do {
      const ch = source[j];
      if (ch === "(") depth++;
      else if (ch === ")") depth--;
      j++;
    } while (j < source.length && depth > 0);
    args.push(source.slice(start + 1, j - 1));
    i = j;
  }
  return args;
}

const HTTP_METHOD_WRAPPERS = new Set([
  "get",
  "post",
  "put",
  "delete",
  "patch",
  "head",
  "options",
  "any",
]);
// Handler symbols that are known "not implemented on web" stubs (parity 501 /
// desktop-only responders). Routes wired to these are parity stubs regardless
// of which handler file registers them (L4). NOTE: a handful of desktop-only
// stubs use named local fns returning `ApiError::desktop_only` (e.g. the
// lightweight-mode routes) rather than these shared symbols; those are still
// classified by file (parity.rs) only. A fuller fix would inspect handler
// return types.
const STUB_HANDLER_SYMBOLS = new Set([
  "web_not_supported",
  "web_desktop_only",
  "web_upload_required",
]);

// routes: path -> { file, methods:Set<string>, handlers:Set<string> }
const routes = new Map();
const wildcardPrefixes = [];
for (const file of fs.readdirSync(handlersDir)) {
  if (!file.endsWith(".rs")) continue;
  const source = fs.readFileSync(path.join(handlersDir, file), "utf8");
  for (const argStr of extractRouteArgStrings(source)) {
    const pathMatch = argStr.match(/"([^"]+)"/);
    if (!pathMatch) continue;
    const route = `/api${pathMatch[1]}`;
    const methods = new Set();
    const handlers = new Set();
    const wrapperRe = /\b([a-z]+)\s*\(\s*([A-Za-z_][A-Za-z0-9_]*)/g;
    let m;
    while ((m = wrapperRe.exec(argStr)) !== null) {
      if (!HTTP_METHOD_WRAPPERS.has(m[1])) continue;
      methods.add(m[1]);
      handlers.add(m[2]);
    }
    if (route.endsWith("/*path")) {
      wildcardPrefixes.push({
        prefix: route.slice(0, -"*path".length),
        file,
        handlers,
      });
    } else {
      routes.set(route, { file, methods, handlers });
    }
  }
}

function isStubRoute(entry) {
  if (!entry) return false;
  if (entry.file === "parity.rs") return true;
  for (const handler of entry.handlers) {
    if (STUB_HANDLER_SYMBOLS.has(handler)) return true;
  }
  return false;
}

const missing = commands.filter(
  (command) =>
    !command.unsupported &&
    !command.webReplacement &&
    !routes.has(command.path) &&
    !wildcardPrefixes.some((route) => command.path.startsWith(route.prefix)),
);

// L2: a command whose HTTP method the matching route does not serve would pass
// path-only coverage but 405 at runtime. `any` matches every method.
const methodMismatch = commands
  .filter(
    (command) =>
      !command.unsupported && !command.webReplacement && routes.has(command.path),
  )
  .map((command) => ({ command, entry: routes.get(command.path) }))
  .filter(
    ({ command, entry }) =>
      entry.methods.size > 0 &&
      !entry.methods.has("any") &&
      !entry.methods.has(command.method.toLowerCase()),
  );

// L3: `webReplacement` commands are exempt from the missing check because their
// real endpoint is a hardcoded webFetch literal in src/, not the placeholder
// `path` in web-commands.ts. Scan src/ for those literals and assert each
// resolves to a real Rust route so a rename/typo on either side fails the gate.
const srcDir = path.join(root, "src");
const webFetchLiteralRe =
  /\b(?:webJsonFetch|webUpload|webDownload|webFetch)\s*(?:<[^>]*>)?\s*\(\s*"(\/api\/[^"]+)"/g;
const replacementPaths = new Set();
function scanSrcForWebFetchLiterals(dir) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      scanSrcForWebFetchLiterals(full);
    } else if (/\.(ts|tsx)$/.test(entry.name)) {
      const source = fs.readFileSync(full, "utf8");
      let m;
      while ((m = webFetchLiteralRe.exec(source)) !== null) {
        replacementPaths.add(m[1]);
      }
    }
  }
}
scanSrcForWebFetchLiterals(srcDir);
const danglingReplacementPaths = [...replacementPaths].filter(
  (p) =>
    !routes.has(p) &&
    !wildcardPrefixes.some((route) => p.startsWith(route.prefix)),
);

const parityFallback = commands.filter(
  (command) =>
    !command.unsupported &&
    !command.webReplacement &&
    !routes.has(command.path) &&
    wildcardPrefixes.some(
      (route) =>
        route.file === "parity.rs" && command.path.startsWith(route.prefix),
    ),
);

// L4: classify parity stubs by handler symbol (not just parity.rs filename), so
// a 501/desktop-only stub registered in a concrete handler file is still counted
// as a parity stub rather than masquerading as real coverage.
const parityExact = commands.filter(
  (command) =>
    !command.unsupported &&
    !command.webReplacement &&
    isStubRoute(routes.get(command.path)),
);

const webReplacements = commands.filter((command) => command.webReplacement);

console.log(
  JSON.stringify(
    {
      commands: commands.length,
      routes: routes.size,
      wildcardRoutes: wildcardPrefixes.length,
      unsupported: commands.filter((command) => command.unsupported).length,
      webReplacements: webReplacements.length,
      webFetchLiteralPaths: replacementPaths.size,
      missing: missing.length,
      methodMismatch: methodMismatch.length,
      danglingReplacementPaths: danglingReplacementPaths.length,
      parityExact: parityExact.length,
      parityFallback: parityFallback.length,
    },
    null,
    2,
  ),
);

let failed = false;

if (missing.length > 0) {
  console.error("Missing Web routes:");
  for (const command of missing) {
    console.error(`${command.name}\t${command.method}\t${command.path}`);
  }
  failed = true;
}

if (methodMismatch.length > 0) {
  console.error("HTTP method mismatch (command method not served by route):");
  for (const { command, entry } of methodMismatch) {
    console.error(
      `${command.name}\t${command.method}\t${command.path}\t(route serves: ${[...entry.methods]
        .map((m) => m.toUpperCase())
        .join(",")})`,
    );
  }
  failed = true;
}

if (danglingReplacementPaths.length > 0) {
  console.error("webFetch literal paths with no matching Rust route:");
  for (const p of danglingReplacementPaths) {
    console.error(p);
  }
  failed = true;
}

if (process.argv.includes("--list-parity")) {
  if (webReplacements.length > 0) {
    console.error("Explicit web replacement commands:");
    for (const command of webReplacements) {
      console.error(`${command.name}\t${command.method}\t${command.path}`);
    }
  }
  if (parityExact.length > 0) {
    console.error("Explicit parity routes:");
    for (const command of parityExact) {
      console.error(`${command.name}\t${command.method}\t${command.path}`);
    }
  }
  if (parityFallback.length > 0) {
    console.error("Parity wildcard fallback routes:");
    for (const command of parityFallback) {
      console.error(`${command.name}\t${command.method}\t${command.path}`);
    }
  }
}

if (failed) {
  process.exit(1);
}

if (
  process.argv.includes("--fail-on-parity-fallback") &&
  parityFallback.length > 0
) {
  console.error(
    `Commands still covered only by parity wildcard: ${parityFallback.length}`,
  );
  process.exit(2);
}
