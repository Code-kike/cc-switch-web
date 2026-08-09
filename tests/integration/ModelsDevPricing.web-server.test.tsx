import fs from "node:fs/promises";
import { http, passthrough } from "msw";
import { afterAll, beforeAll, beforeEach, describe, expect, it } from "vitest";

import "@/lib/api/web-commands";
import { hermesProviderPresets } from "@/config/hermesProviderPresets";
import { openclawProviderPresets } from "@/config/openclawProviderPresets";
import { opencodeProviderPresets } from "@/config/opencodeProviderPresets";
import { usageApi } from "@/lib/api/usage";
import type { ModelPricing, ModelsDevSyncConfig } from "@/types/usage";
import { startTestWebServer, type TestWebServer } from "../helpers/web-server";
import { server } from "../msw/server";

type PricingTuple = {
  input: number;
  output: number;
  cacheRead: number;
  cacheCreation: number;
};

const S6A_SEED_SENTINELS: Readonly<Record<string, PricingTuple>> = {
  "claude-opus-5": {
    input: 5,
    output: 25,
    cacheRead: 0.5,
    cacheCreation: 6.25,
  },
  "qwen3.8-max": {
    input: 2,
    output: 6,
    cacheRead: 0.25,
    cacheCreation: 2.5,
  },
  "gpt-5.6-luna": {
    input: 0.2,
    output: 1.2,
    cacheRead: 0.02,
    cacheCreation: 0.25,
  },
  "gpt-5.6-terra": {
    input: 2,
    output: 12,
    cacheRead: 0.2,
    cacheCreation: 2.5,
  },
  "deepseek-chat": {
    input: 0.14,
    output: 0.28,
    cacheRead: 0.0028,
    cacheCreation: 0,
  },
  "deepseek-reasoner": {
    input: 0.14,
    output: 0.28,
    cacheRead: 0.0028,
    cacheCreation: 0,
  },
  "minimax-m3": {
    input: 0.3,
    output: 1.2,
    cacheRead: 0.06,
    cacheCreation: 0,
  },
};

const runtimeTuple = (pricing: ModelPricing): PricingTuple => ({
  input: Number(pricing.inputCostPerMillion),
  output: Number(pricing.outputCostPerMillion),
  cacheRead: Number(pricing.cacheReadCostPerMillion),
  cacheCreation: Number(pricing.cacheCreationCostPerMillion),
});

describe.sequential(
  "models.dev pricing parity against the real Web server",
  () => {
    let webServer: TestWebServer | undefined;

    beforeAll(async () => {
      // Keep server startup itself outside MSW, then relaunch MSW in strict mode.
      // Each test installs one origin-scoped passthrough below, so real traffic
      // never depends on the global onUnhandledRequest=warn fallback.
      server.close();
      try {
        webServer = await startTestWebServer();
        server.listen({ onUnhandledRequest: "error" });
      } catch (error) {
        server.listen({ onUnhandledRequest: "warn" });
        throw error;
      }
    }, 360_000);

    afterAll(async () => {
      try {
        await webServer?.stop();
      } finally {
        server.close();
        server.listen({ onUnhandledRequest: "warn" });
      }
    }, 20_000);

    beforeEach(() => {
      if (!webServer) throw new Error("test Web server was not started");
      server.use(http.all(`${webServer.baseUrl}/*`, () => passthrough()));
      Object.defineProperty(window, "__TAURI_INTERNALS__", {
        configurable: true,
        value: undefined,
      });
      Object.defineProperty(window, "__TAURI__", {
        configurable: true,
        value: undefined,
      });
      Object.defineProperty(window, "__CC_SWITCH_API_BASE__", {
        configurable: true,
        value: webServer.baseUrl,
      });
    });

    it("compares imported presets with the actual runtime-seeded pricing table", async () => {
      const pricing = await usageApi.getModelPricing();
      const runtimeById = new Map(
        pricing.map((entry) => [
          entry.modelId.toLowerCase(),
          runtimeTuple(entry),
        ]),
      );
      expect(runtimeById.size).toBeGreaterThan(0);

      // Compare every exact shared id. Deliberately do not emulate the lossy
      // Rust slash/dot fallback here: provider-scoped ids can have distinct
      // commercial terms and must not be mistaken for the bare seed row.
      const sharedEntries = openclawProviderPresets.flatMap((preset) =>
        (preset.settingsConfig.models ?? []).flatMap((model) => {
          if (!model.cost) return [];
          const modelId = model.id.toLowerCase();
          const seeded = runtimeById.get(modelId);
          if (!seeded) return [];
          return [{ preset: preset.name, modelId, cost: model.cost, seeded }];
        }),
      );
      expect(sharedEntries.length).toBeGreaterThan(0);

      const sharedIds = new Set(sharedEntries.map((entry) => entry.modelId));
      for (const sentinel of [
        "claude-opus-5",
        "qwen3.8-max",
        "kat-coder-pro",
        "minimax-m2.7",
        "kimi-k3",
      ]) {
        expect(sharedIds.has(sentinel), `${sentinel} must remain shared`).toBe(
          true,
        );
      }

      for (const { preset, modelId, cost, seeded } of sharedEntries) {
        const label = `${preset}/${modelId}`;
        expect(
          cost.input,
          `${label} preset input must be non-zero`,
        ).toBeGreaterThan(0);
        expect(
          cost.output,
          `${label} preset output must be non-zero`,
        ).toBeGreaterThan(0);
        expect(
          seeded.input,
          `${label} seed input must be non-zero`,
        ).toBeGreaterThan(0);
        expect(
          seeded.output,
          `${label} seed output must be non-zero`,
        ).toBeGreaterThan(0);
        expect(
          { input: seeded.input, output: seeded.output },
          `${label} preset/runtime seed mismatch`,
        ).toEqual({ input: cost.input, output: cost.output });
      }

      for (const [modelId, expected] of Object.entries(S6A_SEED_SENTINELS)) {
        const actual = runtimeById.get(modelId);
        expect(actual, `${modelId} runtime seed missing`).toBeDefined();
        expect(actual, `${modelId} runtime seed stale`).toEqual(expected);
        expect(
          actual!.input,
          `${modelId} input must be non-zero`,
        ).toBeGreaterThan(0);
        expect(
          actual!.output,
          `${modelId} output must be non-zero`,
        ).toBeGreaterThan(0);
        expect(Object.values(actual!).every(Number.isFinite)).toBe(true);
      }
    });

    it("keeps Qwen3.8 Max in the intended Bailian/Qwen preset owners", () => {
      const modelId = "qwen3.8-max";
      expect(
        openclawProviderPresets
          .filter((preset) =>
            (preset.settingsConfig.models ?? []).some(
              (model) => model.id === modelId,
            ),
          )
          .map((preset) => preset.name),
      ).toEqual(["Qwen Coder"]);
      expect(
        opencodeProviderPresets
          .filter((preset) => modelId in (preset.settingsConfig.models ?? {}))
          .map((preset) => preset.name),
      ).toEqual(["Bailian"]);
      expect(
        hermesProviderPresets
          .filter((preset) =>
            preset.settingsConfig.models?.some((model) => model.id === modelId),
          )
          .map((preset) => preset.name),
      ).toEqual(["Bailian", "Bailian For Coding"]);
    });

    it("round-trips config, batch/single overrides, outcomes, and tombstones through shared services", async () => {
      const initial = await usageApi.getModelsDevSyncConfig();
      expect(initial.config).toMatchObject({
        autoSyncEnabled: false,
        includeCommonModels: true,
        selectedModelKeys: [],
        excludedCommonModelKeys: [],
        lastSyncAt: null,
        lastSyncError: null,
      });

      const config: ModelsDevSyncConfig = {
        autoSyncEnabled: true,
        includeCommonModels: false,
        selectedModelKeys: [" relay/custom-model ", "relay/custom-model"],
        excludedCommonModelKeys: [" openai/gpt-5 "],
        lastSyncAt: null,
        lastSyncError: null,
      };
      await usageApi.saveModelsDevSyncConfig(config);

      const normalized = await usageApi.getModelsDevSyncConfig();
      expect(normalized.config).toMatchObject({
        autoSyncEnabled: true,
        includeCommonModels: false,
        selectedModelKeys: ["relay/custom-model"],
        excludedCommonModelKeys: ["openai/gpt-5"],
      });

      const pricing: ModelPricing = {
        modelId: "web-parity-custom-model",
        displayName: "Web Parity Custom Model",
        inputCostPerMillion: "1.25",
        outputCostPerMillion: "5",
        cacheReadCostPerMillion: "0.1",
        cacheCreationCostPerMillion: "1.5",
      };
      await expect(usageApi.updateModelPricingBatch([pricing])).resolves.toBe(
        1,
      );
      expect(await usageApi.getModelPricing()).toContainEqual(pricing);

      await usageApi.updateModelPricing(
        pricing.modelId,
        pricing.displayName,
        "2.5",
        pricing.outputCostPerMillion,
        pricing.cacheReadCostPerMillion,
        pricing.cacheCreationCostPerMillion,
      );
      expect(await usageApi.getModelPricing()).toContainEqual({
        ...pricing,
        inputCostPerMillion: "2.5",
      });

      await usageApi.recordModelsDevSyncResult(123456, " offline ");
      const outcome = await usageApi.getModelsDevSyncConfig();
      expect(outcome.config).toMatchObject({
        autoSyncEnabled: true,
        selectedModelKeys: ["relay/custom-model"],
        lastSyncAt: 123456,
        lastSyncError: "offline",
      });

      await usageApi.deleteModelPricing(pricing.modelId);
      expect(
        (await usageApi.getModelPricing()).some(
          (entry) => entry.modelId === pricing.modelId,
        ),
      ).toBe(false);

      const localFile = JSON.parse(
        await fs.readFile(outcome.configPath, "utf8"),
      ) as {
        models: Array<{ modelId: string }>;
        deletedModelIds: string[];
      };
      expect(localFile.models).not.toContainEqual(
        expect.objectContaining({ modelId: pricing.modelId }),
      );
      expect(localFile.deletedModelIds).toContain(pricing.modelId);
    });
  },
);
