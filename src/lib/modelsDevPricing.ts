import type { ModelPricing, ModelsDevSyncConfig } from "@/types/usage";

export const MODELS_DEV_API_URL = "https://models.dev/api.json";
const MODELS_DEV_FETCH_TIMEOUT_MS = 15_000;
const MODELS_DEV_MAX_RESPONSE_BYTES = 16 * 1024 * 1024;
const MODELS_DEV_MAX_PROVIDERS = 512;
const MODELS_DEV_MAX_MODELS_PER_PROVIDER = 5_000;
const MODELS_DEV_MAX_MODELS_TOTAL = 20_000;
const MODELS_DEV_MAX_ID_LENGTH = 256;
const MODELS_DEV_MAX_TEXT_LENGTH = 512;
const MODELS_DEV_MAX_MODALITIES = 32;
const MODELS_DEV_MAX_JSON_DEPTH = 8;
const MODELS_DEV_MAX_JSON_CONTAINERS = 100_000;
const MODELS_DEV_MAX_PRICE = 1_000_000_000_000;

export interface ModelsDevCost {
  input?: number;
  output?: number;
  cache_read?: number;
  cache_write?: number;
}

export interface ModelsDevModalities {
  input?: string[];
  output?: string[];
}

export interface ModelsDevModel {
  id?: string;
  name?: string;
  release_date?: string;
  cost?: ModelsDevCost;
  modalities?: ModelsDevModalities;
  status?: string;
}

export interface ModelsDevProvider {
  id?: string;
  name?: string;
  models?: Record<string, ModelsDevModel>;
}

export type ModelsDevResponse = Record<string, ModelsDevProvider>;

export interface ModelsDevEntry {
  key: string;
  providerId: string;
  providerName: string;
  modelId: string;
  normalizedId: string;
  modelName: string;
  releaseDate: string;
  input: number;
  output: number;
  cacheRead: number;
  cacheWrite: number;
}

const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === "object" && value !== null && !Array.isArray(value);

function assertBoundedJsonDepth(value: unknown): void {
  type Frame = { value: object; depth: number; exiting: boolean };
  const pending: Frame[] = [];
  if (typeof value === "object" && value !== null) {
    pending.push({ value, depth: 0, exiting: false });
  }

  // JSON.parse always produces a tree, but this validator is exported and can
  // receive arbitrary object graphs in tests/callers. Track the active DFS path
  // to reject genuine cycles while allowing repeated, non-cyclic references.
  const active = new WeakSet<object>();
  const validated = new WeakSet<object>();
  let containerCount = 0;

  while (pending.length > 0) {
    const current = pending.pop()!;
    if (current.exiting) {
      active.delete(current.value);
      validated.add(current.value);
      continue;
    }
    if (current.depth > MODELS_DEV_MAX_JSON_DEPTH) {
      throw new Error("Invalid models.dev response: nesting is too deep");
    }
    if (active.has(current.value)) {
      throw new Error("Invalid models.dev response: cyclic structure");
    }
    if (validated.has(current.value)) continue;

    containerCount += 1;
    if (containerCount > MODELS_DEV_MAX_JSON_CONTAINERS) {
      throw new Error("Invalid models.dev response: too many JSON containers");
    }

    active.add(current.value);
    pending.push({ ...current, exiting: true });
    const children = Array.isArray(current.value)
      ? current.value
      : Object.values(current.value);
    for (let index = children.length - 1; index >= 0; index -= 1) {
      const child = children[index];
      if (typeof child === "object" && child !== null) {
        pending.push({
          value: child,
          depth: current.depth + 1,
          exiting: false,
        });
      }
    }
  }
}

function assertBoundedId(value: string, label: string): void {
  if (!value || value.length > MODELS_DEV_MAX_ID_LENGTH) {
    throw new Error(`Invalid models.dev ${label}`);
  }
}

function optionalString(
  record: Record<string, unknown>,
  key: string,
  label: string,
): string | undefined {
  const value = record[key];
  if (value === undefined) return undefined;
  if (typeof value !== "string" || value.length > MODELS_DEV_MAX_TEXT_LENGTH) {
    throw new Error(`Invalid models.dev ${label}`);
  }
  return value;
}

function optionalPrice(
  record: Record<string, unknown>,
  key: string,
  label: string,
): number | undefined {
  const value = record[key];
  if (value === undefined) return undefined;
  if (
    typeof value !== "number" ||
    !Number.isFinite(value) ||
    value < 0 ||
    value >= MODELS_DEV_MAX_PRICE
  ) {
    throw new Error(`Invalid models.dev ${label}`);
  }
  return value;
}

function optionalModalities(
  record: Record<string, unknown>,
  key: string,
  label: string,
): string[] | undefined {
  const value = record[key];
  if (value === undefined) return undefined;
  if (!Array.isArray(value) || value.length > MODELS_DEV_MAX_MODALITIES) {
    throw new Error(`Invalid models.dev ${label}`);
  }
  return value.map((entry) => {
    if (
      typeof entry !== "string" ||
      entry.length > MODELS_DEV_MAX_TEXT_LENGTH
    ) {
      throw new Error(`Invalid models.dev ${label}`);
    }
    return entry;
  });
}

/**
 * Validate and copy the untrusted models.dev document into the small canonical
 * shape consumed by pricing selection. Bounds keep a compromised upstream from
 * turning renderer startup into an unbounded parse/iteration workload.
 */
export function parseModelsDevResponse(value: unknown): ModelsDevResponse {
  assertBoundedJsonDepth(value);
  if (!isRecord(value)) {
    throw new Error("Invalid models.dev response: expected a provider object");
  }

  const providerEntries = Object.entries(value);
  if (providerEntries.length > MODELS_DEV_MAX_PROVIDERS) {
    throw new Error("Invalid models.dev response: too many providers");
  }

  const parsed: ModelsDevResponse = Object.create(null) as ModelsDevResponse;
  let modelCount = 0;
  for (const [providerId, providerValue] of providerEntries) {
    assertBoundedId(providerId, "provider ID");
    if (!isRecord(providerValue)) {
      throw new Error(`Invalid models.dev provider: ${providerId}`);
    }

    const modelsValue = providerValue.models;
    if (modelsValue !== undefined && !isRecord(modelsValue)) {
      throw new Error(`Invalid models.dev model catalog: ${providerId}`);
    }
    const modelsRecord = modelsValue ?? {};
    const modelEntries = Object.entries(modelsRecord);
    if (modelEntries.length > MODELS_DEV_MAX_MODELS_PER_PROVIDER) {
      throw new Error(`Invalid models.dev model catalog size: ${providerId}`);
    }
    modelCount += modelEntries.length;
    if (modelCount > MODELS_DEV_MAX_MODELS_TOTAL) {
      throw new Error("Invalid models.dev response: too many models");
    }

    const models: Record<string, ModelsDevModel> = Object.create(
      null,
    ) as Record<string, ModelsDevModel>;
    for (const [modelId, modelValue] of modelEntries) {
      assertBoundedId(modelId, "model ID");
      if (!isRecord(modelValue)) {
        throw new Error(`Invalid models.dev model: ${providerId}/${modelId}`);
      }

      const costValue = modelValue.cost;
      if (costValue !== undefined && !isRecord(costValue)) {
        throw new Error(`Invalid models.dev cost: ${providerId}/${modelId}`);
      }
      const costRecord = costValue ?? {};
      const cost: ModelsDevCost = {
        input: optionalPrice(costRecord, "input", "input price"),
        output: optionalPrice(costRecord, "output", "output price"),
        cache_read: optionalPrice(costRecord, "cache_read", "cache read price"),
        cache_write: optionalPrice(
          costRecord,
          "cache_write",
          "cache write price",
        ),
      };

      const modalitiesValue = modelValue.modalities;
      if (modalitiesValue !== undefined && !isRecord(modalitiesValue)) {
        throw new Error(
          `Invalid models.dev modalities: ${providerId}/${modelId}`,
        );
      }
      const modalitiesRecord = modalitiesValue ?? {};
      const modalities: ModelsDevModalities = {
        input: optionalModalities(
          modalitiesRecord,
          "input",
          "input modalities",
        ),
        output: optionalModalities(
          modalitiesRecord,
          "output",
          "output modalities",
        ),
      };

      models[modelId] = {
        id: optionalString(modelValue, "id", "model id field"),
        name: optionalString(modelValue, "name", "model name"),
        release_date: optionalString(
          modelValue,
          "release_date",
          "release date",
        ),
        status: optionalString(modelValue, "status", "model status"),
        cost,
        modalities,
      };
    }

    parsed[providerId] = {
      id: optionalString(providerValue, "id", "provider id field"),
      name: optionalString(providerValue, "name", "provider name"),
      models,
    };
  }
  return parsed;
}

async function readBoundedResponseText(response: Response): Promise<string> {
  const contentLength = response.headers.get("content-length");
  if (contentLength !== null) {
    const declaredLength = Number(contentLength);
    if (
      Number.isFinite(declaredLength) &&
      declaredLength > MODELS_DEV_MAX_RESPONSE_BYTES
    ) {
      throw new Error("models.dev response exceeds the size limit");
    }
  }

  if (!response.body) {
    const text = await response.text();
    if (
      new TextEncoder().encode(text).byteLength > MODELS_DEV_MAX_RESPONSE_BYTES
    ) {
      throw new Error("models.dev response exceeds the size limit");
    }
    return text;
  }

  const reader = response.body.getReader();
  const chunks: Uint8Array[] = [];
  let length = 0;
  while (true) {
    const { done, value: chunk } = await reader.read();
    if (done) break;
    length += chunk.byteLength;
    if (length > MODELS_DEV_MAX_RESPONSE_BYTES) {
      await reader.cancel();
      throw new Error("models.dev response exceeds the size limit");
    }
    chunks.push(chunk);
  }

  const bytes = new Uint8Array(length);
  let offset = 0;
  for (const chunk of chunks) {
    bytes.set(chunk, offset);
    offset += chunk.byteLength;
  }
  return new TextDecoder().decode(bytes);
}

const NON_TEXT_MODEL_MARKERS = [
  "audio",
  "deprecated",
  "embedding",
  "image",
  "moderation",
  "realtime",
  "transcribe",
  "tts",
  "video",
];
const NON_TEXT_OUTPUT_MODALITIES = new Set(["audio", "image", "video"]);

const isTextPricingModel = (modelId: string, model?: ModelsDevModel) => {
  if (model?.status?.toLowerCase() === "deprecated") return false;

  const outputModalities = model?.modalities?.output
    ?.filter((modality): modality is string => typeof modality === "string")
    .map((modality) => modality.toLowerCase());
  if (
    outputModalities?.length &&
    (!outputModalities.includes("text") ||
      outputModalities.some((modality) =>
        NON_TEXT_OUTPUT_MODALITIES.has(modality),
      ))
  ) {
    return false;
  }

  const searchableName = `${modelId} ${model?.name ?? ""}`.toLowerCase();
  return !NON_TEXT_MODEL_MARKERS.some((marker) =>
    searchableName.includes(marker),
  );
};

export function normalizeModelIdForPricing(modelId: string): string {
  const afterSlash = modelId.slice(modelId.lastIndexOf("/") + 1);
  const beforeColon = afterSlash.split(":")[0] ?? "";
  let normalized = beforeColon.trim().replace(/@/g, "-").toLowerCase();
  if (normalized.endsWith("[1m]")) {
    normalized = normalized.slice(0, -"[1m]".length).trim();
  }
  return normalized;
}

export function formatPrice(value: number): string {
  if (!Number.isFinite(value) || value <= 0) return "0";
  if (value >= 1e12) return "0";
  const trimmed = value.toFixed(6).replace(/0+$/, "").replace(/\.$/, "");
  return trimmed || "0";
}

export function flattenModels(data: ModelsDevResponse): ModelsDevEntry[] {
  const entries: ModelsDevEntry[] = [];
  for (const [providerId, provider] of Object.entries(data)) {
    if (!provider || typeof provider !== "object") continue;
    const providerName = provider.name || providerId;
    for (const [modelId, model] of Object.entries(provider.models ?? {})) {
      if (!isTextPricingModel(modelId, model)) continue;
      const cost = model?.cost;
      const input = typeof cost?.input === "number" ? cost.input : null;
      const output = typeof cost?.output === "number" ? cost.output : null;
      if (input === null && output === null) continue;
      const normalizedId = normalizeModelIdForPricing(modelId);
      if (!normalizedId) continue;
      entries.push({
        key: `${providerId}/${modelId}`,
        providerId,
        providerName,
        modelId,
        normalizedId,
        modelName: model?.name || modelId,
        releaseDate:
          typeof model?.release_date === "string" ? model.release_date : "",
        input: input ?? 0,
        output: output ?? 0,
        cacheRead: typeof cost?.cache_read === "number" ? cost.cache_read : 0,
        cacheWrite:
          typeof cost?.cache_write === "number" ? cost.cache_write : 0,
      });
    }
  }
  entries.sort(
    (a, b) =>
      b.releaseDate.localeCompare(a.releaseDate) ||
      a.modelName.localeCompare(b.modelName),
  );
  return entries;
}

export async function fetchModelsDevPricing(): Promise<ModelsDevResponse> {
  const controller = new AbortController();
  const timeout = window.setTimeout(
    () => controller.abort(),
    MODELS_DEV_FETCH_TIMEOUT_MS,
  );
  try {
    // Compile-time fixed public catalog. Do not accept a caller-supplied URL:
    // Web mode is unauthenticated and must never become an arbitrary fetch proxy.
    const response = await fetch(MODELS_DEV_API_URL, {
      signal: controller.signal,
    });
    if (!response.ok) {
      throw new Error(`HTTP ${response.status}`);
    }
    const contentType = response.headers.get("content-type");
    if (
      contentType &&
      !contentType.toLowerCase().includes("application/json")
    ) {
      throw new Error("Invalid models.dev response content type");
    }
    const text = await readBoundedResponseText(response);
    let value: unknown;
    try {
      value = JSON.parse(text) as unknown;
    } catch {
      throw new Error("Invalid models.dev JSON response");
    }
    return parseModelsDevResponse(value);
  } finally {
    window.clearTimeout(timeout);
  }
}

const COMMON_MODEL_LIMIT_PER_FAMILY = 6;

interface CommonFamilyRule {
  id: string;
  providers: ReadonlySet<string>;
  matches: (modelId: string) => boolean;
}

const COMMON_FAMILY_RULES: CommonFamilyRule[] = [
  {
    id: "claude",
    providers: new Set(["anthropic"]),
    matches: (modelId) => modelId.startsWith("claude-"),
  },
  {
    id: "gpt",
    providers: new Set(["openai"]),
    matches: (modelId) =>
      modelId.startsWith("gpt-") ||
      modelId.startsWith("o1-") ||
      modelId.startsWith("o3-") ||
      modelId.startsWith("o4-"),
  },
  {
    id: "gemini",
    providers: new Set(["google"]),
    matches: (modelId) => modelId.startsWith("gemini-"),
  },
  {
    id: "grok",
    providers: new Set(["xai"]),
    matches: (modelId) => modelId.startsWith("grok-"),
  },
  {
    id: "deepseek",
    providers: new Set(["deepseek"]),
    matches: (modelId) => modelId.startsWith("deepseek-"),
  },
  {
    id: "qwen",
    providers: new Set(["alibaba"]),
    matches: (modelId) => modelId.startsWith("qwen"),
  },
  {
    id: "mimo",
    providers: new Set(["xiaomi"]),
    matches: (modelId) => modelId.startsWith("mimo-"),
  },
  {
    id: "longcat",
    providers: new Set(["longcat"]),
    matches: (modelId) => modelId.startsWith("longcat-"),
  },
  {
    id: "kimi",
    providers: new Set(["moonshotai"]),
    matches: (modelId) => modelId.startsWith("kimi-"),
  },
  {
    id: "minimax",
    providers: new Set(["minimax-cn"]),
    matches: (modelId) => modelId.startsWith("minimax-m"),
  },
  {
    id: "glm",
    providers: new Set(["zai"]),
    matches: (modelId) => modelId.startsWith("glm-"),
  },
];

/** Pick a bounded, canonical set of recent chat/coding models per family. */
export function getCommonModelKeys(entries: ModelsDevEntry[]): Set<string> {
  const keys = new Set<string>();
  for (const rule of COMMON_FAMILY_RULES) {
    let count = 0;
    for (const entry of entries) {
      if (
        rule.providers.has(entry.providerId) &&
        rule.matches(entry.modelId.toLowerCase())
      ) {
        keys.add(entry.key);
        count += 1;
        if (count >= COMMON_MODEL_LIMIT_PER_FAMILY) break;
      }
    }
  }
  return keys;
}

export function resolveModelsDevSelection(
  entries: ModelsDevEntry[],
  config: ModelsDevSyncConfig,
): ModelsDevEntry[] {
  const explicit = new Set(config.selectedModelKeys);
  const excluded = new Set(config.excludedCommonModelKeys);
  const common = config.includeCommonModels
    ? getCommonModelKeys(entries)
    : new Set<string>();
  return entries.filter(
    (entry) =>
      explicit.has(entry.key) ||
      (common.has(entry.key) && !excluded.has(entry.key)),
  );
}

export function toModelPricing(entries: ModelsDevEntry[]): ModelPricing[] {
  const byModelId = new Map<string, ModelPricing>();
  for (const entry of entries) {
    if (byModelId.has(entry.normalizedId)) continue;
    byModelId.set(entry.normalizedId, {
      modelId: entry.normalizedId,
      displayName: entry.modelName,
      inputCostPerMillion: formatPrice(entry.input),
      outputCostPerMillion: formatPrice(entry.output),
      cacheReadCostPerMillion: formatPrice(entry.cacheRead),
      cacheCreationCostPerMillion: formatPrice(entry.cacheWrite),
    });
  }
  return Array.from(byModelId.values());
}
