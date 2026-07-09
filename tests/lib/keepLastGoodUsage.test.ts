import { describe, expect, it } from "vitest";
import {
  KEEP_LAST_GOOD_MS,
  isTransientUsageError,
  resolveDisplayUsage,
  type LastGoodUsage,
} from "@/lib/query/queries";
import type { UsageResult } from "@/types";

const T0 = 1_700_000_000_000;

const ok = (used: number): UsageResult => ({
  success: true,
  data: [{ planName: "quota", used }],
});

const fail = (error: string): UsageResult => ({
  success: false,
  error,
});

describe("isTransientUsageError", () => {
  it("classifies 5xx and 429 as transient, but keeps auth 4xx deterministic", () => {
    expect(isTransientUsageError(fail("API error (HTTP 500): oops"))).toBe(
      true,
    );
    expect(isTransientUsageError(fail("HTTP 429 Too Many Requests"))).toBe(
      true,
    );
    expect(
      isTransientUsageError(fail("Authentication failed (HTTP 401)")),
    ).toBe(false);
    expect(
      isTransientUsageError(fail("Authentication failed (HTTP 403)")),
    ).toBe(false);
  });

  it("classifies network and response-read failures as transient", () => {
    expect(isTransientUsageError(fail("Network error: timeout"))).toBe(true);
    expect(isTransientUsageError(fail("Failed to read response: reset"))).toBe(
      true,
    );
    expect(isTransientUsageError(fail("读取响应失败: reset"))).toBe(true);
  });
});

describe("resolveDisplayUsage", () => {
  it("keeps last good data for transient 5xx inside the window", () => {
    const prev: LastGoodUsage = { data: ok(42), at: T0 };
    const now = T0 + 1000;

    const result = resolveDisplayUsage(
      fail("API error (HTTP 502): bad gateway"),
      now,
      prev,
      now,
    );

    expect(result.data).toBe(prev.data);
    expect(result.lastQueriedAt).toBe(T0);
    expect(result.lastGood).toBe(prev);
  });

  it("shows deterministic failures immediately and clears last good", () => {
    const prev: LastGoodUsage = { data: ok(42), at: T0 };
    const now = T0 + 1000;
    const failure = fail("Authentication failed (HTTP 401)");

    const result = resolveDisplayUsage(failure, now, prev, now);

    expect(result.data).toBe(failure);
    expect(result.lastGood).toBeNull();
  });

  it("anchors rejected stale success data to dataUpdatedAt and expires it", () => {
    const stale = ok(42);

    const insideWindow = resolveDisplayUsage(
      stale,
      T0,
      null,
      T0 + KEEP_LAST_GOOD_MS - 1,
      { rejected: true },
    );
    expect(insideWindow.data).toBe(stale);
    expect(insideWindow.lastQueriedAt).toBe(T0);
    expect(insideWindow.lastGood).toEqual({ data: stale, at: T0 });

    const expired = resolveDisplayUsage(
      stale,
      T0,
      null,
      T0 + KEEP_LAST_GOOD_MS,
      {
        rejected: true,
      },
    );
    expect(expired.data).toBeUndefined();
    expect(expired.lastQueriedAt).toBe(T0);
    expect(expired.lastGood).toEqual({ data: stale, at: T0 });
  });

  it("supports subscription quota shaped results", () => {
    const quota: {
      success: boolean;
      error: string | null;
      credentialStatus: string;
      queriedAt: number;
    } = {
      success: true,
      error: null,
      credentialStatus: "valid",
      queriedAt: T0,
    };
    const previous = { data: quota, at: T0 };

    const result = resolveDisplayUsage(
      { ...quota, success: false, error: "API error (HTTP 429): slow down" },
      T0 + 1000,
      previous,
      T0 + 1000,
    );

    expect(result.data).toBe(quota);
  });
});
