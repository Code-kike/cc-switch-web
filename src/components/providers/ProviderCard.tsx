import { useMemo, useState } from "react";
import {
  AlertTriangle,
  GripVertical,
  ChevronDown,
  ChevronUp,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import type {
  DraggableAttributes,
  DraggableSyntheticListeners,
} from "@dnd-kit/core";
import type { OpenClawProviderConfig, Provider } from "@/types";
import type { AppId } from "@/lib/api";
import { cn } from "@/lib/utils";
import { ProviderActions } from "@/components/providers/ProviderActions";
import { ProviderIcon } from "@/components/ProviderIcon";
import UsageFooter from "@/components/UsageFooter";
import SubscriptionQuotaFooter from "@/components/SubscriptionQuotaFooter";
import CopilotQuotaFooter from "@/components/CopilotQuotaFooter";
import CodexOauthQuotaFooter from "@/components/CodexOauthQuotaFooter";
import XaiOauthQuotaFooter from "@/components/XaiOauthQuotaFooter";
import { PROVIDER_TYPES, TEMPLATE_TYPES } from "@/config/constants";
import { isHermesReadOnlyProvider } from "@/config/hermesProviderPresets";
import { FailoverPriorityBadge } from "@/components/providers/FailoverPriorityBadge";
import { HealthStatusIndicator } from "@/components/providers/HealthStatusIndicator";
import {
  extractCodexBaseUrl,
  extractCodexExperimentalBearerToken,
} from "@/utils/providerConfigUtils";
import { resolveManagedAccountId } from "@/lib/authBinding";
import {
  resolveCodexOfficialIdentity,
  providerNeedsRouting,
} from "@/utils/providerCapabilities";
import { useProviderHealth } from "@/lib/query/failover";
import { useUsageQuery } from "@/lib/query/queries";
import { useProviderLimits, useProviderStats } from "@/lib/query/usage";
import { extractErrorMessage } from "@/utils/errorUtils";
import { resolveProviderIcon } from "@/utils/providerIcon";
import { ProviderStatusBadge } from "@/components/providers/ProviderStatusBadge";
import { useManagedAuth } from "@/components/providers/forms/hooks/useManagedAuth";

interface DragHandleProps {
  attributes: DraggableAttributes;
  listeners: DraggableSyntheticListeners;
  isDragging: boolean;
}

interface ProviderCardProps {
  provider: Provider;
  isCurrent: boolean;
  appId: AppId;
  isInConfig?: boolean; // OpenCode: 是否已添加到 opencode.json
  isOmo?: boolean;
  isOmoSlim?: boolean;
  onSwitch: (provider: Provider) => void;
  onEdit: (provider: Provider) => void;
  onDelete: (provider: Provider) => void;
  onRemoveFromConfig?: (provider: Provider) => void;
  onDisableOmo?: () => void;
  onDisableOmoSlim?: () => void;
  onConfigureUsage: (provider: Provider) => void;
  onOpenWebsite: (url: string) => void;
  onDuplicate: (provider: Provider) => void;
  onTest?: (provider: Provider) => void;
  onOpenTerminal?: (provider: Provider) => void;
  isTesting?: boolean;
  isProxyRunning: boolean;
  isProxyTakeover?: boolean; // 代理接管模式（Live配置已被接管，切换为热切换）
  dragHandleProps?: DragHandleProps;
  isAutoFailoverEnabled?: boolean; // 是否开启自动故障转移
  failoverPriority?: number; // 故障转移优先级（1 = P1, 2 = P2, ...）
  isInFailoverQueue?: boolean; // 是否在故障转移队列中
  onToggleFailover?: (enabled: boolean) => void; // 切换故障转移队列
  activeProviderId?: string; // 代理当前实际使用的供应商 ID（用于故障转移模式下标注绿色边框）
  // OpenClaw: default model
  isDefaultModel?: boolean;
  // Pi: native provider state unreadable → freeze membership/delete actions
  isStateChangeProtected?: boolean;
  onSetAsDefault?: (modelId?: string) => void;
}

/** 判断是否为官方供应商（无自定义 base URL / API key，直连官方 API） */
function isOfficialProvider(provider: Provider, appId: AppId): boolean {
  // Grok Build's official seed is identified by its explicit category. Do not
  // infer it from an empty config: custom Grok endpoints may intentionally use
  // the same shape while still requiring script credentials.
  if (appId === "grokbuild") {
    return provider.category === "official";
  }

  const config = provider.settingsConfig as Record<string, any>;
  if (appId === "claude") {
    const baseUrl = config?.env?.ANTHROPIC_BASE_URL;
    return !baseUrl || (typeof baseUrl === "string" && baseUrl.trim() === "");
  }
  if (appId === "codex") {
    // 无 OPENAI_API_KEY → 使用 Codex CLI 内置 OAuth（官方）
    const apiKey = config?.auth?.OPENAI_API_KEY;
    const bearerToken =
      typeof config?.config === "string"
        ? extractCodexExperimentalBearerToken(config.config)
        : undefined;
    return (
      !bearerToken &&
      (!apiKey || (typeof apiKey === "string" && apiKey.trim() === ""))
    );
  }
  if (appId === "gemini") {
    // 无 GEMINI_API_KEY 且无 GOOGLE_GEMINI_BASE_URL → Google OAuth 官方模式
    const apiKey = config?.env?.GEMINI_API_KEY;
    const baseUrl = config?.env?.GOOGLE_GEMINI_BASE_URL;
    return (
      (!apiKey || (typeof apiKey === "string" && apiKey.trim() === "")) &&
      (!baseUrl || (typeof baseUrl === "string" && baseUrl.trim() === ""))
    );
  }
  return false;
}

const extractApiUrl = (provider: Provider, fallbackText: string) => {
  if (provider.notes?.trim()) {
    return provider.notes.trim();
  }

  if (provider.websiteUrl) {
    return provider.websiteUrl;
  }

  const config = provider.settingsConfig;

  if (config && typeof config === "object") {
    const object = config as Record<string, any>;
    const envBase =
      object?.env?.ANTHROPIC_BASE_URL || object?.env?.GOOGLE_GEMINI_BASE_URL;
    if (typeof envBase === "string" && envBase.trim()) {
      return envBase;
    }

    // Native additive configs (Pi / OpenClaw / OpenCode) carry the endpoint on
    // the provider node or on its first model entry.
    const directBaseUrl =
      object.baseUrl ||
      object.base_url ||
      object.options?.baseURL ||
      (Array.isArray(object.models)
        ? object.models.find(
            (model: unknown) =>
              model &&
              typeof model === "object" &&
              typeof (model as Record<string, unknown>).baseUrl === "string",
          )?.baseUrl
        : undefined);
    if (typeof directBaseUrl === "string" && directBaseUrl.trim()) {
      return directBaseUrl;
    }

    const baseUrl = object.config;

    if (typeof baseUrl === "string" && baseUrl.includes("base_url")) {
      const extractedBaseUrl = extractCodexBaseUrl(baseUrl);
      if (extractedBaseUrl) {
        return extractedBaseUrl;
      }
    }
  }

  return fallbackText;
};

type ProviderHealthIndicatorStatus = "operational" | "degraded" | "failed";
const CARD_USAGE_RANGE = { preset: "30d" } as const;

function parseUsdNumber(value: unknown): number | null {
  if (typeof value === "number") {
    return Number.isFinite(value) ? value : null;
  }

  if (typeof value === "string") {
    const parsed = Number.parseFloat(value);
    return Number.isFinite(parsed) ? parsed : null;
  }

  return null;
}

function formatUsdAmount(
  value: unknown,
  digits: number,
  fallback: string = "--",
): string {
  const parsed = parseUsdNumber(value);
  if (parsed == null) {
    return fallback;
  }
  return `$${parsed.toFixed(digits)}`;
}

function getHealthIndicatorStatus(
  consecutiveFailures: number,
): ProviderHealthIndicatorStatus {
  if (consecutiveFailures === 0) {
    return "operational";
  }
  if (consecutiveFailures < 5) {
    return "degraded";
  }
  return "failed";
}

export function ProviderCard({
  provider,
  isCurrent,
  appId,
  isInConfig = true,
  isOmo = false,
  isOmoSlim = false,
  onSwitch,
  onEdit,
  onDelete,
  onRemoveFromConfig,
  onDisableOmo,
  onDisableOmoSlim,
  onConfigureUsage,
  onOpenWebsite,
  onDuplicate,
  onTest,
  onOpenTerminal,
  isTesting,
  isProxyRunning,
  isProxyTakeover = false,
  dragHandleProps,
  isAutoFailoverEnabled = false,
  failoverPriority,
  isInFailoverQueue = false,
  onToggleFailover,
  activeProviderId,
  // OpenClaw: default model
  isDefaultModel,
  isStateChangeProtected,
  onSetAsDefault,
}: ProviderCardProps) {
  const { t } = useTranslation();
  const codexOfficialIdentity = resolveCodexOfficialIdentity(appId, provider);
  const managedCodexAccountId = resolveManagedAccountId(
    provider.meta,
    "codex_oauth",
  )?.trim();
  const {
    accounts: managedAuthAccounts,
    authStatus: managedAuthStatus,
    isLoadingStatus: isManagedAuthLoading,
  } = useManagedAuth("codex_oauth", undefined, {
    enabled: codexOfficialIdentity === "managed_account",
  });
  const isCodexAuthStatusSuccess = Boolean(managedAuthStatus);
  const isCodexAuthStatusError = !isManagedAuthLoading && !managedAuthStatus;
  const managedCodexAccount = managedAuthAccounts.find(
    (account) => account.id === managedCodexAccountId,
  );
  const isBoundCodexOfficial = codexOfficialIdentity === "managed_account";
  const manualNote = provider.notes?.trim() || undefined;
  const providerNameIncludesAccountLogin = Boolean(
    managedCodexAccount?.login &&
      (provider.name.trim() === managedCodexAccount.login ||
        provider.name.trim() ===
          `OpenAI Official (${managedCodexAccount.login})`),
  );

  // OMO and OMO Slim share the same card behavior
  const isAnyOmo = isOmo || isOmoSlim;
  const handleDisableAnyOmo = isOmoSlim ? onDisableOmoSlim : onDisableOmo;
  const isAdditiveMode = (appId === "opencode" && !isAnyOmo) || appId === "pi";
  const supportsLiveConfigStatus =
    (appId === "opencode" ||
      appId === "openclaw" ||
      appId === "hermes" ||
      appId === "pi") &&
    !isAnyOmo;
  const healthEnabled = isProxyRunning && isInFailoverQueue;
  const configuredDailyLimit = parseUsdNumber(provider.meta?.limitDailyUsd);
  const configuredMonthlyLimit = parseUsdNumber(provider.meta?.limitMonthlyUsd);
  const hasConfiguredLimits =
    configuredDailyLimit != null || configuredMonthlyLimit != null;

  const { data: health, error: healthError } = useProviderHealth(
    provider.id,
    appId,
    healthEnabled,
  );
  const { data: providerLimits, error: providerLimitsError } =
    useProviderLimits(provider.id, appId, hasConfiguredLimits);
  const { data: providerStats, error: providerStatsError } = useProviderStats(
    CARD_USAGE_RANGE,
    appId,
    {
      refetchInterval: false,
    },
  );

  const fallbackUrlText = t("provider.notConfigured", {
    defaultValue: "未配置接口地址",
  });

  const displayUrl = useMemo(() => {
    return extractApiUrl(provider, fallbackUrlText);
  }, [provider, fallbackUrlText]);

  const openclawDefaultModelOptions = useMemo(() => {
    if (appId !== "openclaw") return [];
    const config = provider.settingsConfig as OpenClawProviderConfig;
    if (!Array.isArray(config?.models)) return [];
    return config.models
      .filter((model) => typeof model.id === "string" && model.id.trim())
      .map((model) => ({ id: model.id, name: model.name }));
  }, [appId, provider.settingsConfig]);

  const isClickableUrl = useMemo(() => {
    if (provider.notes?.trim()) {
      return false;
    }
    if (displayUrl === fallbackUrlText) {
      return false;
    }
    return true;
  }, [provider.notes, displayUrl, fallbackUrlText]);

  const usageEnabled = provider.meta?.usage_script?.enabled ?? false;
  const isOfficial = isOfficialProvider(provider, appId);
  // 官方判定只认显式 category === "official"（SSOT），不回退 isOfficial 的空字段启发式。
  // 理由（此判定曾在「纯 category ↔ category+isOfficial 回退」间反复，结论钉死于此）：
  //  1) 封号保护是高代价决策，不该建立在「base_url/key 缺失」这种脆弱信号上——它无法区分
  //     「想直连官方」与「自定义但还没填完」，两者都表现为字段为空，必然误伤后者。
  //  2) 启发式在 UI 多拦的部分，执行层 useProviderActions.ts 也只认 category === "official"、
  //     并不兑现（绕过 UI 即可切换）→ 属虚保护，却以误伤 category 缺失的自定义供应商为代价。
  //  3) 预设导入的官方一定带 category="official"，category 缺失的「真官方」现实中≈不存在。
  // 真官方就该有显式 category；手动新建官方应引导标注，而不是靠空字段猜。
  const supportsOfficialSubscription =
    isOfficial && ["claude", "codex", "gemini", "grokbuild"].includes(appId);
  const isOfficialSubscriptionUsage =
    provider.meta?.usage_script?.templateType ===
    TEMPLATE_TYPES.OFFICIAL_SUBSCRIPTION;
  const officialSubscriptionEnabled =
    supportsOfficialSubscription && usageEnabled && isOfficialSubscriptionUsage;
  const isOfficialBlockedByProxy =
    isProxyTakeover && provider.category === "official";
  const isCopilot =
    provider.meta?.providerType === PROVIDER_TYPES.GITHUB_COPILOT ||
    provider.meta?.usage_script?.templateType === "github_copilot";
  // Hermes v12+ overlay entries live under the `providers:` dict and are
  // read-only here — writes have to go through Hermes Web UI.
  const isHermesReadOnly =
    appId === "hermes" && isHermesReadOnlyProvider(provider.settingsConfig);
  const isCodexOauth =
    appId === "codex"
      ? isBoundCodexOfficial
      : provider.meta?.providerType === PROVIDER_TYPES.CODEX_OAUTH;
  // xAI OAuth (SuperGrok 反代)：额度经自管 OAuth token 自动显示，与 codex_oauth 同构
  const isXaiOauth = provider.meta?.providerType === PROVIDER_TYPES.XAI_OAUTH;
  const isClaudeThirdParty =
    appId === "claude" && provider.category === "third_party";
  const needsRouting = providerNeedsRouting(appId, provider);
  const usageStats = providerStats?.find(
    (item) => item.providerId === provider.id,
  );
  const healthStatus = health
    ? getHealthIndicatorStatus(health.consecutive_failures)
    : null;
  const dailyLimitText =
    configuredDailyLimit != null
      ? formatUsdAmount(providerLimits?.dailyLimit ?? configuredDailyLimit, 2)
      : null;
  const monthlyLimitText =
    configuredMonthlyLimit != null
      ? formatUsdAmount(
          providerLimits?.monthlyLimit ?? configuredMonthlyLimit,
          2,
        )
      : null;
  const dailyUsageText = formatUsdAmount(providerLimits?.dailyUsage, 4);
  const monthlyUsageText = formatUsdAmount(providerLimits?.monthlyUsage, 4);
  const usageStatsText =
    usageStats && usageStats.requestCount > 0
      ? `${t("provider.usage30dSummary", {
          defaultValue: "30天",
        })} ${usageStats.requestCount.toLocaleString()} ${t(
          "provider.usage30dRequests",
          {
            defaultValue: "次",
          },
        )} / ${formatUsdAmount(usageStats.totalCost, 4)}`
      : null;

  // 获取用量数据以判断是否有多套餐
  // 累加模式应用（OpenCode/OpenClaw/Hermes/Pi）：使用 isInConfig 代替 isCurrent
  const shouldAutoQuery =
    appId === "opencode" ||
    appId === "openclaw" ||
    appId === "hermes" ||
    appId === "pi"
      ? isInConfig
      : isCurrent;
  const autoQueryInterval = shouldAutoQuery
    ? provider.meta?.usage_script?.autoQueryInterval || 0
    : 0;

  const { data: usage } = useUsageQuery(provider.id, appId, {
    enabled: usageEnabled && !isOfficial && !isOfficialSubscriptionUsage,
    autoQueryInterval,
  });

  const isTokenPlan =
    provider.meta?.usage_script?.templateType === "token_plan";
  const hasMultiplePlans =
    usage?.success && usage.data && usage.data.length > 1 && !isTokenPlan;

  const [expandedPreference, setExpandedPreference] = useState<boolean | null>(
    null,
  );
  const isExpanded = expandedPreference ?? hasMultiplePlans;

  const handleOpenWebsite = () => {
    if (!isClickableUrl) {
      return;
    }
    onOpenWebsite(displayUrl);
  };

  // 判断是否是"当前使用中"的供应商
  // - OMO/OMO Slim 供应商：使用 isCurrent
  // - OpenClaw：使用默认模型归属的 provider 作为当前项（蓝色边框）
  // - OpenCode（非 OMO）/ Pi：不存在"当前"概念，返回 false
  // - 故障转移模式：代理实际使用的供应商（activeProviderId）
  // - 普通模式：isCurrent
  const isActiveProvider = isAnyOmo
    ? isCurrent
    : appId === "openclaw"
      ? Boolean(isDefaultModel)
      : appId === "opencode" || appId === "pi"
        ? false
        : isAutoFailoverEnabled
          ? activeProviderId === provider.id
          : isCurrent;

  const shouldUseGreen = !isAnyOmo && isProxyTakeover && isActiveProvider;
  const hasPersistentConfigHighlight = isAdditiveMode && isInConfig;
  const shouldUseBlue =
    (isAnyOmo && isActiveProvider) ||
    (!isAnyOmo &&
      !isProxyTakeover &&
      (isActiveProvider || hasPersistentConfigHighlight));

  return (
    <div
      className={cn(
        "relative overflow-hidden rounded-xl border border-border p-4 transition-all duration-300",
        "bg-card text-card-foreground group",
        isAutoFailoverEnabled || isProxyTakeover
          ? "hover:border-emerald-500/50"
          : "hover:border-border-active",
        shouldUseGreen &&
          "border-emerald-500/60 shadow-sm shadow-emerald-500/10",
        shouldUseBlue && "border-blue-500/60 shadow-sm shadow-blue-500/10",
        !(isActiveProvider || hasPersistentConfigHighlight) &&
          "hover:shadow-sm",
        dragHandleProps?.isDragging &&
          "cursor-grabbing border-primary shadow-lg scale-105 z-10",
      )}
    >
      <div
        className={cn(
          "absolute inset-0 bg-gradient-to-r to-transparent transition-opacity duration-500 pointer-events-none",
          shouldUseGreen && "from-emerald-500/10",
          shouldUseBlue && "from-blue-500/10",
          !shouldUseGreen && !shouldUseBlue && "from-primary/10",
          isActiveProvider || hasPersistentConfigHighlight
            ? "opacity-100"
            : "opacity-0",
        )}
      />
      <div className="relative flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
        <div className="flex flex-1 items-center gap-2">
          <button
            type="button"
            className={cn(
              "-ml-1.5 flex-shrink-0 cursor-grab active:cursor-grabbing p-1.5",
              "text-muted-foreground/50 hover:text-muted-foreground transition-colors",
              dragHandleProps?.isDragging && "cursor-grabbing",
            )}
            aria-label={t("provider.dragHandle")}
            {...(dragHandleProps?.attributes ?? {})}
            {...(dragHandleProps?.listeners ?? {})}
          >
            <GripVertical className="h-4 w-4" />
          </button>

          <div className="h-8 w-8 rounded-lg bg-muted flex items-center justify-center border border-border group-hover:scale-105 transition-transform duration-300">
            <ProviderIcon
              icon={resolveProviderIcon(
                appId,
                provider.icon,
                provider.iconColor,
              )}
              name={provider.name}
              color={provider.iconColor}
              size={20}
            />
          </div>

          <div className="space-y-1">
            <div className="flex flex-wrap items-center gap-2 min-h-7">
              <h3
                className={cn(
                  "text-base font-semibold leading-none",
                  codexOfficialIdentity && "min-w-0 flex-1 truncate",
                )}
                title={codexOfficialIdentity ? provider.name : undefined}
              >
                {provider.name}
              </h3>

              {isOmo && (
                <span className="inline-flex items-center rounded-md bg-violet-100 px-1.5 py-0.5 text-[10px] font-semibold text-violet-700 dark:bg-violet-900/40 dark:text-violet-300">
                  OMO
                </span>
              )}

              {isOmoSlim && (
                <span className="inline-flex items-center rounded-md bg-indigo-100 px-1.5 py-0.5 text-[10px] font-semibold text-indigo-700 dark:bg-indigo-900/40 dark:text-indigo-300">
                  Slim
                </span>
              )}

              {healthEnabled && health && healthStatus && (
                <div title={health.last_error ?? undefined}>
                  <HealthStatusIndicator
                    status={healthStatus}
                    className="rounded-full bg-muted/60 px-2 py-0.5"
                  />
                </div>
              )}

              {(appId === "claude" ||
                appId === "codex" ||
                appId === "grokbuild") &&
                needsRouting && (
                  <ProviderStatusBadge
                    tone="info"
                    label={t("provider.needsRouting", {
                      defaultValue: "需要路由",
                    })}
                  />
                )}

              {(appId === "claude" || appId === "codex") &&
                provider.category === "official" && (
                  <ProviderStatusBadge
                    label={t("provider.noRoutingSupport", {
                      defaultValue: "不支持路由",
                    })}
                  />
                )}

              {isAutoFailoverEnabled &&
                isInFailoverQueue &&
                failoverPriority && (
                  <FailoverPriorityBadge priority={failoverPriority} />
                )}

              {isHermesReadOnly && (
                <span
                  className="inline-flex items-center rounded-md bg-slate-200 px-1.5 py-0.5 text-[10px] font-semibold text-slate-700 dark:bg-slate-700/60 dark:text-slate-200"
                  title={t("provider.managedByHermesHint", {
                    defaultValue: "由 Hermes 管理，请在 Hermes Web UI 中编辑",
                  })}
                >
                  {t("provider.managedByHermes", {
                    defaultValue: "Hermes Managed",
                  })}
                </span>
              )}

              {supportsLiveConfigStatus && (
                <span
                  className={cn(
                    "inline-flex items-center rounded-md px-1.5 py-0.5 text-[10px] font-semibold",
                    isInConfig
                      ? "bg-blue-500/10 text-blue-700 dark:text-blue-300"
                      : "bg-slate-200 text-slate-700 dark:bg-slate-700/60 dark:text-slate-200",
                  )}
                  title={t("provider.liveConfigStatusHint", {
                    defaultValue: "反映当前 live 配置中是否包含该供应商",
                  })}
                >
                  {isInConfig
                    ? t("provider.liveConfigPresent", {
                        defaultValue: "Live Config",
                      })
                    : t("provider.liveConfigMissing", {
                        defaultValue: "DB Only",
                      })}
                </span>
              )}
            </div>

            {codexOfficialIdentity && codexOfficialIdentity !== "api_key" ? (
              <div className="flex min-w-0 items-center gap-2 text-sm text-muted-foreground">
                {codexOfficialIdentity === "native_login" ? (
                  <span className="min-w-0 truncate" title={manualNote}>
                    {manualNote ??
                      t("codex.followCodexLoginDescription", {
                        defaultValue: "账号会随 Codex CLI 当前登录变化",
                      })}
                  </span>
                ) : managedCodexAccount ? (
                  <>
                    <span
                      className="min-w-0 truncate"
                      title={manualNote ?? managedCodexAccount.login}
                    >
                      {manualNote ??
                        (providerNameIncludesAccountLogin
                          ? t("codex.openAiAccount", {
                              defaultValue: "OpenAI 账号",
                            })
                          : managedCodexAccount.login)}
                    </span>
                    {managedCodexAccount.requires_reauth && (
                      <span className="inline-flex shrink-0 items-center gap-1 text-amber-700 dark:text-amber-300">
                        <AlertTriangle className="h-3.5 w-3.5" />
                        {t("codexOauth.reauthBadge", "需要重新登录")}
                      </span>
                    )}
                  </>
                ) : isCodexAuthStatusError ? (
                  <span className="inline-flex min-w-0 items-center gap-1 text-amber-700 dark:text-amber-300">
                    <AlertTriangle className="h-3.5 w-3.5 shrink-0" />
                    <span className="truncate">
                      {t("codex.accountStatusUnavailable", {
                        defaultValue: "无法读取账号信息",
                      })}
                    </span>
                  </span>
                ) : isCodexAuthStatusSuccess ? (
                  <>
                    <span className="inline-flex min-w-0 items-center gap-1 text-sm text-amber-700 dark:text-amber-300">
                      <AlertTriangle className="h-3.5 w-3.5 shrink-0" />
                      <span className="truncate">
                        {t("codex.boundAccountUnavailable", {
                          defaultValue: "绑定的账号不可用",
                        })}
                      </span>
                    </span>
                    <button
                      type="button"
                      className="shrink-0 text-sm font-medium text-primary hover:underline"
                      onClick={() => onEdit(provider)}
                    >
                      {t("codex.chooseAccount", {
                        defaultValue: "选择账号",
                      })}
                    </button>
                  </>
                ) : (
                  <span className="min-w-0 truncate">
                    {t("codex.accountLoading", {
                      defaultValue: "正在加载账号…",
                    })}
                  </span>
                )}
              </div>
            ) : displayUrl ? (
              <button
                type="button"
                onClick={handleOpenWebsite}
                className={cn(
                  "inline-flex items-center text-sm max-w-[280px]",
                  isClickableUrl
                    ? "text-blue-500 transition-colors hover:underline dark:text-blue-400 cursor-pointer"
                    : "text-muted-foreground cursor-default",
                )}
                title={displayUrl}
                disabled={!isClickableUrl}
              >
                <span className="truncate">{displayUrl}</span>
              </button>
            ) : null}

            {(providerStatsError ||
              providerLimitsError ||
              healthError ||
              usageStatsText ||
              dailyLimitText ||
              monthlyLimitText) && (
              <div className="flex flex-wrap items-center gap-2 pt-1">
                {providerStatsError && !usageStatsText && (
                  <span
                    className="inline-flex items-center rounded-full bg-amber-500/10 px-2 py-1 text-xs font-medium text-amber-700 dark:text-amber-300"
                    title={extractErrorMessage(providerStatsError) || undefined}
                  >
                    {t("provider.usageStatsUnavailable", {
                      defaultValue: "用量摘要不可用",
                    })}
                  </span>
                )}

                {providerLimitsError &&
                  !providerLimits &&
                  hasConfiguredLimits && (
                    <span
                      className="inline-flex items-center rounded-full bg-amber-500/10 px-2 py-1 text-xs font-medium text-amber-700 dark:text-amber-300"
                      title={
                        extractErrorMessage(providerLimitsError) || undefined
                      }
                    >
                      {t("provider.limitsUnavailable", {
                        defaultValue: "限额状态不可用",
                      })}
                    </span>
                  )}

                {healthError && !health && healthEnabled && (
                  <span
                    className="inline-flex items-center rounded-full bg-amber-500/10 px-2 py-1 text-xs font-medium text-amber-700 dark:text-amber-300"
                    title={extractErrorMessage(healthError) || undefined}
                  >
                    {t("provider.healthUnavailable", {
                      defaultValue: "健康状态不可用",
                    })}
                  </span>
                )}

                {usageStatsText && (
                  <span
                    className="inline-flex items-center rounded-full bg-muted/60 px-2 py-1 text-xs font-medium text-muted-foreground"
                    title={
                      usageStats
                        ? t("provider.usage30dStatsHint", {
                            defaultValue: `成功率 ${usageStats.successRate.toFixed(1)}% · 平均延迟 ${Math.round(usageStats.avgLatencyMs)}ms`,
                          })
                        : undefined
                    }
                  >
                    {usageStatsText}
                  </span>
                )}

                {dailyLimitText && (
                  <span
                    className={cn(
                      "inline-flex items-center rounded-full px-2 py-1 text-xs font-medium",
                      providerLimits?.dailyExceeded
                        ? "bg-red-500/10 text-red-600 dark:text-red-400"
                        : "bg-muted/60 text-muted-foreground",
                    )}
                    title={
                      providerLimits?.dailyExceeded
                        ? t("provider.dailyLimitExceeded", {
                            defaultValue: "已超出每日限额",
                          })
                        : undefined
                    }
                  >
                    {t("provider.dailyLimitSummary", {
                      defaultValue: "日限额",
                    })}{" "}
                    {dailyUsageText} / {dailyLimitText}
                  </span>
                )}

                {monthlyLimitText && (
                  <span
                    className={cn(
                      "inline-flex items-center rounded-full px-2 py-1 text-xs font-medium",
                      providerLimits?.monthlyExceeded
                        ? "bg-red-500/10 text-red-600 dark:text-red-400"
                        : "bg-muted/60 text-muted-foreground",
                    )}
                    title={
                      providerLimits?.monthlyExceeded
                        ? t("provider.monthlyLimitExceeded", {
                            defaultValue: "已超出每月限额",
                          })
                        : undefined
                    }
                  >
                    {t("provider.monthlyLimitSummary", {
                      defaultValue: "月限额",
                    })}{" "}
                    {monthlyUsageText} / {monthlyLimitText}
                  </span>
                )}
              </div>
            )}
          </div>
        </div>

        <div className="flex items-center ml-auto min-w-0 gap-3">
          <div className="ml-auto">
            <div className="flex items-center gap-1">
              {isCopilot ? (
                <CopilotQuotaFooter
                  meta={provider.meta}
                  inline={true}
                  isCurrent={isCurrent}
                />
              ) : isCodexOauth ? (
                !isBoundCodexOfficial || usageEnabled ? (
                  <CodexOauthQuotaFooter
                    meta={provider.meta}
                    inline={true}
                    isCurrent={isCurrent}
                    autoQueryInterval={
                      isBoundCodexOfficial
                        ? (provider.meta?.usage_script?.autoQueryInterval ?? 5)
                        : undefined
                    }
                  />
                ) : null
              ) : isXaiOauth ? (
                <XaiOauthQuotaFooter
                  meta={provider.meta}
                  inline={true}
                  isCurrent={isCurrent}
                />
              ) : isOfficial ? (
                officialSubscriptionEnabled ? (
                  <SubscriptionQuotaFooter
                    appId={appId}
                    inline={true}
                    isCurrent={isCurrent}
                    autoQueryInterval={
                      provider.meta?.usage_script?.autoQueryInterval ?? 0
                    }
                  />
                ) : null
              ) : hasMultiplePlans ? (
                <div className="flex items-center gap-2 text-xs text-gray-600 dark:text-gray-400">
                  <span className="font-medium">
                    {t("usage.multiplePlans", {
                      count: usage?.data?.length || 0,
                      defaultValue: `${usage?.data?.length || 0} 个套餐`,
                    })}
                  </span>
                </div>
              ) : (
                <UsageFooter
                  provider={provider}
                  providerId={provider.id}
                  appId={appId}
                  usageEnabled={usageEnabled}
                  isCurrent={isCurrent}
                  isInConfig={isInConfig}
                  inline={true}
                />
              )}
              {hasMultiplePlans && (
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    setExpandedPreference(
                      (prev) => !(prev ?? hasMultiplePlans),
                    );
                  }}
                  className="p-1 rounded hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors text-gray-500 dark:text-gray-400 flex-shrink-0"
                  title={
                    isExpanded
                      ? t("usage.collapse", { defaultValue: "收起" })
                      : t("usage.expand", { defaultValue: "展开" })
                  }
                >
                  {isExpanded ? (
                    <ChevronUp size={14} />
                  ) : (
                    <ChevronDown size={14} />
                  )}
                </button>
              )}
            </div>
          </div>

          <div className="flex items-center gap-1.5 flex-shrink-0 opacity-0 pointer-events-none group-hover:opacity-100 group-focus-within:opacity-100 group-hover:pointer-events-auto group-focus-within:pointer-events-auto transition-opacity duration-200">
            <ProviderActions
              appId={appId}
              isCurrent={isCurrent}
              isInConfig={isInConfig}
              isTesting={isTesting}
              isProxyTakeover={isProxyTakeover}
              isOfficialBlockedByProxy={isOfficialBlockedByProxy}
              isReadOnly={isHermesReadOnly}
              isOmo={isAnyOmo}
              onSwitch={() => onSwitch(provider)}
              onEdit={() => onEdit(provider)}
              onDuplicate={() => onDuplicate(provider)}
              onTest={
                onTest &&
                !isOfficial &&
                !isCopilot &&
                !isCodexOauth &&
                !isClaudeThirdParty
                  ? () => onTest(provider)
                  : undefined
              }
              onConfigureUsage={
                (isOfficial && !supportsOfficialSubscription) ||
                isCopilot ||
                (isCodexOauth && !isBoundCodexOfficial) ||
                isXaiOauth
                  ? undefined
                  : () => onConfigureUsage(provider)
              }
              onDelete={() => onDelete(provider)}
              onRemoveFromConfig={
                onRemoveFromConfig
                  ? () => onRemoveFromConfig(provider)
                  : undefined
              }
              onDisableOmo={handleDisableAnyOmo}
              onOpenTerminal={
                onOpenTerminal ? () => onOpenTerminal(provider) : undefined
              }
              isAutoFailoverEnabled={isAutoFailoverEnabled}
              isInFailoverQueue={isInFailoverQueue}
              onToggleFailover={onToggleFailover}
              // OpenClaw: default model
              isDefaultModel={isDefaultModel}
              isStateChangeProtected={isStateChangeProtected}
              defaultModelOptions={openclawDefaultModelOptions}
              onSetAsDefault={onSetAsDefault}
            />
          </div>
        </div>
      </div>

      {isExpanded && hasMultiplePlans && (
        <div className="mt-4 pt-4 border-t border-border-default">
          <UsageFooter
            provider={provider}
            providerId={provider.id}
            appId={appId}
            usageEnabled={usageEnabled}
            isCurrent={isCurrent}
            isInConfig={isInConfig}
            inline={false}
          />
        </div>
      )}
    </div>
  );
}
