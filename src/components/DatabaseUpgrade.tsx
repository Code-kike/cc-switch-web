import { useState } from "react";
import { useTranslation } from "react-i18next";
import {
  AlertTriangle,
  Database,
  ExternalLink,
  FolderOpen,
  LogOut,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { invoke, isWebMode } from "@/lib/api/adapter";

const RELEASES_URL = "https://github.com/farion1231/cc-switch/releases";

interface DatabaseUpgradeProps {
  payload: {
    path?: string;
    error?: string;
    kind?: string;
    db_version?: number;
    supported_version?: number;
  };
}

export function DatabaseUpgrade({ payload }: DatabaseUpgradeProps) {
  const { t } = useTranslation();
  const webMode = isWebMode();
  const [actionError, setActionError] = useState<string | null>(null);

  const dbVersion = payload.db_version;
  const supportedVersion = payload.supported_version;

  const openReleases = async () => {
    setActionError(null);
    if (webMode) {
      window.open(RELEASES_URL, "_blank", "noopener,noreferrer");
      return;
    }
    try {
      await invoke("open_external", { url: RELEASES_URL });
    } catch (error) {
      setActionError(error instanceof Error ? error.message : String(error));
    }
  };

  const openConfigDir = async () => {
    setActionError(null);
    try {
      await invoke("open_app_config_folder");
    } catch (error) {
      setActionError(error instanceof Error ? error.message : String(error));
    }
  };

  const quit = async () => {
    if (webMode) return;
    const { exit } = await import("@tauri-apps/plugin-process");
    await exit(0);
  };

  return (
    <div className="flex min-h-screen items-center justify-center bg-background p-6 text-foreground">
      <div className="w-full max-w-xl space-y-5 rounded-lg border border-border bg-card p-6 shadow-lg">
        <div className="flex items-start gap-4">
          <div className="flex h-11 w-11 shrink-0 items-center justify-center rounded-lg bg-amber-100 text-amber-700 dark:bg-amber-950/50 dark:text-amber-300">
            <Database className="h-5 w-5" />
          </div>
          <div className="min-w-0 space-y-1">
            <h1 className="text-lg font-semibold">
              {t("dbUpgrade.title", "Database Version Is Too New")}
            </h1>
            <p className="text-sm leading-6 text-muted-foreground">
              {webMode
                ? t(
                    "dbUpgrade.webDescription",
                    "This data directory was opened by a newer schema. Upgrade the Web service before starting it again; the database was left untouched.",
                  )
                : t(
                    "dbUpgrade.description",
                    "This database was created by a newer version of CC Switch. Upgrade the app before continuing; the database was left untouched.",
                  )}
            </p>
          </div>
        </div>

        {dbVersion != null && supportedVersion != null && (
          <div className="grid grid-cols-2 gap-3 rounded-lg border border-border/70 bg-muted/35 p-3 text-sm">
            <div>
              <div className="text-xs text-muted-foreground">
                {t("dbUpgrade.dbVersion", "Database")}
              </div>
              <div className="font-mono">v{dbVersion}</div>
            </div>
            <div>
              <div className="text-xs text-muted-foreground">
                {t("dbUpgrade.supportedVersion", "Supported")}
              </div>
              <div className="font-mono">v{supportedVersion}</div>
            </div>
          </div>
        )}

        <div className="space-y-2 rounded-lg border border-border/70 bg-muted/35 p-3 text-xs text-muted-foreground">
          {payload.error && (
            <p className="break-words font-mono leading-5">{payload.error}</p>
          )}
          {payload.path && (
            <p className="break-all">
              {t("dbUpgrade.dbPath", "Database file")}: {payload.path}
            </p>
          )}
        </div>

        {webMode && (
          <div className="flex gap-3 rounded-lg border border-amber-300/70 bg-amber-50 p-3 text-sm text-amber-900 dark:border-amber-500/40 dark:bg-amber-950/30 dark:text-amber-200">
            <AlertTriangle className="mt-0.5 h-4 w-4 shrink-0" />
            <p className="leading-6">
              {t(
                "dbUpgrade.webOperatorNote",
                "Stop the service, install a build that supports this schema, then start it with the same data directory. Do not downgrade or edit the SQLite user_version manually.",
              )}
            </p>
          </div>
        )}

        {actionError && (
          <p className="rounded-lg border border-red-300/70 bg-red-50 p-3 text-sm text-red-700 dark:border-red-500/40 dark:bg-red-950/30 dark:text-red-300">
            {actionError}
          </p>
        )}

        <div className="flex flex-wrap items-center gap-2">
          <Button onClick={openReleases} className="gap-2">
            <ExternalLink className="h-4 w-4" />
            {t("dbUpgrade.openReleases", "Open Releases")}
          </Button>
          {!webMode && (
            <Button variant="outline" onClick={openConfigDir} className="gap-2">
              <FolderOpen className="h-4 w-4" />
              {t("dbUpgrade.openConfigDir", "Open Config Folder")}
            </Button>
          )}
          {!webMode && (
            <Button variant="ghost" onClick={quit} className="ml-auto gap-2">
              <LogOut className="h-4 w-4" />
              {t("dbUpgrade.quit", "Quit")}
            </Button>
          )}
        </div>
      </div>
    </div>
  );
}

export default DatabaseUpgrade;
