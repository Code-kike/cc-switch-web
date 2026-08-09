import { beforeEach, describe, expect, it, vi } from "vitest";

import {
  fetchModelsDevPricing,
  MODELS_DEV_API_URL,
  parseModelsDevResponse,
} from "@/lib/modelsDevPricing";

const validCatalog = {
  alibaba: {
    id: "alibaba",
    name: "Alibaba",
    models: {
      "qwen3.8-max": {
        id: "qwen3.8-max",
        name: "Qwen3.8 Max",
        release_date: "2026-08-03",
        modalities: { input: ["text", "image"], output: ["text"] },
        cost: {
          input: 2,
          output: 6,
          cache_read: 0.25,
          cache_write: 2.5,
        },
      },
    },
  },
};

const responseFor = (
  body: BodyInit,
  headers?: Record<string, string>,
): Response =>
  new Response(body, {
    headers: { "Content-Type": "application/json", ...headers },
  });

describe("models.dev response security", () => {
  beforeEach(() => vi.restoreAllMocks());

  it("fetches only the compile-time fixed catalog URL and validates the result", async () => {
    const fetchMock = vi
      .spyOn(globalThis, "fetch")
      .mockResolvedValueOnce(responseFor(JSON.stringify(validCatalog)));

    await expect(fetchModelsDevPricing()).resolves.toMatchObject(validCatalog);
    expect(fetchMock).toHaveBeenCalledTimes(1);
    expect(fetchMock).toHaveBeenCalledWith(MODELS_DEV_API_URL, {
      signal: expect.any(AbortSignal),
    });
    expect(MODELS_DEV_API_URL).toBe("https://models.dev/api.json");
  });

  it("rejects malformed catalogs", () => {
    expect(() => parseModelsDevResponse([])).toThrow(
      "expected a provider object",
    );
    expect(() => parseModelsDevResponse({ alibaba: { models: [] } })).toThrow(
      "Invalid models.dev model catalog",
    );
    expect(() => parseModelsDevResponse({ alibaba: { models: null } })).toThrow(
      "Invalid models.dev model catalog",
    );
  });

  it.each([Number.NaN, Number.POSITIVE_INFINITY, -1, 1_000_000_000_000])(
    "rejects unsafe numeric price %s for every cost field",
    (price) => {
      for (const key of ["input", "output", "cache_read", "cache_write"]) {
        expect(() =>
          parseModelsDevResponse({
            alibaba: {
              models: {
                "qwen3.8-max": { cost: { [key]: price } },
              },
            },
          }),
        ).toThrow("Invalid models.dev");
      }
    },
  );

  it("rejects excessive nesting and cyclic object graphs without recursion", () => {
    const tooDeep: Record<string, unknown> = {};
    let cursor = tooDeep;
    for (let depth = 0; depth < 12; depth += 1) {
      const child: Record<string, unknown> = {};
      cursor.child = child;
      cursor = child;
    }
    expect(() => parseModelsDevResponse(tooDeep)).toThrow(
      "nesting is too deep",
    );

    const cyclic: Record<string, unknown> = {
      alibaba: { models: {} },
    };
    (cyclic.alibaba as Record<string, unknown>).cycle = cyclic;
    expect(() => parseModelsDevResponse(cyclic)).toThrow("cyclic structure");
  });

  it("allows a repeated non-cyclic value while copying it canonically", () => {
    const sharedCost = { input: 1, output: 2 };
    const parsed = parseModelsDevResponse({
      relay: {
        models: {
          first: { cost: sharedCost },
          second: { cost: sharedCost },
        },
      },
    });

    expect(parsed.relay.models?.first.cost).toEqual(sharedCost);
    expect(parsed.relay.models?.second.cost).toEqual(sharedCost);
    expect(parsed.relay.models?.first.cost).not.toBe(sharedCost);
  });

  it("rejects excessive provider and per-provider model counts", () => {
    const providers = Object.fromEntries(
      Array.from({ length: 513 }, (_, index) => [
        `provider-${index}`,
        { models: {} },
      ]),
    );
    expect(() => parseModelsDevResponse(providers)).toThrow(
      "too many providers",
    );

    const models = Object.fromEntries(
      Array.from({ length: 5_001 }, (_, index) => [
        `model-${index}`,
        { cost: { input: 1, output: 2 } },
      ]),
    );
    expect(() => parseModelsDevResponse({ relay: { models } })).toThrow(
      "model catalog size",
    );
  });

  it("rejects a response declared larger than the bounded catalog limit", async () => {
    vi.spyOn(globalThis, "fetch").mockResolvedValueOnce(
      responseFor(JSON.stringify(validCatalog), {
        "Content-Length": String(17 * 1024 * 1024),
      }),
    );

    await expect(fetchModelsDevPricing()).rejects.toThrow(
      "models.dev response exceeds the size limit",
    );
  });

  it("rejects an oversized streamed body even without Content-Length", async () => {
    const stream = new ReadableStream<Uint8Array>({
      start(controller) {
        controller.enqueue(new Uint8Array(9 * 1024 * 1024));
        controller.enqueue(new Uint8Array(8 * 1024 * 1024 + 1));
        controller.close();
      },
    });
    vi.spyOn(globalThis, "fetch").mockResolvedValueOnce(responseFor(stream));

    await expect(fetchModelsDevPricing()).rejects.toThrow(
      "models.dev response exceeds the size limit",
    );
  });

  it("rejects non-JSON response content without retrying another URL", async () => {
    const fetchMock = vi
      .spyOn(globalThis, "fetch")
      .mockResolvedValueOnce(
        new Response("not json", { headers: { "Content-Type": "text/plain" } }),
      );

    await expect(fetchModelsDevPricing()).rejects.toThrow(
      "Invalid models.dev response content type",
    );
    expect(fetchMock).toHaveBeenCalledTimes(1);
    expect(fetchMock.mock.calls[0]?.[0]).toBe(MODELS_DEV_API_URL);
  });
});
