import type { AppId } from "@/lib/api";

/**
 * App navigation/persistence helpers extracted from App.tsx (L26 conservative
 * split). Pure, module-scope logic only — no React, no component state. The
 * view-clamping rules (GLOBAL_VIEWS / APP_VIEWS / isViewAllowedForApp) back the
 * L32 fix and are pinned by tests/integration/App.test.tsx.
 */

export type View =
  | "providers"
  | "settings"
  | "prompts"
  | "skills"
  | "skillsDiscovery"
  | "mcp"
  | "agents"
  | "universal"
  | "sessions"
  | "workspace"
  | "openclawEnv"
  | "openclawTools"
  | "openclawAgents"
  | "hermesMemory";

const STORAGE_KEY = "cc-switch-last-app";
const VALID_APPS: AppId[] = [
  "claude",
  "codex",
  "gemini",
  "opencode",
  "openclaw",
  "hermes",
];

export const getInitialApp = (): AppId => {
  const saved = localStorage.getItem(STORAGE_KEY) as AppId | null;
  if (saved && VALID_APPS.includes(saved)) {
    return saved;
  }
  return "claude";
};

export const VIEW_STORAGE_KEY = "cc-switch-last-view";
const VALID_VIEWS: View[] = [
  "providers",
  "settings",
  "prompts",
  "skills",
  "skillsDiscovery",
  "mcp",
  "agents",
  "universal",
  "sessions",
  "workspace",
  "openclawEnv",
  "openclawTools",
  "openclawAgents",
  "hermesMemory",
];

// Views reachable regardless of the active app: the providers list (default),
// the settings page (gear icon / Cmd+,), and the app-agnostic agents panel.
const GLOBAL_VIEWS: View[] = ["providers", "settings", "agents"];

// App-specific views, derived from each app's header toolbar in App():
// an app only exposes navigation to the views listed here. A persisted view
// that is not permitted for the active app must be clamped, otherwise a panel
// belonging to a different app can render under a mismatched app (e.g. the
// openclaw workspace panel under claude, or hermesMemory under codex). This
// happens because `activeApp` and `currentView` are persisted independently
// and the visibility effect can switch `activeApp` away from a hidden app
// while a now-incompatible view is still active (L32).
const APP_VIEWS: Record<AppId, View[]> = {
  claude: ["skills", "skillsDiscovery", "prompts", "universal", "sessions", "mcp"],
  codex: ["skills", "skillsDiscovery", "prompts", "universal", "sessions", "mcp"],
  gemini: ["skills", "skillsDiscovery", "prompts", "universal", "sessions", "mcp"],
  opencode: ["skills", "skillsDiscovery", "prompts", "sessions", "mcp"],
  openclaw: [
    "workspace",
    "openclawEnv",
    "openclawTools",
    "openclawAgents",
    "sessions",
  ],
  hermes: ["skills", "skillsDiscovery", "hermesMemory", "mcp"],
};

export const isViewAllowedForApp = (view: View, app: AppId): boolean =>
  GLOBAL_VIEWS.includes(view) || APP_VIEWS[app].includes(view);

export const getInitialView = (app: AppId): View => {
  const saved = localStorage.getItem(VIEW_STORAGE_KEY) as View | null;
  if (saved && VALID_VIEWS.includes(saved) && isViewAllowedForApp(saved, app)) {
    return saved;
  }
  return "providers";
};
