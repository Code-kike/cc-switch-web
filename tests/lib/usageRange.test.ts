import { afterEach, describe, expect, it } from "vitest";
import { resolveUsageRange } from "@/lib/usageRange";
import { setServerUtcOffsetMinutes } from "@/lib/serverClock";

// In jsdom isWebMode() is true, so a set server offset is used for day
// boundaries. Reset it after each test to avoid leaking state.
afterEach(() => setServerUtcOffsetMinutes(null));

describe("resolveUsageRange timezone awareness (M5)", () => {
  it("computes 'today' start at server-local midnight, not browser-local", () => {
    setServerUtcOffsetMinutes(480); // UTC+8
    // 2026-07-11T02:00:00Z == 2026-07-11 10:00 in UTC+8.
    const nowMs = Date.UTC(2026, 6, 11, 2, 0, 0);
    const { startDate } = resolveUsageRange({ preset: "today" }, nowMs);
    // Local (UTC+8) midnight of 2026-07-11 is 2026-07-10T16:00:00Z.
    expect(startDate).toBe(Math.floor(Date.UTC(2026, 6, 10, 16, 0, 0) / 1000));
  });

  it("start epoch lands exactly on a server-local midnight boundary", () => {
    setServerUtcOffsetMinutes(-300); // UTC-5
    const nowMs = Date.UTC(2026, 6, 11, 2, 0, 0); // 2026-07-10 21:00 in UTC-5
    const { startDate } = resolveUsageRange({ preset: "today" }, nowMs);
    // Local (UTC-5) midnight of 2026-07-10 is 2026-07-10T05:00:00Z.
    expect(startDate).toBe(Math.floor(Date.UTC(2026, 6, 10, 5, 0, 0) / 1000));
    // The resulting instant is an exact multiple of 60 s at a wall-clock
    // midnight in the server tz (no partial-day offset).
    expect((startDate + -300 * 60) % 86400).toBe(0);
  });

  it("7d preset lookback also anchors to server-local midnight", () => {
    setServerUtcOffsetMinutes(480);
    const nowMs = Date.UTC(2026, 6, 11, 2, 0, 0);
    const { startDate } = resolveUsageRange({ preset: "7d" }, nowMs);
    // 7-day window ending 2026-07-11 (local) starts at 2026-07-05 local midnight
    // = 2026-07-04T16:00:00Z.
    expect(startDate).toBe(Math.floor(Date.UTC(2026, 6, 4, 16, 0, 0) / 1000));
  });
});
