import { invoke } from "./adapter";

/**
 * Profile scopes mirrored from the backend `ProfileScope`.
 *
 * Project entities are shared, while snapshots, apply operations, and current
 * pointers are independent for Claude Code and Codex. Other application tabs
 * do not expose project profiles yet.
 */
export type ProfileScope = "claude" | "codex";

/** Per-application payload slots mirrored from backend `PerApp<T>`. */
export interface PerApp<T> {
  claude: T;
  codex: T;
}

/**
 * Snapshot payload mirrored from backend `ProfilePayload`.
 *
 * A null slot means that scope has never been captured and must be left
 * untouched when applied. An empty array is an authoritative captured empty
 * set and disables every item in that category.
 */
export interface ProfilePayload {
  providers: PerApp<string | null>;
  mcp: PerApp<string[] | null>;
  skills: PerApp<string[] | null>;
  prompts: PerApp<string | null>;
}

export interface Profile {
  id: string;
  name: string;
  payload: ProfilePayload;
  createdAt?: number;
  updatedAt?: number;
}

/** Current project id for each supported scope. */
export interface CurrentProfileIds {
  claude: string | null;
  codex: string | null;
}

export interface ProfilesResponse {
  profiles: Profile[];
  currentIds: CurrentProfileIds;
}

export const profilesApi = {
  async list(): Promise<ProfilesResponse> {
    return await invoke("list_profiles");
  },

  async create(name: string, scope: ProfileScope): Promise<Profile> {
    return await invoke("create_profile", { name, scope });
  },

  async update(
    id: string,
    options: { name?: string; resnapshot?: boolean; scope?: ProfileScope },
  ): Promise<Profile> {
    return await invoke("update_profile", {
      id,
      name: options.name,
      resnapshot: options.resnapshot,
      scope: options.scope,
    });
  },

  async delete(id: string): Promise<void> {
    await invoke("delete_profile", { id });
  },

  async apply(id: string, scope: ProfileScope): Promise<string[]> {
    return await invoke("apply_profile", { id, scope });
  },

  async clearCurrent(scope: ProfileScope): Promise<void> {
    await invoke("clear_current_profile", { scope });
  },
};
