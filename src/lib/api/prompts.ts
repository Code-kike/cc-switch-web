import { invoke, isWebMode, pickWebFile, webUpload } from "./adapter";
import type { AppId } from "./types";

export interface Prompt {
  id: string;
  name: string;
  content: string;
  description?: string;
  enabled: boolean;
  createdAt?: number;
  updatedAt?: number;
}

export const promptsApi = {
  async getPrompts(app: AppId): Promise<Record<string, Prompt>> {
    return await invoke("get_prompts", { app });
  },

  async upsertPrompt(app: AppId, id: string, prompt: Prompt): Promise<void> {
    return await invoke("upsert_prompt", { app, id, prompt });
  },

  async deletePrompt(app: AppId, id: string): Promise<void> {
    return await invoke("delete_prompt", { app, id });
  },

  async enablePrompt(app: AppId, id: string): Promise<void> {
    return await invoke("enable_prompt", { app, id });
  },

  async importFromFile(app: AppId): Promise<string | null> {
    if (isWebMode()) {
      const file = await pickWebFile(".md,text/markdown,text/plain");
      if (!file) return null;
      const formData = new FormData();
      formData.set("file", file);
      return await webUpload(
        `/api/prompts/import-prompt-upload?app=${encodeURIComponent(app)}`,
        formData,
      );
    }
    return await invoke("import_prompt_from_file", { app });
  },

  async getCurrentFileContent(app: AppId): Promise<string | null> {
    return await invoke("get_current_prompt_file_content", { app });
  },
};
