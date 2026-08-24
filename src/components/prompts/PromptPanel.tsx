import React, { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Download, FileText, Search } from "lucide-react";
import { Button } from "@/components/ui/button";
import { type AppId } from "@/lib/api";
import { usePromptActions } from "@/hooks/usePromptActions";
import { getPromptFilename } from "./promptFilename";
import { ManagementListSearch } from "@/components/common/ManagementListSearch";
import { ScrollArea } from "@/components/ui/scroll-area";
import PromptListItem from "./PromptListItem";
import PromptFormPanel from "./PromptFormPanel";
import PiPromptPanel, { type PromptPrimaryAction } from "./PiPromptPanel";
import { ConfirmDialog } from "../ConfirmDialog";

interface PromptPanelProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  appId: AppId;
  onInteractionBlockedChange?: (blocked: boolean) => void;
  onNavigationBlockedChange?: (blocked: boolean) => void;
  onPrimaryActionChange?: (action: PromptPrimaryAction) => void;
}

export interface PromptPanelHandle {
  openAdd: () => void;
  openImport: () => Promise<void>;
  reload: () => Promise<void>;
}

export type { PromptPrimaryAction } from "./PiPromptPanel";

/**
 * Prompt panel for every app except Pi. Pi keeps a dedicated panel because its
 * prompt surface has three tabs (global AGENTS.md, system files, templates)
 * instead of a single list — see `PiPromptPanel`.
 *
 * This standard path is intentionally NOT refactored onto `PromptLibrary`:
 * the seven pre-existing apps keep the search + inline list UI they shipped
 * with.
 */
const StandardPromptPanel = React.forwardRef<
  PromptPanelHandle,
  PromptPanelProps
>(
  (
    {
      open,
      appId,
      onInteractionBlockedChange,
      onNavigationBlockedChange,
      onPrimaryActionChange,
    },
    ref,
  ) => {
    const { t } = useTranslation();
    const [isFormOpen, setIsFormOpen] = useState(false);
    const [editingId, setEditingId] = useState<string | null>(null);
    const [searchQuery, setSearchQuery] = useState("");
    const [confirmDialog, setConfirmDialog] = useState<{
      isOpen: boolean;
      titleKey: string;
      messageKey: string;
      messageParams?: Record<string, unknown>;
      onConfirm: () => void;
    } | null>(null);
    const [writePending, setWritePending] = useState(false);
    const [reloadPending, setReloadPending] = useState(false);
    const writeLockRef = React.useRef(false);
    const reloadLockRef = React.useRef(false);
    const reloadRunGenerationRef = React.useRef(0);
    const overlayOpenRef = React.useRef(false);
    const externalReloadQueuedRef = React.useRef(false);

    const {
      prompts,
      loading,
      currentFileContent,
      reload,
      savePrompt,
      deletePrompt,
      toggleEnabled,
      importFromFile,
    } = usePromptActions(appId);
    const reloadRef = React.useRef(reload);
    reloadRef.current = reload;

    const dialogOpen = confirmDialog !== null;
    const interactionBlocked =
      loading || reloadPending || writePending || isFormOpen || dialogOpen;
    const navigationBlocked = writePending || isFormOpen || dialogOpen;

    useEffect(() => {
      onInteractionBlockedChange?.(interactionBlocked);
    }, [interactionBlocked, onInteractionBlockedChange]);

    useEffect(() => {
      onNavigationBlockedChange?.(navigationBlocked);
    }, [navigationBlocked, onNavigationBlockedChange]);

    useEffect(() => {
      onPrimaryActionChange?.("prompt");
    }, [onPrimaryActionChange]);

    useEffect(
      () => () => {
        onInteractionBlockedChange?.(false);
        onNavigationBlockedChange?.(false);
      },
      [onInteractionBlockedChange, onNavigationBlockedChange],
    );

    const runExternalReload = React.useCallback(async () => {
      if (writeLockRef.current || overlayOpenRef.current) {
        externalReloadQueuedRef.current = true;
        return;
      }

      const runGeneration = ++reloadRunGenerationRef.current;
      externalReloadQueuedRef.current = false;
      reloadLockRef.current = true;
      setReloadPending(true);
      try {
        await reloadRef.current();
      } finally {
        if (reloadRunGenerationRef.current === runGeneration) {
          reloadLockRef.current = false;
          setReloadPending(false);
        }
      }
    }, []);

    const beginWrite = () => {
      if (loading || reloadLockRef.current || writeLockRef.current)
        return false;
      writeLockRef.current = true;
      setWritePending(true);
      return true;
    };

    const endWrite = () => {
      writeLockRef.current = false;
      setWritePending(false);
      if (externalReloadQueuedRef.current) {
        void runExternalReload();
      }
    };

    useEffect(() => {
      if (open) void runExternalReload();
    }, [appId, open, runExternalReload]);

    useEffect(() => {
      setSearchQuery("");
      overlayOpenRef.current = false;
      setIsFormOpen(false);
      setEditingId(null);
      setConfirmDialog(null);
      if (externalReloadQueuedRef.current) {
        void runExternalReload();
      }
    }, [appId, runExternalReload]);

    // Listen for prompt import events from deep link
    useEffect(() => {
      const handlePromptImported = (event: Event) => {
        const customEvent = event as CustomEvent;
        // Reload if the import is for this app
        if (customEvent.detail?.app === appId) {
          void runExternalReload();
        }
      };

      window.addEventListener("prompt-imported", handlePromptImported);
      return () => {
        window.removeEventListener("prompt-imported", handlePromptImported);
      };
    }, [appId, runExternalReload]);

    const handleAdd = () => {
      if (reloadLockRef.current || writeLockRef.current || interactionBlocked) {
        return;
      }
      overlayOpenRef.current = true;
      setEditingId(null);
      setIsFormOpen(true);
    };

    const handleImport = async () => {
      if (!beginWrite()) return;
      try {
        await importFromFile();
      } finally {
        endWrite();
      }
    };

    React.useImperativeHandle(ref, () => ({
      openAdd: handleAdd,
      openImport: handleImport,
      reload: runExternalReload,
    }));

    const handleEdit = (id: string) => {
      if (reloadLockRef.current || writeLockRef.current || interactionBlocked) {
        return;
      }
      overlayOpenRef.current = true;
      setEditingId(id);
      setIsFormOpen(true);
    };

    const handleDelete = (id: string) => {
      if (reloadLockRef.current || writeLockRef.current || interactionBlocked) {
        return;
      }
      const prompt = prompts[id];
      overlayOpenRef.current = true;
      setConfirmDialog({
        isOpen: true,
        titleKey: "prompts.confirm.deleteTitle",
        messageKey: "prompts.confirm.deleteMessage",
        messageParams: { name: prompt?.name },
        onConfirm: async () => {
          if (!beginWrite()) return;
          try {
            const refreshed = await deletePrompt(id);
            if (refreshed === false) {
              externalReloadQueuedRef.current = true;
            }
            overlayOpenRef.current = false;
            setConfirmDialog(null);
          } catch (e) {
            // Error handled by hook
          } finally {
            endWrite();
          }
        },
      });
    };

    const handleToggle = async (id: string, enabled: boolean) => {
      if (!beginWrite()) return;
      try {
        const refreshed = await toggleEnabled(id, enabled);
        if (refreshed === false) {
          externalReloadQueuedRef.current = true;
        }
      } catch (error) {
        // Error handled by hook
      } finally {
        endWrite();
      }
    };

    const handleSave = async (
      id: string,
      prompt: Parameters<typeof savePrompt>[1],
    ) => {
      if (!beginWrite()) return false;
      try {
        const refreshed = await savePrompt(id, prompt);
        if (refreshed === false) {
          externalReloadQueuedRef.current = true;
        }
        return true;
      } catch (error) {
        // Error handled by hook
        return false;
      } finally {
        endWrite();
      }
    };

    const handleCloseForm = () => {
      if (writeLockRef.current) return;
      overlayOpenRef.current = false;
      setIsFormOpen(false);
      setEditingId(null);
      if (externalReloadQueuedRef.current) {
        void runExternalReload();
      }
    };

    const promptEntries = useMemo(() => Object.entries(prompts), [prompts]);
    const normalizedSearchQuery = searchQuery.trim().toLocaleLowerCase();
    const filteredPromptEntries = useMemo(() => {
      if (!normalizedSearchQuery) return promptEntries;

      return promptEntries.filter(([recordId, prompt]) =>
        [
          recordId,
          prompt.id,
          prompt.name,
          prompt.description,
          prompt.content,
        ].some((value) =>
          value?.toLocaleLowerCase().includes(normalizedSearchQuery),
        ),
      );
    }, [normalizedSearchQuery, promptEntries]);

    const enabledPrompt = promptEntries.find(([_, p]) => p.enabled);

    return (
      <div className="flex flex-col flex-1 min-h-0 px-6">
        <div className="flex-shrink-0 py-4 glass rounded-xl border border-white/10 mb-4 px-6">
          <div className="text-sm text-muted-foreground">
            {t("prompts.count", { count: promptEntries.length })} ·{" "}
            {enabledPrompt
              ? t("prompts.enabledName", { name: enabledPrompt[1].name })
              : t("prompts.noneEnabled")}
          </div>
        </div>

        {currentFileContent ? (
          <div className="flex-shrink-0 mb-4 rounded-xl border border-border bg-muted/40 p-4">
            <div className="mb-2 text-sm font-medium text-foreground">
              {t("prompts.currentFile", {
                filename: getPromptFilename(appId),
              })}
            </div>
            <pre className="max-h-48 overflow-auto whitespace-pre-wrap break-words rounded-lg bg-background/80 p-3 text-xs text-muted-foreground">
              {currentFileContent}
            </pre>
          </div>
        ) : null}

        <ManagementListSearch
          value={searchQuery}
          onValueChange={setSearchQuery}
          placeholder={t("prompts.searchPlaceholder")}
          ariaLabel={t("prompts.searchAriaLabel")}
          clearLabel={t("common.clear")}
        />

        <ScrollArea className="-mr-3 flex-1 min-h-0" type="auto">
          <div className="pb-16 pr-3">
            {loading ? (
              <div className="text-center py-12 text-muted-foreground">
                {t("prompts.loading")}
              </div>
            ) : promptEntries.length === 0 ? (
              <div className="text-center py-12">
                <div className="w-16 h-16 mx-auto mb-4 bg-muted rounded-full flex items-center justify-center">
                  <FileText size={24} className="text-muted-foreground" />
                </div>
                <h3 className="text-lg font-medium text-foreground mb-2">
                  {t("prompts.empty")}
                </h3>
                <p className="text-muted-foreground text-sm">
                  {t("prompts.emptyDescription")}
                </p>
                <div className="mt-6">
                  <Button
                    type="button"
                    variant="outline"
                    disabled={interactionBlocked}
                    onClick={() => void handleImport()}
                  >
                    <Download className="mr-2 h-4 w-4" />
                    {t("prompts.import")}
                  </Button>
                </div>
              </div>
            ) : filteredPromptEntries.length === 0 ? (
              <div className="flex flex-col items-center justify-center py-12 text-center text-muted-foreground">
                <Search className="mb-4 h-10 w-10 opacity-40" />
                <p className="text-sm">{t("prompts.noSearchResults")}</p>
              </div>
            ) : (
              <div className="space-y-3">
                {filteredPromptEntries.map(([id, prompt]) => (
                  <PromptListItem
                    key={id}
                    id={id}
                    prompt={prompt}
                    onToggle={handleToggle}
                    onEdit={handleEdit}
                    onDelete={handleDelete}
                    disabled={interactionBlocked}
                  />
                ))}
              </div>
            )}
          </div>
        </ScrollArea>

        {isFormOpen && (
          <PromptFormPanel
            appId={appId}
            editingId={editingId || undefined}
            initialData={editingId ? prompts[editingId] : undefined}
            onSave={handleSave}
            onClose={handleCloseForm}
          />
        )}

        {confirmDialog && (
          <ConfirmDialog
            isOpen={confirmDialog.isOpen}
            title={t(confirmDialog.titleKey)}
            message={t(confirmDialog.messageKey, confirmDialog.messageParams)}
            pending={writePending}
            onConfirm={confirmDialog.onConfirm}
            onCancel={() => {
              if (!writeLockRef.current) {
                overlayOpenRef.current = false;
                setConfirmDialog(null);
                if (externalReloadQueuedRef.current) {
                  void runExternalReload();
                }
              }
            }}
          />
        )}
      </div>
    );
  },
);

StandardPromptPanel.displayName = "StandardPromptPanel";

const PromptPanel = React.forwardRef<PromptPanelHandle, PromptPanelProps>(
  (props, ref) => {
    if (props.appId === "pi") {
      return (
        <PiPromptPanel
          ref={ref}
          open={props.open}
          onInteractionBlockedChange={props.onInteractionBlockedChange}
          onNavigationBlockedChange={props.onNavigationBlockedChange}
          onPrimaryActionChange={props.onPrimaryActionChange}
        />
      );
    }

    return <StandardPromptPanel ref={ref} {...props} />;
  },
);

PromptPanel.displayName = "PromptPanel";

export default PromptPanel;
