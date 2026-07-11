import type { UsageRangePreset, UsageRangeSelection } from "@/types/usage";
import { getUsageUtcOffsetMinutes } from "@/lib/serverClock";

const DAY_SECONDS = 24 * 60 * 60;
const DAY_MS = DAY_SECONDS * 1000;
const MINUTE_MS = 60 * 1000;

export interface ResolvedUsageRange {
  startDate: number;
  endDate: number;
}

/**
 * Start-of-day, in the timezone the backend buckets usage by.
 *
 * M5: computes midnight in the resolved usage timezone (server-local in web
 * mode, browser-local on desktop) rather than always browser-local, so range
 * queries align with the server-local day bucketing used by usage rollups.
 */
function getStartOfLocalDayDate(nowMs: number): Date {
  const offsetMs = getUsageUtcOffsetMinutes() * MINUTE_MS;
  // Shift into the target timezone, truncate to its midnight, shift back to UTC.
  const shifted = new Date(nowMs + offsetMs);
  const midnightUtc = Date.UTC(
    shifted.getUTCFullYear(),
    shifted.getUTCMonth(),
    shifted.getUTCDate(),
  );
  return new Date(midnightUtc - offsetMs);
}

function getPresetLookbackStart(
  preset: Exclude<UsageRangePreset, "today" | "1d" | "custom">,
  nowMs: number,
): number {
  const dayCount = preset === "7d" ? 7 : preset === "14d" ? 14 : 30;
  return Math.floor(
    getStartOfLocalDayDate(nowMs - (dayCount - 1) * DAY_MS).getTime() / 1000,
  );
}

export function resolveUsageRange(
  selection: UsageRangeSelection,
  nowMs: number = Date.now(),
): ResolvedUsageRange {
  const endDate = Math.floor(nowMs / 1000);

  switch (selection.preset) {
    case "today":
      return {
        startDate: Math.floor(getStartOfLocalDayDate(nowMs).getTime() / 1000),
        endDate,
      };
    case "1d":
      return {
        startDate: endDate - DAY_SECONDS,
        endDate,
      };
    case "7d":
    case "14d":
    case "30d":
      return {
        startDate: getPresetLookbackStart(selection.preset, nowMs),
        endDate,
      };
    case "custom": {
      const startDate = selection.customStartDate ?? endDate - DAY_SECONDS;
      const customEndDate = selection.customEndDate ?? endDate;
      return {
        startDate,
        endDate: customEndDate,
      };
    }
  }
}

export function getUsageRangePresetLabel(
  preset: UsageRangePreset,
  t: (key: string, options?: { defaultValue?: string }) => string,
): string {
  switch (preset) {
    case "today":
      return t("usage.presetToday", { defaultValue: "当天" });
    case "1d":
      return t("usage.preset1d", { defaultValue: "1d" });
    case "7d":
      return t("usage.preset7d", { defaultValue: "7d" });
    case "14d":
      return t("usage.preset14d", { defaultValue: "14d" });
    case "30d":
      return t("usage.preset30d", { defaultValue: "30d" });
    case "custom":
      return t("usage.customRange", { defaultValue: "日历筛选" });
  }
}
