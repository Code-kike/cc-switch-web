import { useEffect, useMemo, useState, useRef } from "react";
import { useTranslation } from "react-i18next";
import { motion, AnimatePresence } from "framer-motion";
import { toast } from "sonner";
import { useQueryClient } from "@tanstack/react-query";
import {
  Plus,
  Settings,
  ArrowLeft,
  Minus,
  Maximize2,
  Minimize2,
  X,
  Book,
  Brain,
  Wrench,
  RefreshCw,
  History,
  BarChart2,
  Download,
  FolderArchive,
  Link2,
  Search,
  FolderOpen,
  KeyRound,
  Shield,
  Cpu,
  Layers,
  LayoutDashboard,
  Loader2,
} from "lucide-react";
import type { Provider, VisibleApps } from "@/types";
import type { EnvConflict } from "@/types/env";
import { invoke, isWebMode } from "@/lib/api/adapter";
import { listen } from "@/lib/api/event-adapter";
import { proxyKeys, useProvidersQuery, useSettingsQuery } from "@/lib/query";
import { universalProviderKeys } from "@/lib/query/universal";
import {
  providersApi,
  settingsApi,
  piApi,
  type AppId,
  type ProviderSwitchEvent,
} from "@/lib/api";
import { checkAllEnvConflicts, checkEnvConflicts } from "@/lib/api/env";
import { useProviderActions } from "@/hooks/useProviderActions";
import { openclawKeys, useOpenClawHealth } from "@/hooks/useOpenClaw";
import {
  hermesKeys,
  invalidateHermesProviderCaches,
  useOpenHermesWebUI,
} from "@/hooks/useHermes";
import { hermesApi } from "@/lib/api/hermes";
import { useProxyStatus } from "@/hooks/useProxyStatus";
import { useUsageCacheBridge } from "@/hooks/useUsageCacheBridge";
import { useLaggedRecovery } from "@/hooks/useLaggedRecovery";
import { useLastValidValue } from "@/hooks/useLastValidValue";
import {
  extractErrorMessage,
  translatePiProviderMutationError,
} from "@/utils/errorUtils";
import { isTextEditableTarget } from "@/utils/domUtils";
import { cn } from "@/lib/utils";
import {
  isWindows,
  isLinux,
  DRAG_REGION_ATTR,
  DRAG_REGION_STYLE,
} from "@/lib/platform";
import { AppSwitcher } from "@/components/AppSwitcher";
import { ProfileSwitcher } from "@/components/profiles/ProfileSwitcher";
import { ProviderList } from "@/components/providers/ProviderList";
import { AddProviderDialog } from "@/components/providers/AddProviderDialog";
import { EditProviderDialog } from "@/components/providers/EditProviderDialog";
import { ConfirmDialog } from "@/components/ConfirmDialog";
import { SettingsPage } from "@/components/settings/SettingsPage";
import { UpdateBadge } from "@/components/UpdateBadge";
import { EnvWarningBanner } from "@/components/env/EnvWarningBanner";
import { ProxyToggle } from "@/components/proxy/ProxyToggle";
import { FailoverToggle } from "@/components/proxy/FailoverToggle";
import { RoutingActivationBrand } from "@/components/proxy/RoutingActivationBrand";
import UsageScriptModal from "@/components/UsageScriptModal";
import UnifiedMcpPanel, {
  type UnifiedMcpPanelHandle,
} from "@/components/mcp/UnifiedMcpPanel";
import PromptPanel, {
  type PromptPanelHandle,
  type PromptPrimaryAction,
} from "@/components/prompts/PromptPanel";
import {
  SkillsPage,
  type SkillsPageHandle,
} from "@/components/skills/SkillsPage";
import UnifiedSkillsPanel, {
  type UnifiedSkillsPanelHandle,
  type SkillsCheckUpdatesState,
} from "@/components/skills/UnifiedSkillsPanel";
import {
  DeepLinkImportDialog,
  type DeepLinkImportDialogHandle,
} from "@/components/DeepLinkImportDialog";
import { FirstRunNoticeDialog } from "@/components/FirstRunNoticeDialog";
import { AgentsPanel } from "@/components/agents/AgentsPanel";
import { UniversalProviderPanel } from "@/components/universal";
import { McpIcon } from "@/components/BrandIcons";
import { Button } from "@/components/ui/button";
import { SessionManagerPage } from "@/components/sessions/SessionManagerPage";
import {
  useDisableCurrentOmo,
  useDisableCurrentOmoSlim,
} from "@/lib/query/omo";
import {
  invalidatePiProviderCaches,
  piKeys,
  usePiCurrentState,
} from "@/lib/query/pi";
import WorkspaceFilesPanel from "@/components/workspace/WorkspaceFilesPanel";
import EnvPanel from "@/components/openclaw/EnvPanel";
import ToolsPanel from "@/components/openclaw/ToolsPanel";
import AgentsDefaultsPanel from "@/components/openclaw/AgentsDefaultsPanel";
import OpenClawHealthBanner from "@/components/openclaw/OpenClawHealthBanner";
import HermesMemoryPanel from "@/components/hermes/HermesMemoryPanel";
import { importCurrentProviderConfig } from "@/lib/providers/import-current-config";
import { generateUniqueProviderCopyKey } from "@/lib/providers/providerCopyKey";
import {
  type View,
  VIEW_STORAGE_KEY,
  getInitialApp,
  getInitialView,
  isViewAllowedForApp,
} from "./App.navigation";

interface SyncStatusUpdatedPayload {
  source?: string;
  status?: string;
  error?: string;
}

const DEFAULT_DRAG_BAR_HEIGHT = isWindows() || isLinux() ? 0 : 28; // px
const HEADER_HEIGHT = 64; // px

const getDesktopCurrentWindow = async () => {
  const tauriWindow = await import("@tauri-apps/api/window");
  return tauriWindow.getCurrentWindow();
};

function App() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const webMode = isWebMode();

  const [activeApp, setActiveApp] = useState<AppId>(getInitialApp);
  const [currentView, setCurrentView] = useState<View>(() =>
    getInitialView(getInitialApp()),
  );
  const [settingsDefaultTab, setSettingsDefaultTab] = useState("general");
  const [isAddOpen, setIsAddOpen] = useState(false);
  const [isWindowMaximized, setIsWindowMaximized] = useState(false);
  const [mcpManagementBusy, setMcpManagementBusy] = useState(false);
  const [skillsManagementBusy, setSkillsManagementBusy] = useState(false);
  const [skillsNavigationBusy, setSkillsNavigationBusy] = useState(false);
  const [promptManagementBusy, setPromptManagementBusy] = useState(false);
  const [promptNavigationBusy, setPromptNavigationBusy] = useState(false);
  const [skillsCheckUpdatesState, setSkillsCheckUpdatesState] =
    useState<SkillsCheckUpdatesState>({
      isChecking: false,
      hasSkills: false,
    });

  useEffect(() => {
    localStorage.setItem(VIEW_STORAGE_KEY, currentView);
  }, [currentView]);

  const { data: settingsData } = useSettingsQuery();
  const useAppWindowControls =
    !webMode && isLinux() && (settingsData?.useAppWindowControls ?? false);
  const dragBarHeight = webMode
    ? 0
    : useAppWindowControls
      ? 32
      : DEFAULT_DRAG_BAR_HEIGHT;
  const contentTopOffset = dragBarHeight + HEADER_HEIGHT;
  const showDesktopTitlebar =
    !webMode && (dragBarHeight > 0 || useAppWindowControls);
  const visibleApps: VisibleApps = settingsData?.visibleApps ?? {
    claude: true,
    codex: true,
    gemini: true,
    grokbuild: true,
    opencode: true,
    openclaw: true,
    hermes: true,
    pi: true,
  };

  const getFirstVisibleApp = (): AppId => {
    if (visibleApps.claude) return "claude";
    if (visibleApps.codex) return "codex";
    if (visibleApps.gemini) return "gemini";
    if (visibleApps.grokbuild) return "grokbuild";
    if (visibleApps.opencode) return "opencode";
    if (visibleApps.openclaw) return "openclaw";
    if (visibleApps.hermes) return "hermes";
    if (visibleApps.pi) return "pi";
    return "claude"; // fallback
  };

  useEffect(() => {
    if (!visibleApps[activeApp]) {
      setActiveApp(getFirstVisibleApp());
    }
  }, [visibleApps, activeApp]);

  // Keep the active view consistent with the active app. When `activeApp`
  // changes (initial restore, or the visibility effect above switching away
  // from a hidden app), clamp any view that is not reachable for the new app
  // back to the providers list, so an app-specific panel never renders under a
  // mismatched app (L32).
  useEffect(() => {
    if (!isViewAllowedForApp(currentView, activeApp)) {
      setCurrentView("providers");
    }
  }, [activeApp, currentView]);

  const [editingProvider, setEditingProvider] = useState<Provider | null>(null);
  const [usageProvider, setUsageProvider] = useState<Provider | null>(null);
  const [confirmAction, setConfirmAction] = useState<{
    provider: Provider;
    action: "remove" | "delete";
  } | null>(null);
  const [envConflicts, setEnvConflicts] = useState<EnvConflict[]>([]);
  const [showEnvBanner, setShowEnvBanner] = useState(false);

  const effectiveEditingProvider = useLastValidValue(editingProvider);
  const effectiveUsageProvider = useLastValidValue(usageProvider);

  useUsageCacheBridge();
  useLaggedRecovery();

  const promptPanelRef = useRef<PromptPanelHandle>(null);
  const [promptPrimaryAction, setPromptPrimaryAction] =
    useState<PromptPrimaryAction>("prompt");
  const mcpPanelRef = useRef<UnifiedMcpPanelHandle>(null);
  const skillsPageRef = useRef<SkillsPageHandle>(null);
  const unifiedSkillsPanelRef = useRef<UnifiedSkillsPanelHandle>(null);
  const deepLinkImportDialogRef = useRef<DeepLinkImportDialogHandle>(null);
  const addActionButtonClass =
    "bg-orange-500 hover:bg-orange-600 dark:bg-orange-500 dark:hover:bg-orange-600 text-white shadow-lg shadow-orange-500/30 dark:shadow-orange-500/40 rounded-full w-8 h-8";

  const {
    isRunning: isProxyRunning,
    takeoverStatus,
    status: proxyStatus,
  } = useProxyStatus();
  const isCurrentAppTakeoverActive = takeoverStatus?.[activeApp] || false;

  const handleImportCurrentProvider = async () => {
    try {
      const result = await importCurrentProviderConfig(activeApp);
      if (result === "imported") {
        await queryClient.invalidateQueries({
          queryKey: ["providers", activeApp],
        });
        toast.success(t("provider.importCurrentDescription"));
      } else {
        toast.info(t("provider.importCurrentNoChanges"));
      }
    } catch (error) {
      toast.error(extractErrorMessage(error) || t("provider.noProviders"));
    }
  };

  const showWebDeepLinkImport =
    webMode &&
    (currentView === "providers" ||
      currentView === "prompts" ||
      currentView === "mcp" ||
      currentView === "skills" ||
      currentView === "skillsDiscovery");
  const activeProviderId = useMemo(() => {
    const target = proxyStatus?.active_targets?.find(
      (t) => t.app_type === activeApp,
    );
    return target?.provider_id;
  }, [proxyStatus?.active_targets, activeApp]);

  const { data, isLoading, refetch } = useProvidersQuery(activeApp, {
    isProxyRunning,
  });
  const { data: piCurrentState } = usePiCurrentState(activeApp === "pi");
  const providers = useMemo(() => data?.providers ?? {}, [data]);
  const currentProviderId = data?.currentProviderId ?? "";
  const isOpenClawView =
    activeApp === "openclaw" &&
    (currentView === "providers" ||
      currentView === "workspace" ||
      currentView === "sessions" ||
      currentView === "openclawEnv" ||
      currentView === "openclawTools" ||
      currentView === "openclawAgents");
  const { data: openclawHealthWarnings = [] } =
    useOpenClawHealth(isOpenClawView);
  const hasSkillsSupport = true;
  const hasSessionSupport =
    activeApp === "claude" ||
    activeApp === "codex" ||
    activeApp === "grokbuild" ||
    activeApp === "opencode" ||
    activeApp === "openclaw" ||
    activeApp === "gemini" ||
    activeApp === "hermes" ||
    activeApp === "pi";
  const hasUniversalProviderSupport =
    activeApp === "claude" || activeApp === "codex" || activeApp === "gemini";
  // Pi has no native MCP registry, so it never exposes the MCP panel.
  const hasMcpSupport = activeApp !== "pi";

  const {
    addProvider,
    updateProvider,
    switchProvider,
    deleteProvider,
    saveUsageScript,
    setAsDefaultModel,
  } = useProviderActions(
    activeApp,
    isProxyRunning,
    isProxyRunning && isCurrentAppTakeoverActive,
  );

  // Pi is additive: "enable" writes the provider node into models.json instead
  // of swapping a single live config, so it does not reuse switchProvider's
  // proxy/routing preflight or its "switched" notification.
  const handleEnablePiProvider = async (provider: Provider) => {
    try {
      await providersApi.switch(provider.id, "pi");
      await invalidatePiProviderCaches(queryClient);
      await providersApi.updateTrayMenu().catch((error) => {
        console.error(
          "Failed to update tray menu after enabling Pi provider",
          error,
        );
      });
      toast.success(
        t("pi.provider.enabled", {
          defaultValue: "已在 Pi 中启用",
        }),
        { closeButton: true },
      );
    } catch (error) {
      const detail = extractErrorMessage(error);
      toast.error(
        t("pi.provider.enableFailed", {
          defaultValue: "无法在 Pi 中启用此供应商",
        }),
        {
          description:
            translatePiProviderMutationError(detail, t) || detail || undefined,
          closeButton: true,
        },
      );
    }
  };

  const disableOmoMutation = useDisableCurrentOmo();
  const handleDisableOmo = () => {
    disableOmoMutation.mutate(undefined, {
      onSuccess: () => {
        toast.success(t("omo.disabled", { defaultValue: "OMO 已停用" }));
      },
      onError: (error: Error) => {
        toast.error(
          t("omo.disableFailed", {
            defaultValue: "停用 OMO 失败: {{error}}",
            error: extractErrorMessage(error),
          }),
        );
      },
    });
  };

  const disableOmoSlimMutation = useDisableCurrentOmoSlim();
  const handleDisableOmoSlim = () => {
    disableOmoSlimMutation.mutate(undefined, {
      onSuccess: () => {
        toast.success(t("omo.disabled", { defaultValue: "OMO 已停用" }));
      },
      onError: (error: Error) => {
        toast.error(
          t("omo.disableFailed", {
            defaultValue: "停用 OMO 失败: {{error}}",
            error: extractErrorMessage(error),
          }),
        );
      },
    });
  };

  useEffect(() => {
    let unsubscribe: (() => void) | undefined;
    let active = true;

    const setupListener = async () => {
      try {
        const off = await providersApi.onSwitched(
          async (event: ProviderSwitchEvent) => {
            if (event.appType === activeApp) {
              await refetch();
            }
            // Pi membership is derived from models.json, not from the provider
            // list payload, so its own cache must be refreshed independently
            // (e.g. a tray/other-window switch while another app is active).
            if (event.appType === "pi") {
              await invalidatePiProviderCaches(queryClient);
            }
          },
        );
        if (!active) {
          off();
          return;
        }
        unsubscribe = off;
      } catch (error) {
        console.error("[App] Failed to subscribe provider switch event", error);
      }
    };

    void setupListener();
    return () => {
      active = false;
      unsubscribe?.();
    };
  }, [activeApp, queryClient, refetch]);

  // Profile application can originate from this window, another Web client,
  // or the desktop tray. Refresh every derived cache changed by the snapshot;
  // takeover flags are written directly by the backend and do not pass through
  // the normal proxy mutations.
  useEffect(() => {
    let unsubscribe: (() => void) | undefined;
    let active = true;

    const setupListener = async () => {
      try {
        const off = await listen("profile-applied", async (event) => {
          const payload = (event.payload ?? {}) as {
            profileId?: string | null;
            scope?: "claude" | "codex";
          };
          const scopedInvalidations = payload.scope
            ? [
                queryClient.invalidateQueries({
                  queryKey: ["providers", payload.scope],
                }),
              ]
            : [];
          await Promise.all([
            queryClient.invalidateQueries({ queryKey: ["profiles"] }),
            queryClient.invalidateQueries({ queryKey: ["mcp", "all"] }),
            queryClient.invalidateQueries({ queryKey: ["skills"] }),
            queryClient.invalidateQueries({
              queryKey: proxyKeys.takeoverStatus,
            }),
            queryClient.invalidateQueries({ queryKey: proxyKeys.status }),
            ...scopedInvalidations,
          ]);
          if (payload.scope === activeApp) {
            await promptPanelRef.current?.reload();
          }
        });
        if (!active) {
          off();
          return;
        }
        unsubscribe = off;
      } catch (error) {
        console.error("[App] Failed to subscribe profile-applied event", error);
      }
    };

    void setupListener();
    return () => {
      active = false;
      unsubscribe?.();
    };
  }, [activeApp, queryClient]);

  useEffect(() => {
    let unsubscribe: (() => void) | undefined;
    let active = true;

    const setupListener = async () => {
      try {
        const off = await listen("universal-provider-synced", async () => {
          await queryClient.invalidateQueries({ queryKey: ["providers"] });
          // 让 UniversalProviderPanel 的 React Query 同步刷新（M42）
          await queryClient.invalidateQueries({
            queryKey: universalProviderKeys.all,
          });
          try {
            await providersApi.updateTrayMenu();
          } catch (error) {
            console.error("[App] Failed to update tray menu", error);
          }
        });
        if (!active) {
          off();
          return;
        }
        unsubscribe = off;
      } catch (error) {
        console.error(
          "[App] Failed to subscribe universal-provider-synced event",
          error,
        );
      }
    };

    void setupListener();
    return () => {
      active = false;
      unsubscribe?.();
    };
  }, [queryClient]);

  useEffect(() => {
    let unsubscribe: (() => void) | undefined;
    let active = true;

    const setupListener = async () => {
      try {
        const off = await listen(
          "webdav-sync-status-updated",
          async (event) => {
            const payload = (event.payload ?? {}) as SyncStatusUpdatedPayload;
            await queryClient.invalidateQueries({ queryKey: ["settings"] });

            if (payload.source !== "auto" || payload.status !== "error") {
              return;
            }

            toast.error(
              t("settings.webdavSync.autoSyncFailedToast", {
                error: payload.error || t("common.unknown"),
              }),
            );
          },
        );
        if (!active) {
          off();
          return;
        }
        unsubscribe = off;
      } catch (error) {
        console.error(
          "[App] Failed to subscribe webdav-sync-status-updated event",
          error,
        );
      }
    };

    void setupListener();
    return () => {
      active = false;
      unsubscribe?.();
    };
  }, [queryClient, t]);

  useEffect(() => {
    let unsubscribe: (() => void) | undefined;
    let active = true;

    const setupListener = async () => {
      try {
        const off = await listen("s3-sync-status-updated", async (event) => {
          const payload = (event.payload ?? {}) as SyncStatusUpdatedPayload;
          await queryClient.invalidateQueries({ queryKey: ["settings"] });

          if (payload.source !== "auto" || payload.status !== "error") {
            return;
          }

          toast.error(
            t("settings.s3Sync.autoSyncFailedToast", {
              error: payload.error || t("common.unknown"),
            }),
          );
        });
        if (!active) {
          off();
          return;
        }
        unsubscribe = off;
      } catch (error) {
        console.error(
          "[App] Failed to subscribe s3-sync-status-updated event",
          error,
        );
      }
    };

    void setupListener();
    return () => {
      active = false;
      unsubscribe?.();
    };
  }, [queryClient, t]);

  // Listen for proxy-official-warning: warn when takeover is enabled with an official provider
  useEffect(() => {
    let unsubscribe: (() => void) | undefined;
    let active = true;

    const setup = async () => {
      const off = await listen("proxy-official-warning", (event) => {
        const { providerName } = event.payload as {
          appType: string;
          providerName: string;
        };
        toast.warning(
          t("notifications.proxyOfficialWarning", {
            name: providerName,
            defaultValue: `当前供应商 ${providerName} 是官方供应商，建议切换到第三方供应商后再使用代理接管`,
          }),
          { duration: 8000 },
        );
      });
      if (!active) {
        off();
        return;
      }
      unsubscribe = off;
    };

    void setup();
    return () => {
      active = false;
      unsubscribe?.();
    };
  }, [t]);

  useEffect(() => {
    if (webMode || !useAppWindowControls) {
      setIsWindowMaximized(false);
      return;
    }

    let active = true;
    let unlistenResize: (() => void) | undefined;

    const setupWindowStateSync = async () => {
      try {
        const currentWindow = await getDesktopCurrentWindow();
        const syncWindowMaximizedState = async () => {
          const maximized = await currentWindow.isMaximized();
          if (active) {
            setIsWindowMaximized(maximized);
          }
        };

        await syncWindowMaximizedState();
        unlistenResize = await currentWindow.onResized(() => {
          void syncWindowMaximizedState();
        });
      } catch (error) {
        console.error("[App] Failed to sync window maximized state", error);
      }
    };

    void setupWindowStateSync();
    return () => {
      active = false;
      unlistenResize?.();
    };
  }, [useAppWindowControls, webMode]);

  useEffect(() => {
    // settingsData 未加载时跳过，避免用 fallback false 覆盖 Rust 侧已设好的装饰状态
    if (webMode || !settingsData) return;

    const syncWindowDecorations = async () => {
      try {
        const currentWindow = await getDesktopCurrentWindow();
        await currentWindow.setDecorations(!useAppWindowControls);
      } catch (error) {
        console.error("[App] Failed to update window decorations", error);
      }
    };

    void syncWindowDecorations();
  }, [settingsData, useAppWindowControls, webMode]);

  useEffect(() => {
    const checkEnvOnStartup = async () => {
      try {
        const allConflicts = await checkAllEnvConflicts();
        const flatConflicts = Object.values(allConflicts).flat();

        if (flatConflicts.length > 0) {
          setEnvConflicts(flatConflicts);
          const dismissed = sessionStorage.getItem("env_banner_dismissed");
          if (!dismissed) {
            setShowEnvBanner(true);
          }
        }
      } catch (error) {
        console.error(
          "[App] Failed to check environment conflicts on startup:",
          error,
        );
      }
    };

    checkEnvOnStartup();
  }, []);

  useEffect(() => {
    const checkMigration = async () => {
      try {
        const migrated = await invoke<boolean>("get_migration_result");
        if (migrated) {
          toast.success(
            t("migration.success", { defaultValue: "配置迁移成功" }),
            { closeButton: true },
          );
        }
      } catch (error) {
        console.error("[App] Failed to check migration result:", error);
      }
    };

    checkMigration();
  }, [t]);

  useEffect(() => {
    const checkSkillsMigration = async () => {
      try {
        const result = await invoke<{ count: number; error?: string } | null>(
          "get_skills_migration_result",
        );
        if (result?.error) {
          toast.error(t("migration.skillsFailed"), {
            description: t("migration.skillsFailedDescription"),
            closeButton: true,
          });
          console.error("[App] Skills SSOT migration failed:", result.error);
          return;
        }
        if (result && result.count > 0) {
          toast.success(t("migration.skillsSuccess", { count: result.count }), {
            closeButton: true,
          });
          await queryClient.invalidateQueries({ queryKey: ["skills"] });
        }
      } catch (error) {
        console.error("[App] Failed to check skills migration result:", error);
      }
    };

    checkSkillsMigration();
  }, [t, queryClient]);

  useEffect(() => {
    const checkEnvOnSwitch = async () => {
      try {
        const conflicts = await checkEnvConflicts(activeApp);

        if (conflicts.length > 0) {
          setEnvConflicts((prev) => {
            const existingKeys = new Set(
              prev.map((c) => `${c.varName}:${c.sourcePath}`),
            );
            const newConflicts = conflicts.filter(
              (c) => !existingKeys.has(`${c.varName}:${c.sourcePath}`),
            );
            return [...prev, ...newConflicts];
          });
          const dismissed = sessionStorage.getItem("env_banner_dismissed");
          if (!dismissed) {
            setShowEnvBanner(true);
          }
        }
      } catch (error) {
        console.error(
          "[App] Failed to check environment conflicts on app switch:",
          error,
        );
      }
    };

    checkEnvOnSwitch();
  }, [activeApp]);

  const currentViewRef = useRef(currentView);
  const managementBusy =
    mcpManagementBusy || skillsNavigationBusy || promptNavigationBusy;
  const managementBusyRef = useRef(false);
  managementBusyRef.current = managementBusy;

  useEffect(() => {
    currentViewRef.current = currentView;
  }, [currentView]);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "," && (event.metaKey || event.ctrlKey)) {
        if (managementBusyRef.current) {
          event.preventDefault();
          return;
        }
        event.preventDefault();
        setCurrentView("settings");
        return;
      }

      if (event.key !== "Escape" || event.defaultPrevented) return;

      if (document.body.style.overflow === "hidden") return;

      const view = currentViewRef.current;
      if (view === "providers") return;
      if (managementBusyRef.current) return;

      if (isTextEditableTarget(event.target)) return;

      event.preventDefault();
      setCurrentView(view === "skillsDiscovery" ? "skills" : "providers");
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => {
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, []);

  const [launchDashboardOpen, setLaunchDashboardOpen] = useState(false);
  const openHermesWebUI = useOpenHermesWebUI(() =>
    setLaunchDashboardOpen(true),
  );

  const handleOpenWebsite = async (url: string) => {
    try {
      await settingsApi.openExternal(url);
    } catch (error) {
      const detail =
        extractErrorMessage(error) ||
        t("notifications.openLinkFailed", {
          defaultValue: "链接打开失败",
        });
      toast.error(detail);
    }
  };

  const handleEditProvider = async ({
    provider,
    originalId,
  }: {
    provider: Provider;
    originalId?: string;
  }) => {
    await updateProvider(provider, originalId);
    setEditingProvider(null);
  };

  const handleConfirmAction = async () => {
    if (!confirmAction) return;
    const { provider, action } = confirmAction;

    if (action === "remove") {
      // Remove from live config only (for additive mode apps)
      // Does NOT delete from database - provider remains in the list
      try {
        await providersApi.removeFromLiveConfig(provider.id, activeApp);
      } catch (error) {
        const detail = extractErrorMessage(error);
        const description =
          activeApp === "pi"
            ? translatePiProviderMutationError(detail, t) || detail
            : detail;
        if (activeApp === "pi") {
          // A rejected write may still mean our cached view is stale.
          void invalidatePiProviderCaches(queryClient).catch(() => undefined);
        }
        toast.error(t("notifications.removeFromConfigFailed"), {
          description: description || t("common.unknown"),
          closeButton: true,
        });
        // Keep the confirm dialog open so the user can retry after fixing the
        // underlying config conflict.
        return;
      }
      // Invalidate queries to refresh live/config/current state
      if (activeApp === "opencode") {
        await queryClient.invalidateQueries({
          queryKey: ["opencodeLiveProviderIds"],
        });
      } else if (activeApp === "openclaw") {
        await queryClient.invalidateQueries({
          queryKey: openclawKeys.liveProviderIds,
        });
        await queryClient.invalidateQueries({
          queryKey: openclawKeys.health,
        });
      } else if (activeApp === "hermes") {
        await invalidateHermesProviderCaches(queryClient, provider.id);
      } else if (activeApp === "pi") {
        await invalidatePiProviderCaches(queryClient);
      }
      toast.success(
        activeApp === "pi"
          ? t("pi.provider.removed", {
              defaultValue: "已从 Pi 移除",
            })
          : t("notifications.removeFromConfigSuccess", {
              defaultValue: "已从配置移除",
            }),
        { closeButton: true },
      );
    } else {
      await deleteProvider(provider.id);
    }
    setConfirmAction(null);
  };

  const handleDuplicateProvider = async (provider: Provider) => {
    const newSortIndex =
      provider.sortIndex !== undefined ? provider.sortIndex + 1 : undefined;

    const duplicatedProvider: Omit<Provider, "id" | "createdAt"> & {
      providerKey?: string;
      addToLive?: boolean;
    } = {
      name: `${provider.name} copy`,
      settingsConfig: JSON.parse(JSON.stringify(provider.settingsConfig)), // 深拷贝
      websiteUrl: provider.websiteUrl,
      category: provider.category,
      sortIndex: newSortIndex, // 复制原 sortIndex + 1
      meta: provider.meta
        ? JSON.parse(JSON.stringify(provider.meta))
        : undefined, // 深拷贝
      icon: provider.icon,
      iconColor: provider.iconColor,
    };

    if (
      activeApp === "opencode" ||
      activeApp === "openclaw" ||
      activeApp === "hermes" ||
      activeApp === "pi"
    ) {
      let liveProviderIds: string[] = [];
      try {
        liveProviderIds =
          activeApp === "opencode"
            ? await queryClient.ensureQueryData({
                queryKey: ["opencodeLiveProviderIds"],
                queryFn: () => providersApi.getOpenCodeLiveProviderIds(),
              })
            : activeApp === "openclaw"
              ? await queryClient.ensureQueryData({
                  queryKey: openclawKeys.liveProviderIds,
                  queryFn: () => providersApi.getOpenClawLiveProviderIds(),
                })
              : activeApp === "hermes"
                ? await queryClient.ensureQueryData({
                    queryKey: hermesKeys.liveProviderIds,
                    queryFn: () => providersApi.getHermesLiveProviderIds(),
                  })
                : (
                    await queryClient.ensureQueryData({
                      queryKey: piKeys.currentState,
                      queryFn: () => piApi.getCurrentState(),
                    })
                  ).enabledProviderIds;
      } catch (error) {
        console.error(
          "[App] Failed to load live provider IDs for duplication",
          error,
        );
        const errorMessage = extractErrorMessage(error);
        toast.error(
          t("provider.duplicateLiveIdsLoadFailed", {
            defaultValue: "读取配置中的供应商标识失败，请先修复配置后再试",
          }) + (errorMessage ? `: ${errorMessage}` : ""),
        );
        return;
      }
      const existingKeys = Array.from(
        new Set([...Object.keys(providers), ...liveProviderIds]),
      );
      duplicatedProvider.providerKey = generateUniqueProviderCopyKey(
        provider.id,
        existingKeys,
      );
      duplicatedProvider.addToLive = false;
    }

    if (provider.sortIndex !== undefined) {
      const updates = Object.values(providers)
        .filter(
          (p) =>
            p.sortIndex !== undefined &&
            p.sortIndex >= newSortIndex! &&
            p.id !== provider.id,
        )
        .map((p) => ({
          id: p.id,
          sortIndex: p.sortIndex! + 1,
        }));

      if (updates.length > 0) {
        try {
          await providersApi.updateSortOrder(updates, activeApp);
        } catch (error) {
          console.error("[App] Failed to update sort order", error);
          toast.error(
            t("provider.sortUpdateFailed", {
              defaultValue: "排序更新失败",
            }),
          );
          return; // 如果排序更新失败，不继续添加
        }
      }
    }

    await addProvider(duplicatedProvider);
  };

  const confirmActionMessage = useMemo(() => {
    if (!confirmAction) return "";

    const message =
      confirmAction.action === "remove"
        ? t("confirm.removeProviderMessage", {
            name: confirmAction.provider.name,
          })
        : t("confirm.deleteProviderMessage", {
            name: confirmAction.provider.name,
          });
    // Pi keeps its own global default; warn before we pull it out from under it.
    const isPiGlobalDefault =
      activeApp === "pi" &&
      piCurrentState?.defaultProviderId === confirmAction.provider.id;

    return isPiGlobalDefault
      ? `${message}\n\n${t("confirm.piDefaultProviderWarning")}`
      : message;
  }, [activeApp, confirmAction, piCurrentState?.defaultProviderId, t]);

  const handleOpenTerminal = async (provider: Provider) => {
    try {
      const selectedDir = await settingsApi.pickDirectory();
      if (!selectedDir) {
        return;
      }

      await providersApi.openTerminal(provider.id, activeApp, {
        cwd: selectedDir,
      });
      toast.success(
        t("provider.terminalOpened", {
          defaultValue: "终端已打开",
        }),
      );
    } catch (error) {
      console.error("[App] Failed to open terminal", error);
      const errorMessage = extractErrorMessage(error);
      toast.error(
        t("provider.terminalOpenFailed", {
          defaultValue: "打开终端失败",
        }) + (errorMessage ? `: ${errorMessage}` : ""),
      );
    }
  };

  const handleImportSuccess = async () => {
    try {
      await queryClient.invalidateQueries({
        queryKey: ["providers"],
        refetchType: "all",
      });
      await queryClient.refetchQueries({
        queryKey: ["providers"],
        type: "all",
      });
    } catch (error) {
      console.error("[App] Failed to refresh providers after import", error);
      await refetch();
    }
    try {
      await providersApi.updateTrayMenu();
    } catch (error) {
      console.error("[App] Failed to refresh tray menu", error);
    }
  };

  const notifyWindowControlError = (error: unknown) => {
    toast.error(
      t("notifications.windowControlFailed", {
        defaultValue: "窗口控制失败：{{error}}",
        error: extractErrorMessage(error),
      }),
    );
  };

  const handleWindowMinimize = async () => {
    if (webMode) return;
    try {
      const currentWindow = await getDesktopCurrentWindow();
      await currentWindow.minimize();
    } catch (error) {
      console.error("[App] Failed to minimize window", error);
      notifyWindowControlError(error);
    }
  };

  const handleWindowToggleMaximize = async () => {
    if (webMode) return;
    try {
      const currentWindow = await getDesktopCurrentWindow();
      await currentWindow.toggleMaximize();
      setIsWindowMaximized(await currentWindow.isMaximized());
    } catch (error) {
      console.error("[App] Failed to toggle maximize", error);
      notifyWindowControlError(error);
    }
  };

  const handleWindowClose = async () => {
    if (webMode) return;
    try {
      const currentWindow = await getDesktopCurrentWindow();
      await currentWindow.close();
    } catch (error) {
      console.error("[App] Failed to close window", error);
      notifyWindowControlError(error);
    }
  };

  const renderContent = () => {
    const content = (() => {
      switch (currentView) {
        case "settings":
          return (
            <SettingsPage
              open={true}
              onOpenChange={() => setCurrentView("providers")}
              onImportSuccess={handleImportSuccess}
              defaultTab={settingsDefaultTab}
            />
          );
        case "prompts":
          return (
            <PromptPanel
              ref={promptPanelRef}
              open={true}
              onOpenChange={() => setCurrentView("providers")}
              appId={activeApp}
              onInteractionBlockedChange={setPromptManagementBusy}
              onNavigationBlockedChange={setPromptNavigationBusy}
              onPrimaryActionChange={setPromptPrimaryAction}
            />
          );
        case "hermesMemory":
          return <HermesMemoryPanel />;
        case "skills":
          return (
            <UnifiedSkillsPanel
              ref={unifiedSkillsPanelRef}
              onOpenDiscovery={() => setCurrentView("skillsDiscovery")}
              onInteractionBlockedChange={setSkillsManagementBusy}
              onNavigationBlockedChange={setSkillsNavigationBusy}
              onCheckUpdatesStateChange={setSkillsCheckUpdatesState}
              currentApp={activeApp === "openclaw" ? "claude" : activeApp}
            />
          );
        case "skillsDiscovery":
          return (
            <SkillsPage
              ref={skillsPageRef}
              initialApp={activeApp === "openclaw" ? "claude" : activeApp}
            />
          );
        case "mcp":
          return (
            <UnifiedMcpPanel
              ref={mcpPanelRef}
              onOpenChange={() => setCurrentView("providers")}
              onInteractionBlockedChange={setMcpManagementBusy}
            />
          );
        case "agents":
          return (
            <AgentsPanel onOpenChange={() => setCurrentView("providers")} />
          );
        case "universal":
          return (
            <div className="px-6 pt-4">
              <UniversalProviderPanel />
            </div>
          );

        case "sessions":
          return <SessionManagerPage key={activeApp} appId={activeApp} />;
        case "workspace":
          return <WorkspaceFilesPanel />;
        case "openclawEnv":
          return <EnvPanel />;
        case "openclawTools":
          return <ToolsPanel />;
        case "openclawAgents":
          return <AgentsDefaultsPanel />;
        default:
          return (
            <div className="px-6 flex flex-col flex-1 min-h-0 overflow-hidden">
              <div className="flex-1 overflow-y-auto overflow-x-hidden pb-12 px-1">
                <AnimatePresence mode="wait">
                  <motion.div
                    key={activeApp}
                    initial={{ opacity: 0 }}
                    animate={{ opacity: 1 }}
                    exit={{ opacity: 0 }}
                    transition={{ duration: 0.15 }}
                    className="space-y-4"
                  >
                    <ProviderList
                      providers={providers}
                      currentProviderId={currentProviderId}
                      appId={activeApp}
                      isLoading={isLoading}
                      isProxyRunning={isProxyRunning}
                      isProxyTakeover={
                        isProxyRunning && isCurrentAppTakeoverActive
                      }
                      activeProviderId={activeProviderId}
                      onSwitch={
                        activeApp === "pi"
                          ? handleEnablePiProvider
                          : switchProvider
                      }
                      onEdit={(provider) => {
                        setEditingProvider(provider);
                      }}
                      onDelete={(provider) =>
                        setConfirmAction({ provider, action: "delete" })
                      }
                      onRemoveFromConfig={
                        activeApp === "opencode" ||
                        activeApp === "openclaw" ||
                        activeApp === "hermes" ||
                        activeApp === "pi"
                          ? (provider) =>
                              setConfirmAction({ provider, action: "remove" })
                          : undefined
                      }
                      onDisableOmo={
                        activeApp === "opencode" ? handleDisableOmo : undefined
                      }
                      onDisableOmoSlim={
                        activeApp === "opencode"
                          ? handleDisableOmoSlim
                          : undefined
                      }
                      onDuplicate={handleDuplicateProvider}
                      onConfigureUsage={setUsageProvider}
                      onOpenWebsite={handleOpenWebsite}
                      onOpenTerminal={
                        activeApp === "claude" ? handleOpenTerminal : undefined
                      }
                      onCreate={() => setIsAddOpen(true)}
                      onSetAsDefault={
                        activeApp === "openclaw"
                          ? setAsDefaultModel
                          : activeApp === "hermes"
                            ? switchProvider
                            : undefined
                      }
                    />
                  </motion.div>
                </AnimatePresence>
              </div>
            </div>
          );
      }
    })();

    return (
      <AnimatePresence mode="wait">
        <motion.div
          key={currentView}
          className="flex-1 min-h-0"
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          transition={{ duration: 0.2 }}
        >
          {content}
        </motion.div>
      </AnimatePresence>
    );
  };

  return (
    <div
      className="flex flex-col h-screen overflow-hidden bg-background text-foreground selection:bg-primary/30 pb-4"
      style={{ overflowX: "hidden", paddingTop: contentTopOffset }}
    >
      {showDesktopTitlebar && (
        <div
          className="fixed top-0 left-0 right-0 z-[70] flex items-center justify-end px-2"
          data-tauri-drag-region
          style={{ WebkitAppRegion: "drag", height: dragBarHeight } as any}
        >
          {useAppWindowControls && (
            <div
              className="flex items-center gap-1"
              style={{ WebkitAppRegion: "no-drag" } as any}
            >
              <Button
                variant="ghost"
                size="icon"
                onClick={() => void handleWindowMinimize()}
                title={t("header.windowMinimize")}
                className="h-7 w-7"
              >
                <Minus className="w-4 h-4" />
              </Button>
              <Button
                variant="ghost"
                size="icon"
                onClick={() => void handleWindowToggleMaximize()}
                title={
                  isWindowMaximized
                    ? t("header.windowRestore")
                    : t("header.windowMaximize")
                }
                className="h-7 w-7"
              >
                {isWindowMaximized ? (
                  <Minimize2 className="w-4 h-4" />
                ) : (
                  <Maximize2 className="w-4 h-4" />
                )}
              </Button>
              <Button
                variant="ghost"
                size="icon"
                onClick={() => void handleWindowClose()}
                title={t("header.windowClose")}
                className="h-7 w-7 hover:bg-red-500/15 hover:text-red-500"
              >
                <X className="w-4 h-4" />
              </Button>
            </div>
          )}
        </div>
      )}
      {showEnvBanner && envConflicts.length > 0 && (
        <EnvWarningBanner
          conflicts={envConflicts}
          onDismiss={() => {
            setShowEnvBanner(false);
            sessionStorage.setItem("env_banner_dismissed", "true");
          }}
          onDeleted={async () => {
            try {
              const allConflicts = await checkAllEnvConflicts();
              const flatConflicts = Object.values(allConflicts).flat();
              setEnvConflicts(flatConflicts);
              if (flatConflicts.length === 0) {
                setShowEnvBanner(false);
              }
            } catch (error) {
              console.error(
                "[App] Failed to re-check conflicts after deletion:",
                error,
              );
            }
          }}
        />
      )}

      <header
        className="fixed z-50 w-full transition-all duration-300 bg-background/80 backdrop-blur-md"
        {...DRAG_REGION_ATTR}
        style={
          {
            ...DRAG_REGION_STYLE,
            top: dragBarHeight,
            height: HEADER_HEIGHT,
          } as any
        }
      >
        <div
          className="flex h-full items-center justify-between gap-2 px-6"
          {...DRAG_REGION_ATTR}
          style={{ ...DRAG_REGION_STYLE } as any}
        >
          <div
            className="flex items-center gap-1"
            style={{ WebkitAppRegion: "no-drag" } as any}
          >
            {currentView !== "providers" ? (
              <div className="flex items-center gap-2">
                <Button
                  variant="outline"
                  size="icon"
                  disabled={managementBusy}
                  onClick={() =>
                    setCurrentView(
                      currentView === "skillsDiscovery"
                        ? "skills"
                        : "providers",
                    )
                  }
                  className={cn(
                    "mr-2 rounded-lg",
                    managementBusy && "disabled:opacity-100",
                  )}
                >
                  <ArrowLeft className="w-4 h-4" />
                </Button>
                <h1 className="text-lg font-semibold">
                  {currentView === "settings" && t("settings.title")}
                  {currentView === "prompts" &&
                    t("prompts.title", { appName: t(`apps.${activeApp}`) })}
                  {currentView === "skills" && t("skills.title")}
                  {currentView === "skillsDiscovery" && t("skills.title")}
                  {currentView === "mcp" && t("mcp.unifiedPanel.title")}
                  {currentView === "agents" && t("agents.title")}
                  {currentView === "universal" &&
                    t("universalProvider.title", {
                      defaultValue: "统一供应商",
                    })}
                  {currentView === "sessions" && t("sessionManager.title")}
                  {currentView === "workspace" && t("workspace.title")}
                  {currentView === "openclawEnv" && t("openclaw.env.title")}
                  {currentView === "openclawTools" && t("openclaw.tools.title")}
                  {currentView === "openclawAgents" &&
                    t("openclaw.agents.title")}
                  {currentView === "hermesMemory" && t("hermes.memory.title")}
                </h1>
              </div>
            ) : (
              <div className="flex items-center gap-2">
                <RoutingActivationBrand
                  active={isProxyRunning && isCurrentAppTakeoverActive}
                  contextKey={activeApp}
                  ready={
                    proxyStatus !== undefined && takeoverStatus !== undefined
                  }
                />
                <Button
                  variant="ghost"
                  size="icon"
                  onClick={() => {
                    setSettingsDefaultTab("general");
                    setCurrentView("settings");
                  }}
                  title={t("common.settings")}
                  className="hover:bg-black/5 dark:hover:bg-white/5"
                >
                  <Settings className="w-4 h-4" />
                </Button>
                <UpdateBadge
                  onClick={() => {
                    setSettingsDefaultTab("about");
                    setCurrentView("settings");
                  }}
                />
                {isCurrentAppTakeoverActive && (
                  <Button
                    variant="ghost"
                    size="icon"
                    onClick={() => {
                      setSettingsDefaultTab("usage");
                      setCurrentView("settings");
                    }}
                    title={t("usage.title", {
                      defaultValue: "使用统计",
                    })}
                    className="hover:bg-black/5 dark:hover:bg-white/5"
                  >
                    <BarChart2 className="w-4 h-4" />
                  </Button>
                )}
              </div>
            )}
          </div>

          <div className="flex flex-1 min-w-0 items-center justify-end gap-1.5">
            {currentView === "providers" &&
              activeApp !== "opencode" &&
              activeApp !== "openclaw" &&
              activeApp !== "hermes" &&
              activeApp !== "pi" && (
                <div
                  className="flex shrink-0 items-center gap-1.5"
                  style={{ WebkitAppRegion: "no-drag" } as any}
                >
                  {settingsData?.enableLocalProxy && (
                    <ProxyToggle activeApp={activeApp} />
                  )}
                  {settingsData?.enableFailoverToggle && (
                    <FailoverToggle activeApp={activeApp} />
                  )}
                </div>
              )}
            {currentView === "providers" &&
              (settingsData?.showProfileSwitcher ?? true) && (
                <div
                  className="flex shrink-0 items-center"
                  style={{ WebkitAppRegion: "no-drag" } as any}
                >
                  <ProfileSwitcher activeApp={activeApp} />
                </div>
              )}
            {/* 弹性中段：空间不足时由 AppSwitcher 自行收纳溢出应用；
                justify-end + overflow-hidden 只裁剪 resize 瞬间的过渡帧 */}
            <div className="flex flex-1 min-w-0 items-center justify-end overflow-hidden py-4">
              {currentView === "providers" && (
                <AppSwitcher
                  activeApp={activeApp}
                  onSwitch={setActiveApp}
                  visibleApps={visibleApps}
                />
              )}
            </div>
            {/* 固定右端：主操作（添加供应商等）shrink-0，任何配置下不被挤出 */}
            <div className="flex shrink-0 items-center py-4">
              <div
                className="flex shrink-0 items-center gap-1.5"
                style={{ WebkitAppRegion: "no-drag" } as any}
              >
                {showWebDeepLinkImport && (
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() =>
                      deepLinkImportDialogRef.current?.openManualImport()
                    }
                    className="hover:bg-black/5 dark:hover:bg-white/5"
                  >
                    <Link2 className="w-4 h-4 mr-2" />
                    {t("deeplink.pasteImport")}
                  </Button>
                )}
                {currentView === "prompts" && promptPrimaryAction && (
                  <>
                    {promptPrimaryAction === "prompt" && (
                      <Button
                        variant="ghost"
                        size="sm"
                        disabled={promptManagementBusy}
                        onClick={() => promptPanelRef.current?.openImport()}
                        className="hover:bg-black/5 disabled:opacity-100 dark:hover:bg-white/5"
                      >
                        <Download className="w-4 h-4 mr-2" />
                        {t("prompts.import")}
                      </Button>
                    )}
                    <Button
                      variant="ghost"
                      size="sm"
                      disabled={promptManagementBusy}
                      onClick={() => promptPanelRef.current?.openAdd()}
                      className="hover:bg-black/5 disabled:opacity-100 dark:hover:bg-white/5"
                    >
                      <Plus className="w-4 h-4 mr-2" />
                      {t(
                        promptPrimaryAction === "template"
                          ? "pi.prompts.newTemplate"
                          : "prompts.add",
                      )}
                    </Button>
                  </>
                )}
                {currentView === "mcp" && (
                  <>
                    <Button
                      variant="ghost"
                      size="sm"
                      disabled={mcpManagementBusy}
                      onClick={() => mcpPanelRef.current?.openImport()}
                      className="hover:bg-black/5 disabled:opacity-100 dark:hover:bg-white/5"
                    >
                      <Download className="w-4 h-4 mr-2" />
                      {t("mcp.importExisting")}
                    </Button>
                    <Button
                      variant="ghost"
                      size="sm"
                      disabled={mcpManagementBusy}
                      onClick={() => mcpPanelRef.current?.openAdd()}
                      className="hover:bg-black/5 disabled:opacity-100 dark:hover:bg-white/5"
                    >
                      <Plus className="w-4 h-4 mr-2" />
                      {t("mcp.addMcp")}
                    </Button>
                  </>
                )}
                {currentView === "skills" && (
                  <>
                    <Button
                      variant="ghost"
                      size="sm"
                      disabled={
                        skillsManagementBusy ||
                        skillsCheckUpdatesState.isChecking ||
                        !skillsCheckUpdatesState.hasSkills
                      }
                      onClick={() =>
                        unifiedSkillsPanelRef.current?.checkUpdates()
                      }
                      className={cn(
                        "hover:bg-black/5 dark:hover:bg-white/5",
                        skillsManagementBusy && "disabled:opacity-100",
                      )}
                    >
                      {skillsCheckUpdatesState.isChecking ? (
                        <Loader2 className="w-4 h-4 mr-2 animate-spin" />
                      ) : (
                        <RefreshCw className="w-4 h-4 mr-2" />
                      )}
                      {skillsCheckUpdatesState.isChecking
                        ? t("skills.checkingUpdates")
                        : t("skills.checkUpdates")}
                    </Button>
                    <Button
                      variant="ghost"
                      size="sm"
                      disabled={skillsManagementBusy}
                      onClick={() =>
                        unifiedSkillsPanelRef.current?.openRestoreFromBackup()
                      }
                      className="hover:bg-black/5 disabled:opacity-100 dark:hover:bg-white/5"
                    >
                      <History className="w-4 h-4 mr-2" />
                      {t("skills.restoreFromBackup.button")}
                    </Button>
                    <Button
                      variant="ghost"
                      size="sm"
                      disabled={skillsManagementBusy}
                      onClick={() =>
                        unifiedSkillsPanelRef.current?.openInstallFromZip()
                      }
                      className="hover:bg-black/5 disabled:opacity-100 dark:hover:bg-white/5"
                    >
                      <FolderArchive className="w-4 h-4 mr-2" />
                      {t("skills.installFromZip.button")}
                    </Button>
                    <Button
                      variant="ghost"
                      size="sm"
                      disabled={skillsManagementBusy}
                      onClick={() =>
                        unifiedSkillsPanelRef.current?.openImport()
                      }
                      className="relative hover:bg-black/5 disabled:opacity-100 dark:hover:bg-white/5"
                    >
                      <Download className="w-4 h-4 mr-2" />
                      {t("skills.import")}
                    </Button>
                    <Button
                      variant="ghost"
                      size="sm"
                      disabled={skillsManagementBusy}
                      onClick={() =>
                        unifiedSkillsPanelRef.current?.openDiscovery()
                      }
                      className="hover:bg-black/5 disabled:opacity-100 dark:hover:bg-white/5"
                    >
                      <Search className="w-4 h-4 mr-2" />
                      {t("skills.discover")}
                    </Button>
                  </>
                )}
                {currentView === "skillsDiscovery" && (
                  <>
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => skillsPageRef.current?.refresh()}
                      className="hover:bg-black/5 dark:hover:bg-white/5"
                    >
                      <RefreshCw className="w-4 h-4 mr-2" />
                      {t("skills.refresh")}
                    </Button>
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => skillsPageRef.current?.openRepoManager()}
                      className="hover:bg-black/5 dark:hover:bg-white/5"
                    >
                      <Settings className="w-4 h-4 mr-2" />
                      {t("skills.repoManager")}
                    </Button>
                  </>
                )}
                {currentView === "providers" && (
                  <>
                    <div className="flex items-center gap-1 p-1 bg-muted rounded-xl">
                      <AnimatePresence mode="wait">
                        <motion.div
                          key={
                            activeApp === "openclaw"
                              ? "openclaw"
                              : activeApp === "hermes"
                                ? "hermes"
                                : activeApp === "grokbuild"
                                  ? "grokbuild"
                                  : "default"
                          }
                          className="flex items-center gap-1"
                          initial={{ opacity: 0 }}
                          animate={{ opacity: 1 }}
                          exit={{ opacity: 0 }}
                          transition={{ duration: 0.15 }}
                        >
                          {activeApp === "hermes" ? (
                            <>
                              <Button
                                variant="ghost"
                                size="sm"
                                onClick={() => setCurrentView("skills")}
                                className="text-muted-foreground hover:text-foreground hover:bg-black/5 dark:hover:bg-white/5 w-8 px-2"
                                title={t("skills.manage")}
                              >
                                <Wrench className="w-4 h-4" />
                              </Button>
                              <Button
                                variant="ghost"
                                size="sm"
                                onClick={() => setCurrentView("hermesMemory")}
                                className="text-muted-foreground hover:text-foreground hover:bg-black/5 dark:hover:bg-white/5 w-8 px-2"
                                title={t("hermes.memory.title")}
                              >
                                <Brain className="w-4 h-4" />
                              </Button>
                              <Button
                                variant="ghost"
                                size="sm"
                                onClick={() => void openHermesWebUI()}
                                className="text-muted-foreground hover:text-foreground hover:bg-black/5 dark:hover:bg-white/5 w-8 px-2"
                                title={t("hermes.webui.open")}
                              >
                                <LayoutDashboard className="w-4 h-4" />
                              </Button>
                              <Button
                                variant="ghost"
                                size="sm"
                                onClick={() => setCurrentView("mcp")}
                                className="text-muted-foreground hover:text-foreground hover:bg-black/5 dark:hover:bg-white/5 w-8 px-2"
                                title={t("mcp.title")}
                              >
                                <McpIcon size={16} />
                              </Button>
                            </>
                          ) : activeApp === "openclaw" ? (
                            <>
                              <Button
                                variant="ghost"
                                size="sm"
                                onClick={() => setCurrentView("workspace")}
                                className="text-muted-foreground hover:text-foreground hover:bg-black/5 dark:hover:bg-white/5 w-8 px-2"
                                title={t("workspace.manage")}
                              >
                                <FolderOpen className="w-4 h-4" />
                              </Button>
                              <Button
                                variant="ghost"
                                size="sm"
                                onClick={() => setCurrentView("openclawEnv")}
                                className="text-muted-foreground hover:text-foreground hover:bg-black/5 dark:hover:bg-white/5 w-8 px-2"
                                title={t("openclaw.env.title")}
                              >
                                <KeyRound className="w-4 h-4" />
                              </Button>
                              <Button
                                variant="ghost"
                                size="sm"
                                onClick={() => setCurrentView("openclawTools")}
                                className="text-muted-foreground hover:text-foreground hover:bg-black/5 dark:hover:bg-white/5 w-8 px-2"
                                title={t("openclaw.tools.title")}
                              >
                                <Shield className="w-4 h-4" />
                              </Button>
                              <Button
                                variant="ghost"
                                size="sm"
                                onClick={() => setCurrentView("openclawAgents")}
                                className="text-muted-foreground hover:text-foreground hover:bg-black/5 dark:hover:bg-white/5 w-8 px-2"
                                title={t("openclaw.agents.title")}
                              >
                                <Cpu className="w-4 h-4" />
                              </Button>
                              <Button
                                variant="ghost"
                                size="sm"
                                onClick={() => setCurrentView("sessions")}
                                className="text-muted-foreground hover:text-foreground hover:bg-black/5 dark:hover:bg-white/5 w-8 px-2"
                                title={t("sessionManager.title")}
                              >
                                <History className="w-4 h-4" />
                              </Button>
                            </>
                          ) : (
                            <>
                              <Button
                                variant="ghost"
                                size="sm"
                                onClick={() => setCurrentView("skills")}
                                className={cn(
                                  "text-muted-foreground hover:text-foreground hover:bg-black/5 dark:hover:bg-white/5",
                                  "transition-all duration-200 ease-in-out overflow-hidden",
                                  hasSkillsSupport
                                    ? "opacity-100 w-8 scale-100 px-2"
                                    : "opacity-0 w-0 scale-75 pointer-events-none px-0 -ml-1",
                                )}
                                title={t("skills.manage")}
                              >
                                <Wrench className="flex-shrink-0 w-4 h-4" />
                              </Button>
                              <Button
                                variant="ghost"
                                size="sm"
                                onClick={() => setCurrentView("prompts")}
                                className="text-muted-foreground hover:text-foreground hover:bg-black/5 dark:hover:bg-white/5 w-8 px-2"
                                title={t("prompts.manage")}
                              >
                                <Book className="w-4 h-4" />
                              </Button>
                              {hasUniversalProviderSupport ? (
                                <Button
                                  variant="ghost"
                                  size="sm"
                                  onClick={() => setCurrentView("universal")}
                                  className="text-muted-foreground hover:text-foreground hover:bg-black/5 dark:hover:bg-white/5 w-8 px-2"
                                  title={t("universalProvider.title", {
                                    defaultValue: "统一供应商",
                                  })}
                                  data-testid="open-universal-provider-view"
                                >
                                  <Layers className="flex-shrink-0 w-4 h-4" />
                                </Button>
                              ) : null}
                              <Button
                                variant="ghost"
                                size="sm"
                                onClick={() => setCurrentView("sessions")}
                                className={cn(
                                  "text-muted-foreground hover:text-foreground hover:bg-black/5 dark:hover:bg-white/5",
                                  "transition-all duration-200 ease-in-out overflow-hidden",
                                  hasSessionSupport
                                    ? "opacity-100 w-8 scale-100 px-2"
                                    : "opacity-0 w-0 scale-75 pointer-events-none px-0 -ml-1",
                                )}
                                title={t("sessionManager.title")}
                              >
                                <History className="flex-shrink-0 w-4 h-4" />
                              </Button>
                              {hasMcpSupport && (
                                <Button
                                  variant="ghost"
                                  size="sm"
                                  onClick={() => setCurrentView("mcp")}
                                  className="text-muted-foreground hover:text-foreground hover:bg-black/5 dark:hover:bg-white/5 w-8 px-2"
                                  title={t("mcp.title")}
                                >
                                  <McpIcon size={16} />
                                </Button>
                              )}
                            </>
                          )}
                        </motion.div>
                      </AnimatePresence>
                    </div>

                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => void handleImportCurrentProvider()}
                      className="hover:bg-black/5 dark:hover:bg-white/5"
                    >
                      <Download className="w-4 h-4 mr-2" />
                      {t("provider.importCurrent")}
                    </Button>
                    <Button
                      onClick={() => setIsAddOpen(true)}
                      size="icon"
                      className={`ml-2 ${addActionButtonClass}`}
                    >
                      <Plus className="w-5 h-5" />
                    </Button>
                  </>
                )}
              </div>
            </div>
          </div>
        </div>
      </header>

      <main className="flex-1 min-h-0 flex flex-col overflow-y-auto animate-fade-in">
        {isOpenClawView && openclawHealthWarnings.length > 0 && (
          <OpenClawHealthBanner warnings={openclawHealthWarnings} />
        )}
        {renderContent()}
      </main>

      <AddProviderDialog
        open={isAddOpen}
        onOpenChange={setIsAddOpen}
        appId={activeApp}
        onSubmit={addProvider}
      />

      <EditProviderDialog
        open={Boolean(editingProvider)}
        provider={effectiveEditingProvider}
        onOpenChange={(open) => {
          if (!open) {
            setEditingProvider(null);
          }
        }}
        onSubmit={handleEditProvider}
        appId={activeApp}
        isProxyTakeover={isCurrentAppTakeoverActive}
      />

      {effectiveUsageProvider && (
        <UsageScriptModal
          key={effectiveUsageProvider.id}
          provider={effectiveUsageProvider}
          appId={activeApp}
          isOpen={Boolean(usageProvider)}
          onClose={() => setUsageProvider(null)}
          onSave={(script) => {
            if (usageProvider) {
              void saveUsageScript(usageProvider, script);
            }
          }}
        />
      )}

      <ConfirmDialog
        isOpen={Boolean(confirmAction)}
        title={
          confirmAction?.action === "remove"
            ? t("confirm.removeProvider")
            : t("confirm.deleteProvider")
        }
        message={confirmActionMessage}
        onConfirm={() => void handleConfirmAction()}
        onCancel={() => setConfirmAction(null)}
      />

      <ConfirmDialog
        isOpen={launchDashboardOpen}
        title={t("hermes.webui.launchConfirmTitle")}
        message={t("hermes.webui.launchConfirmMessage")}
        confirmText={t("hermes.webui.launchConfirmAction")}
        variant="info"
        onConfirm={() => {
          setLaunchDashboardOpen(false);
          void (async () => {
            try {
              await hermesApi.launchDashboard();
              toast.success(t("hermes.webui.launching"));
            } catch (error) {
              toast.error(t("hermes.webui.launchFailed"), {
                description: extractErrorMessage(error) || undefined,
              });
            }
          })();
        }}
        onCancel={() => setLaunchDashboardOpen(false)}
      />

      <DeepLinkImportDialog ref={deepLinkImportDialogRef} />
      <FirstRunNoticeDialog />
    </div>
  );
}

export default App;
