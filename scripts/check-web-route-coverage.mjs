#!/usr/bin/env node
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

const routes = new Map();
const wildcardPrefixes = [];
const routeRe = /\.route\s*\(\s*"([^"]+)"/g;
for (const file of fs.readdirSync(handlersDir)) {
  if (!file.endsWith(".rs")) continue;
  const source = fs.readFileSync(path.join(handlersDir, file), "utf8");
  let routeMatch;
  while ((routeMatch = routeRe.exec(source)) !== null) {
    const route = `/api${routeMatch[1]}`;
    if (route.endsWith("/*path")) {
      wildcardPrefixes.push({
        prefix: route.slice(0, -"*path".length),
        file,
      });
    } else {
      routes.set(route, file);
    }
  }
}

const missing = commands.filter(
  (command) =>
    !command.unsupported &&
    !command.webReplacement &&
    !routes.has(command.path) &&
    !wildcardPrefixes.some((route) => command.path.startsWith(route.prefix)),
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

const parityExact = commands.filter(
  (command) =>
    !command.unsupported &&
    !command.webReplacement &&
    routes.get(command.path) === "parity.rs",
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
      missing: missing.length,
      parityExact: parityExact.length,
      parityFallback: parityFallback.length,
    },
    null,
    2,
  ),
);

if (missing.length > 0) {
  console.error("Missing Web routes:");
  for (const command of missing) {
    console.error(`${command.name}\t${command.method}\t${command.path}`);
  }
  process.exit(1);
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

if (
  process.argv.includes("--fail-on-parity-fallback") &&
  parityFallback.length > 0
) {
  console.error(
    `Commands still covered only by parity wildcard: ${parityFallback.length}`,
  );
  process.exit(2);
}
