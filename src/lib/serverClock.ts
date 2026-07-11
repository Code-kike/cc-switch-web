import { useEffect } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { isWebMode, webJsonFetch } from "./api/adapter";

/**
 * Server-timezone awareness for usage day-boundary math.
 *
 * M5: In the web deployment the browser and the server may be in different
 * timezones. Usage rollups are pre-aggregated by SERVER-local day and cannot be
 * re-bucketed per client, so the frontend must compute usage day boundaries in
 * the SERVER's timezone (not the browser's) for range queries and rollups to
 * agree. On desktop the two coincide, so this is a no-op there.
 *
 * The offset is fetched once from `/api/env/platform` (web mode only) and cached
 * at module scope. Until it is loaded — and always on desktop — callers fall
 * back to the browser's own UTC offset, preserving prior behavior.
 */

// Browser's minutes-east-of-UTC. `getTimezoneOffset()` is minutes WEST of UTC,
// so negate it to match the server's `local_minus_utc()/60` convention.
function browserUtcOffsetMinutes(): number {
  return -new Date().getTimezoneOffset();
}

let serverUtcOffsetMinutes: number | null = null;

export function setServerUtcOffsetMinutes(minutes: number | null): void {
  serverUtcOffsetMinutes =
    typeof minutes === "number" && Number.isFinite(minutes) ? minutes : null;
}

/**
 * The UTC offset (minutes east of UTC) to use for usage day boundaries.
 * Server offset in web mode once known; otherwise the browser's own offset.
 */
export function getUsageUtcOffsetMinutes(): number {
  if (isWebMode() && serverUtcOffsetMinutes !== null) {
    return serverUtcOffsetMinutes;
  }
  return browserUtcOffsetMinutes();
}

interface PlatformInfo {
  utcOffsetMinutes?: number;
}

/**
 * Populate the cached server UTC offset once, in web mode. Safe to mount from
 * any usage view; failures leave the browser-offset fallback in place.
 */
export function useServerTimezone(): void {
  const queryClient = useQueryClient();
  useEffect(() => {
    if (!isWebMode()) return;
    let cancelled = false;
    webJsonFetch<PlatformInfo>("/api/env/platform")
      .then((info) => {
        if (cancelled || typeof info?.utcOffsetMinutes !== "number") return;
        const previous = getUsageUtcOffsetMinutes();
        setServerUtcOffsetMinutes(info.utcOffsetMinutes);
        // If the server timezone differs from the browser's, usage day
        // boundaries just changed — refetch usage queries that were computed
        // with the browser-offset fallback on first render.
        if (getUsageUtcOffsetMinutes() !== previous) {
          void queryClient.invalidateQueries({ queryKey: ["usage"] });
        }
      })
      .catch(() => {
        /* keep browser-offset fallback */
      });
    return () => {
      cancelled = true;
    };
  }, [queryClient]);
}
