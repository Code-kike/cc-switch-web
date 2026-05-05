import type { AppId } from "@/lib/api";

const PROMPT_FILENAME_MAP: Record<AppId, string> = {
  claude: "CLAUDE.md",
  codex: "AGENTS.md",
  gemini: "GEMINI.md",
  opencode: "AGENTS.md",
  openclaw: "AGENTS.md",
  hermes: "AGENTS.md",
};

export function getPromptFilename(appId: AppId): string {
  return PROMPT_FILENAME_MAP[appId];
}
