import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { motion } from "framer-motion";
import { Card, CardContent } from "@/components/ui/card";
import { useUsageSummaryByApp } from "@/lib/query/usage";
import { cn } from "@/lib/utils";
import {
  Activity,
  ArrowDownToLine,
  ArrowUpFromLine,
  Database,
  Info,
  Loader2,
  Sparkles,
  Zap,
} from "lucide-react";
import {
  fmtUsd,
  formatTokensShort,
  getResolvedLang,
  parseFiniteNumber,
} from "./format";
import {
  CACHE_INCLUSIVE_APP_TYPES,
  type AppType,
  type UsageRangeSelection,
  type UsageSummary,
  type UsageSummaryByApp,
} from "@/types/usage";

interface UsageHeroProps {
  range: UsageRangeSelection;
  appType?: string;
  refreshIntervalMs: number;
}

interface TitleTheme {
  accent: string;
  iconBg: string;
}

const TITLE_THEMES: Record<AppType | "all", TitleTheme> = {
  all: { accent: "text-primary", iconBg: "bg-primary/10" },
  claude: {
    accent: "text-amber-600 dark:text-amber-400",
    iconBg: "bg-amber-500/10",
  },
  codex: {
    accent: "text-emerald-600 dark:text-emerald-400",
    iconBg: "bg-emerald-500/10",
  },
  gemini: {
    accent: "text-sky-600 dark:text-sky-400",
    iconBg: "bg-sky-500/10",
  },
  grokbuild: {
    accent: "text-rose-600 dark:text-rose-400",
    iconBg: "bg-rose-500/10",
  },
  opencode: {
    accent: "text-purple-600 dark:text-purple-400",
    iconBg: "bg-purple-500/10",
  },
  pi: {
    accent: "text-fuchsia-600 dark:text-fuchsia-400",
    iconBg: "bg-fuchsia-500/10",
  },
};

function aggregateSummaries(items: UsageSummary[]): UsageSummary {
  let totalRequests = 0;
  let successCount = 0;
  let totalCost = 0;
  let input = 0;
  let output = 0;
  let cacheCreation = 0;
  let cacheRead = 0;

  for (const summary of items) {
    totalRequests += summary.totalRequests;
    successCount += Math.round(
      (summary.totalRequests * summary.successRate) / 100,
    );
    totalCost += parseFiniteNumber(summary.totalCost) ?? 0;
    input += summary.totalInputTokens;
    output += summary.totalOutputTokens;
    cacheCreation += summary.totalCacheCreationTokens;
    cacheRead += summary.totalCacheReadTokens;
  }

  const cacheableInput = input + cacheCreation + cacheRead;
  return {
    totalRequests,
    totalCost: totalCost.toFixed(6),
    totalInputTokens: input,
    totalOutputTokens: output,
    totalCacheCreationTokens: cacheCreation,
    totalCacheReadTokens: cacheRead,
    successRate: totalRequests > 0 ? (successCount / totalRequests) * 100 : 0,
    realTotalTokens: input + output + cacheCreation + cacheRead,
    cacheHitRate: cacheableInput > 0 ? cacheRead / cacheableInput : 0,
  };
}

function pickSummary(
  apps: UsageSummaryByApp[],
  appType: string | undefined,
): UsageSummary | undefined {
  if (apps.length === 0) return undefined;
  if (appType) {
    return apps.find((app) => app.appType === appType)?.summary;
  }
  return aggregateSummaries(apps.map((app) => app.summary));
}

type CacheWriteState = "ok" | "partial" | "na";

function deriveCacheWriteState(appTypes: string[]): CacheWriteState {
  if (appTypes.length === 0) return "ok";
  const inclusiveCount = appTypes.filter((appType) =>
    CACHE_INCLUSIVE_APP_TYPES.has(appType),
  ).length;
  if (inclusiveCount === appTypes.length) return "na";
  if (inclusiveCount === 0) return "ok";
  return "partial";
}

export function UsageHero({
  range,
  appType,
  refreshIntervalMs,
}: UsageHeroProps) {
  const { t, i18n } = useTranslation();
  const lang = getResolvedLang(i18n);
  const { data, isLoading } = useUsageSummaryByApp(range, {
    refetchInterval: refreshIntervalMs > 0 ? refreshIntervalMs : false,
  });

  const selectedApp = appType === "all" ? undefined : appType;
  const allApps = data ?? [];
  const summary = pickSummary(allApps, selectedApp);
  const titleTheme =
    TITLE_THEMES[(selectedApp ?? "all") as keyof typeof TITLE_THEMES] ??
    TITLE_THEMES.all;
  const appLabel =
    selectedApp && selectedApp in TITLE_THEMES
      ? t(`usage.appFilter.${selectedApp}`)
      : null;

  const cacheWriteState = deriveCacheWriteState(
    selectedApp ? [selectedApp] : allApps.map((app) => app.appType),
  );

  const input = summary?.totalInputTokens ?? 0;
  const output = summary?.totalOutputTokens ?? 0;
  const cacheWrite = summary?.totalCacheCreationTokens ?? 0;
  const cacheRead = summary?.totalCacheReadTokens ?? 0;
  const realTotal = summary?.realTotalTokens ?? 0;
  const hitRate = summary?.cacheHitRate ?? 0;
  const totalCost = parseFiniteNumber(summary?.totalCost);
  const requests = summary?.totalRequests ?? 0;

  const cacheWriteDisplay = {
    value:
      cacheWriteState === "na" ? "N/A" : formatTokensShort(cacheWrite, lang),
    muted: cacheWriteState === "na",
    tooltip:
      cacheWriteState === "na"
        ? t(
            "usage.cacheWriteNotReported",
            "OpenAI-compatible protocols report cache hits but not cache writes.",
          )
        : cacheWriteState === "partial"
          ? t(
              "usage.cacheWritePartial",
              "Some protocols do not report cache writes, so this value may be low.",
            )
          : undefined,
  };

  if (isLoading) {
    return (
      <Card className="border border-border/50 bg-card/40 backdrop-blur-sm">
        <CardContent className="flex min-h-[200px] items-center justify-center">
          <Loader2 className="h-6 w-6 animate-spin text-muted-foreground/50" />
        </CardContent>
      </Card>
    );
  }

  const hitPercent = Math.max(0, Math.min(100, hitRate * 100));
  const hitPercentLabel = hitPercent.toFixed(hitPercent >= 99.95 ? 0 : 1);

  return (
    <motion.div
      initial={{ opacity: 0, y: 5 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.4 }}
    >
      <Card className="relative overflow-hidden border border-border/50 bg-card/60 backdrop-blur-xl shadow-sm">
        <CardContent className="p-4 md:p-5">
          <div className="flex flex-col gap-4">
            {/* Top row: Main Token Count, Requests, Cost */}
            <div className="flex flex-col md:flex-row md:items-center justify-between gap-4">
              <div className="flex items-center gap-3">
                <div
                  className={cn(
                    "p-2.5 rounded-xl bg-gradient-to-br shadow-sm",
                    titleTheme.iconBg,
                  )}
                >
                  <Zap className={cn("h-5 w-5", titleTheme.accent)} />
                </div>
                <div>
                  <div className="text-xs font-medium text-muted-foreground flex items-center gap-1.5 mb-0.5">
                    {appLabel && (
                      <>
                        <span
                          className={cn("font-semibold", titleTheme.accent)}
                        >
                          {appLabel}
                        </span>
                        <span className="text-muted-foreground/30">•</span>
                      </>
                    )}
                    {t("usage.realTotal", "Real token consumption")}
                  </div>
                  <div className="flex items-baseline gap-2">
                    <span
                      className="text-2xl md:text-3xl font-bold tabular-nums tracking-tight leading-none"
                      title={realTotal.toLocaleString()}
                    >
                      {realTotal.toLocaleString()}
                    </span>
                    <span className="text-xs text-muted-foreground font-medium bg-muted/40 px-1.5 py-0.5 rounded-md">
                      ≈ {formatTokensShort(realTotal, lang, 2)}
                    </span>
                  </div>
                </div>
              </div>

              <div className="flex items-center gap-5 bg-background/50 px-4 py-2.5 rounded-xl border border-border/40 shadow-sm">
                <div className="flex flex-col">
                  <span className="text-[10px] text-muted-foreground uppercase tracking-wider font-medium">
                    {t("usage.totalRequests")}
                  </span>
                  <span className="font-semibold flex items-center gap-1.5 text-sm tabular-nums">
                    <Activity className="h-3.5 w-3.5 text-blue-500" />
                    {requests.toLocaleString()}
                  </span>
                </div>
                <div className="w-px h-8 bg-border/60" />
                <div className="flex flex-col">
                  <span className="text-[10px] text-muted-foreground uppercase tracking-wider font-medium">
                    {t("usage.totalCost")}
                  </span>
                  <span className="font-semibold text-green-500 text-sm tabular-nums">
                    {totalCost == null ? "--" : fmtUsd(totalCost, 4)}
                  </span>
                </div>
              </div>
            </div>

            {/* Bottom row: Breakdown and Hit Rate */}
            <div className="grid grid-cols-2 lg:grid-cols-5 gap-3">
              <MiniStat
                icon={<ArrowDownToLine className="h-3.5 w-3.5" />}
                label={t("usage.freshInput", "Fresh input")}
                value={formatTokensShort(input, lang)}
                accent="text-blue-500"
              />
              <MiniStat
                icon={<ArrowUpFromLine className="h-3.5 w-3.5" />}
                label={t("usage.output")}
                value={formatTokensShort(output, lang)}
                accent="text-purple-500"
              />
              <MiniStat
                icon={<Database className="h-3.5 w-3.5" />}
                label={t("usage.cacheWrite", "Cache write")}
                value={cacheWriteDisplay.value}
                accent="text-amber-500"
                muted={cacheWriteDisplay.muted}
                tooltip={cacheWriteDisplay.tooltip}
              />
              <MiniStat
                icon={<Sparkles className="h-3.5 w-3.5" />}
                label={t("usage.cacheRead", "Cache hit")}
                value={formatTokensShort(cacheRead, lang)}
                accent="text-emerald-500"
              />

              <div className="col-span-2 lg:col-span-1 flex flex-col justify-center rounded-xl border border-border/40 bg-background/40 p-3 shadow-sm">
                <div className="flex items-center justify-between text-[11px] mb-2">
                  <span className="text-muted-foreground font-medium">
                    {t("usage.cacheHitRate", "Cache hit rate")}
                  </span>
                  <span className="font-bold text-emerald-500 tabular-nums">
                    {hitPercentLabel}%
                  </span>
                </div>
                <div className="relative h-1.5 rounded-full bg-muted/60 overflow-hidden">
                  <motion.div
                    className="absolute inset-y-0 left-0 bg-emerald-500 rounded-full"
                    initial={{ width: 0 }}
                    animate={{ width: `${hitPercent}%` }}
                    transition={{ duration: 0.8, ease: "easeOut" }}
                  />
                </div>
              </div>
            </div>
          </div>
        </CardContent>
      </Card>
    </motion.div>
  );
}

interface MiniStatProps {
  icon: ReactNode;
  label: string;
  value: string;
  accent: string;
  tooltip?: string;
  muted?: boolean;
}

function MiniStat({
  icon,
  label,
  value,
  accent,
  tooltip,
  muted,
}: MiniStatProps) {
  return (
    <div
      className="flex flex-col gap-1 rounded-xl border border-border/40 bg-background/40 p-3 shadow-sm"
      title={tooltip}
    >
      <div
        className={`flex items-center gap-1.5 text-[11px] font-medium text-muted-foreground ${accent}`}
      >
        {icon}
        <span className="text-foreground/70 tracking-wide">{label}</span>
        {tooltip && (
          <Info className="h-3 w-3 shrink-0 text-muted-foreground/60 ml-auto" />
        )}
      </div>
      <div
        className={cn(
          "text-sm font-semibold tabular-nums",
          muted && "text-muted-foreground/70",
        )}
      >
        {value}
      </div>
    </div>
  );
}
