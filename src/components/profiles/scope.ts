import type { AppId } from "@/lib/api/types";
import type { PerApp, Profile, ProfileScope } from "@/lib/api/profiles";

/**
 * Application tab to profile scope mapping. This mirrors backend
 * `ProfileScope::for_app`; unsupported tabs must not render the switcher.
 */
export const APP_PROFILE_SCOPE: Partial<Record<AppId, ProfileScope>> = {
  claude: "claude",
  codex: "codex",
};

/** Payload slots managed by each scope; mirrors backend `ProfileScope::apps`. */
const SCOPE_SLOT_KEYS: Record<ProfileScope, (keyof PerApp<unknown>)[]> = {
  claude: ["claude"],
  codex: ["codex"],
};

/** Return whether any category has been captured for the selected scope. */
export function hasScopeSnapshot(profile: Profile, scope: ProfileScope) {
  const { providers, mcp, skills, prompts } = profile.payload;
  return SCOPE_SLOT_KEYS[scope].some(
    (app) =>
      providers[app] !== null ||
      mcp[app] !== null ||
      skills[app] !== null ||
      prompts[app] !== null,
  );
}
