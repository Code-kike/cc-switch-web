export type HttpMethod = "GET" | "POST" | "PUT" | "PATCH" | "DELETE"

export type CommandSpec = {
  method: HttpMethod
  path: string
  pathParams?: readonly string[]
  queryParams?: boolean
  bodyParams?: boolean
  unsupported?: true
  webReplacement?: true
}

export type CommandMap = Record<string, CommandSpec>

export function defineCommands<const T extends CommandMap>(map: T): T {
  return map
}
