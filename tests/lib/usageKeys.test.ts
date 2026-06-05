import { describe, expect, it } from "vitest";
import {
  usageKeys,
  isUsageLogDerivedKey,
  USAGE_LOG_DERIVED_SECTIONS,
} from "@/lib/query/usage";

describe("usageKeys.script (M39 SSOT)", () => {
  it("produces the exact array the hand-rolled call sites used", () => {
    // The migrated literals were ["usage", id, app]; the factory must keep
    // that shape so writers (UsageScriptModal / useProviderActions / useHermes)
    // and readers (queries.ts / useUsageCacheBridge) never desync.
    expect(usageKeys.script("provider-1", "claude")).toEqual([
      "usage",
      "provider-1",
      "claude",
    ]);
    expect(usageKeys.script("hermes-a", "hermes")).toEqual([
      "usage",
      "hermes-a",
      "hermes",
    ]);
  });
});

describe("isUsageLogDerivedKey (M38)", () => {
  it("matches every log-derived dashboard aggregate section", () => {
    const aggregateKeys = [
      usageKeys.summary("today", undefined, undefined, "all"),
      usageKeys.summaryByApp("today", undefined, undefined),
      usageKeys.trends("today", undefined, undefined, "all"),
      usageKeys.providerStats("today", undefined, undefined, "all"),
      usageKeys.modelStats("today", undefined, undefined, "all"),
      usageKeys.logs({ preset: "today" }, 0, 20),
    ];
    for (const key of aggregateKeys) {
      expect(isUsageLogDerivedKey(key)).toBe(true);
    }
  });

  it("does NOT match per-provider script queries (external billing data)", () => {
    expect(isUsageLogDerivedKey(usageKeys.script("provider-1", "claude"))).toBe(
      false,
    );
    expect(isUsageLogDerivedKey(usageKeys.script("hermes-a", "hermes"))).toBe(
      false,
    );
  });

  it("does NOT match pricing / limits / detail (not log-derived)", () => {
    expect(isUsageLogDerivedKey(usageKeys.pricing())).toBe(false);
    expect(isUsageLogDerivedKey(usageKeys.limits("p1", "claude"))).toBe(false);
    expect(isUsageLogDerivedKey(usageKeys.detail("req-1"))).toBe(false);
  });

  it("does NOT match keys outside the usage namespace", () => {
    expect(isUsageLogDerivedKey(["proxyStatus"])).toBe(false);
    expect(isUsageLogDerivedKey(["providers", "claude"])).toBe(false);
    expect(isUsageLogDerivedKey([])).toBe(false);
  });

  it("section list pins the discriminators the predicate accepts", () => {
    expect([...USAGE_LOG_DERIVED_SECTIONS]).toEqual([
      "summary",
      "summary-by-app",
      "trends",
      "provider-stats",
      "model-stats",
      "logs",
    ]);
  });
});
