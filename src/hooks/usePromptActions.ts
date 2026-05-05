import { useState, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { promptsApi, type Prompt, type AppId } from "@/lib/api";
import { extractErrorMessage } from "@/utils/errorUtils";

type PromptErrorKey =
  | "prompts.loadFailed"
  | "prompts.currentFileLoadFailed"
  | "prompts.saveFailed"
  | "prompts.deleteFailed"
  | "prompts.enableFailed"
  | "prompts.disableFailed"
  | "prompts.importFailed";

export function usePromptActions(appId: AppId) {
  const { t } = useTranslation();
  const [prompts, setPrompts] = useState<Record<string, Prompt>>({});
  const [loading, setLoading] = useState(false);
  const [currentFileContent, setCurrentFileContent] = useState<string | null>(
    null,
  );

  const showPromptError = useCallback(
    (key: PromptErrorKey, error: unknown) => {
      toast.error(t(key), {
        description: extractErrorMessage(error) || undefined,
      });
    },
    [t],
  );

  const reload = useCallback(async (options?: { silent?: boolean }) => {
    const silent = options?.silent === true;
    if (!silent) {
      setLoading(true);
    }
    try {
      const data = await promptsApi.getPrompts(appId);
      setPrompts(data);

      // 同时加载当前文件内容
      try {
        const content = await promptsApi.getCurrentFileContent(appId);
        setCurrentFileContent(content);
      } catch (error) {
        setCurrentFileContent(null);
        showPromptError("prompts.currentFileLoadFailed", error);
      }
    } catch (error) {
      showPromptError("prompts.loadFailed", error);
    } finally {
      if (!silent) {
        setLoading(false);
      }
    }
  }, [appId, showPromptError]);

  const savePrompt = useCallback(
    async (id: string, prompt: Prompt) => {
      try {
        await promptsApi.upsertPrompt(appId, id, prompt);
        setPrompts((prev) => ({
          ...prev,
          [id]: prompt,
        }));
        if (prompt.enabled) {
          setCurrentFileContent(prompt.content);
        }
        void reload({ silent: true });
        toast.success(t("prompts.saveSuccess"), { closeButton: true });
      } catch (error) {
        showPromptError("prompts.saveFailed", error);
        throw error;
      }
    },
    [appId, reload, showPromptError, t],
  );

  const deletePrompt = useCallback(
    async (id: string) => {
      try {
        await promptsApi.deletePrompt(appId, id);
        await reload();
        toast.success(t("prompts.deleteSuccess"), { closeButton: true });
      } catch (error) {
        showPromptError("prompts.deleteFailed", error);
        throw error;
      }
    },
    [appId, reload, showPromptError, t],
  );

  const enablePrompt = useCallback(
    async (id: string) => {
      try {
        await promptsApi.enablePrompt(appId, id);
        await reload();
        toast.success(t("prompts.enableSuccess"), { closeButton: true });
      } catch (error) {
        showPromptError("prompts.enableFailed", error);
        throw error;
      }
    },
    [appId, reload, showPromptError, t],
  );

  const toggleEnabled = useCallback(
    async (id: string, enabled: boolean) => {
      // Optimistic update
      const previousPrompts = prompts;

      // 如果要启用当前提示词，先禁用其他所有提示词
      if (enabled) {
        const updatedPrompts = Object.keys(prompts).reduce(
          (acc, key) => {
            acc[key] = {
              ...prompts[key],
              enabled: key === id,
            };
            return acc;
          },
          {} as Record<string, Prompt>,
        );
        setPrompts(updatedPrompts);
      } else {
        setPrompts((prev) => ({
          ...prev,
          [id]: {
            ...prev[id],
            enabled: false,
          },
        }));
      }

      try {
        if (enabled) {
          await promptsApi.enablePrompt(appId, id);
          toast.success(t("prompts.enableSuccess"), { closeButton: true });
        } else {
          // 禁用提示词 - 需要后端支持
          await promptsApi.upsertPrompt(appId, id, {
            ...prompts[id],
            enabled: false,
          });
          toast.success(t("prompts.disableSuccess"), { closeButton: true });
        }
        await reload();
      } catch (error) {
        // Rollback on failure
        setPrompts(previousPrompts);
        showPromptError(
          enabled ? "prompts.enableFailed" : "prompts.disableFailed",
          error,
        );
        throw error;
      }
    },
    [appId, prompts, reload, showPromptError, t],
  );

  const importFromFile = useCallback(async () => {
    try {
      const id = await promptsApi.importFromFile(appId);
      if (!id) {
        return null;
      }
      await reload();
      toast.success(t("prompts.importSuccess"), { closeButton: true });
      return id;
    } catch (error) {
      showPromptError("prompts.importFailed", error);
      throw error;
    }
  }, [appId, reload, showPromptError, t]);

  return {
    prompts,
    loading,
    currentFileContent,
    reload,
    savePrompt,
    deletePrompt,
    enablePrompt,
    toggleEnabled,
    importFromFile,
  };
}
