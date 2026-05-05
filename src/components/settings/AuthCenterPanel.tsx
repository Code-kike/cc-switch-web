import { Github, Info, ShieldCheck } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Badge } from "@/components/ui/badge";
import { CodexIcon } from "@/components/BrandIcons";
import { CopilotAuthSection } from "@/components/providers/forms/CopilotAuthSection";
import { CodexOAuthSection } from "@/components/providers/forms/CodexOAuthSection";
import { isWebMode } from "@/lib/api/adapter";

export function AuthCenterPanel() {
  const { t } = useTranslation();
  const webMode = isWebMode();

  return (
    <div className="space-y-6">
      <section className="rounded-xl border border-border/60 bg-card/60 p-6">
        <div className="flex items-start justify-between gap-4">
          <div className="space-y-2">
            <div className="flex items-center gap-2">
              <ShieldCheck className="h-5 w-5 text-primary" />
              <h3 className="text-base font-semibold">
                {t("settings.authCenter.title", {
                  defaultValue: "OAuth 认证中心",
                })}
              </h3>
            </div>
            <p className="text-sm text-muted-foreground">
              {t("settings.authCenter.description", {
                defaultValue:
                  "在 Claude Code 中使用您的其他订阅，请注意合规风险。",
              })}
            </p>
          </div>
          <Badge variant="secondary">
            {t("settings.authCenter.beta", { defaultValue: "Beta" })}
          </Badge>
        </div>
      </section>

      {webMode && (
        <section className="rounded-xl border border-amber-500/30 bg-amber-500/10 p-4">
          <div className="flex items-start gap-3">
            <Info className="mt-0.5 h-4 w-4 shrink-0 text-amber-700 dark:text-amber-400" />
            <div className="space-y-1 text-sm">
              <p className="font-medium text-amber-900 dark:text-amber-200">
                {t("settings.authCenter.webRemoteHint", {
                  defaultValue: "远程 Web 模式会把 OAuth 账号保存到服务端机器",
                })}
              </p>
              <p className="text-amber-800/90 dark:text-amber-300/90">
                {t("settings.authCenter.webRemoteHintDescription", {
                  defaultValue:
                    "授权页面会在当前浏览器打开，但登录成功后的 Copilot / ChatGPT 账号会绑定到运行 cc-switch Web 的那台机器。仅在您信任该服务端时继续。",
                })}
              </p>
            </div>
          </div>
        </section>
      )}

      <section className="rounded-xl border border-border/60 bg-card/60 p-6">
        <div className="mb-4 flex items-center gap-3">
          <div className="flex h-10 w-10 items-center justify-center rounded-xl bg-muted">
            <Github className="h-5 w-5" />
          </div>
          <div>
            <h4 className="font-medium">GitHub Copilot</h4>
            <p className="text-sm text-muted-foreground">
              {t("settings.authCenter.copilotDescription", {
                defaultValue: "管理 GitHub Copilot 账号",
              })}
            </p>
          </div>
        </div>

        <CopilotAuthSection />
      </section>

      <section className="rounded-xl border border-border/60 bg-card/60 p-6">
        <div className="mb-4 flex items-center gap-3">
          <div className="flex h-10 w-10 items-center justify-center rounded-xl bg-muted">
            <CodexIcon size={20} />
          </div>
          <div>
            <h4 className="font-medium">ChatGPT (Codex OAuth)</h4>
            <p className="text-sm text-muted-foreground">
              {t("settings.authCenter.codexOauthDescription", {
                defaultValue: "管理 ChatGPT 账号",
              })}
            </p>
          </div>
        </div>

        <CodexOAuthSection />
      </section>
    </div>
  );
}
