#!/usr/bin/env node
import fs from "node:fs"
import path from "node:path"

const root = process.cwd()
const webCommandsPath = path.join(root, "src/lib/api/web-commands.ts")
const handlersDir = path.join(root, "src-tauri/src/web_api/handlers")

const webCommands = fs.readFileSync(webCommandsPath, "utf8")
const commandRe =
  /^\s*([A-Za-z0-9_]+): \{ method: "([A-Z]+)", path: "([^"]+)"([^}]*)\}/gm

const commands = []
let match
while ((match = commandRe.exec(webCommands)) !== null) {
  commands.push({
    name: match[1],
    method: match[2],
    path: match[3],
    unsupported: match[4].includes("unsupported"),
    webReplacement: match[4].includes("webReplacement"),
  })
}

const routes = new Map()
const wildcardPrefixes = []
const routeRe = /\.route\s*\(\s*"([^"]+)"/g
for (const file of fs.readdirSync(handlersDir)) {
  if (!file.endsWith(".rs")) continue
  const source = fs.readFileSync(path.join(handlersDir, file), "utf8")
  let routeMatch
  while ((routeMatch = routeRe.exec(source)) !== null) {
    const route = `/api${routeMatch[1]}`
    if (route.endsWith("/*path")) {
      wildcardPrefixes.push({
        prefix: route.slice(0, -"*path".length),
        file,
      })
    } else {
      routes.set(route, file)
    }
  }
}

const missing = commands.filter(
  (command) =>
    !command.unsupported &&
    !command.webReplacement &&
    !routes.has(command.path) &&
    !wildcardPrefixes.some((route) => command.path.startsWith(route.prefix)),
)

const parityFallback = commands.filter(
  (command) =>
    !command.unsupported &&
    !command.webReplacement &&
    !routes.has(command.path) &&
    wildcardPrefixes.some(
      (route) =>
        route.file === "parity.rs" && command.path.startsWith(route.prefix),
    ),
)

const parityExact = commands.filter(
  (command) =>
    !command.unsupported &&
    !command.webReplacement &&
    routes.get(command.path) === "parity.rs",
)

const webReplacements = commands.filter((command) => command.webReplacement)

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
)

if (missing.length > 0) {
  console.error("Missing Web routes:")
  for (const command of missing) {
    console.error(`${command.name}\t${command.method}\t${command.path}`)
  }
  process.exit(1)
}

if (process.argv.includes("--list-parity")) {
  if (webReplacements.length > 0) {
    console.error("Explicit web replacement commands:")
    for (const command of webReplacements) {
      console.error(`${command.name}\t${command.method}\t${command.path}`)
    }
  }
  if (parityExact.length > 0) {
    console.error("Explicit parity routes:")
    for (const command of parityExact) {
      console.error(`${command.name}\t${command.method}\t${command.path}`)
    }
  }
  if (parityFallback.length > 0) {
    console.error("Parity wildcard fallback routes:")
    for (const command of parityFallback) {
      console.error(`${command.name}\t${command.method}\t${command.path}`)
    }
  }
}

if (process.argv.includes("--fail-on-parity-fallback") && parityFallback.length > 0) {
  console.error(`Commands still covered only by parity wildcard: ${parityFallback.length}`)
  process.exit(2)
}
