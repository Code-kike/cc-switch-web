import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Trash2, ExternalLink, Plus } from "lucide-react";
import { settingsApi } from "@/lib/api";
import { FullScreenPanel } from "@/components/common/FullScreenPanel";
import type { DiscoverableSkill, SkillRepo } from "@/lib/api/skills";
import { extractErrorMessage } from "@/utils/errorUtils";

interface RepoManagerPanelProps {
  repos: SkillRepo[];
  skills: DiscoverableSkill[];
  onAdd: (repo: SkillRepo) => Promise<void>;
  onRemove: (owner: string, name: string) => Promise<void>;
  onClose: () => void;
}

export function RepoManagerPanel({
  repos,
  skills,
  onAdd,
  onRemove,
  onClose,
}: RepoManagerPanelProps) {
  const { t } = useTranslation();
  const [repoUrl, setRepoUrl] = useState("");
  const [branch, setBranch] = useState("");
  const [formError, setFormError] = useState("");
  const [isAdding, setIsAdding] = useState(false);
  const [removingRepoKey, setRemovingRepoKey] = useState<string | null>(null);
  const [removeError, setRemoveError] = useState<{
    repoKey: string;
    message: string;
  } | null>(null);

  const getSkillCount = (repo: SkillRepo) =>
    skills.filter(
      (skill) =>
        skill.repoOwner === repo.owner &&
        skill.repoName === repo.name &&
        (skill.repoBranch || "main") === (repo.branch || "main"),
    ).length;

  const parseRepoUrl = (
    url: string,
  ): { owner: string; name: string } | null => {
    let cleaned = url.trim();
    cleaned = cleaned.replace(/^https?:\/\/github\.com\//, "");
    cleaned = cleaned.replace(/\.git$/, "");

    const parts = cleaned.split("/");
    if (parts.length === 2 && parts[0] && parts[1]) {
      return { owner: parts[0], name: parts[1] };
    }

    return null;
  };

  const parsedRepo = parseRepoUrl(repoUrl);
  const normalizedBranch = branch.trim() || "main";
  const duplicateRepo = parsedRepo
    ? repos.find(
        (repo) => repo.owner === parsedRepo.owner && repo.name === parsedRepo.name,
      )
    : undefined;

  const handleAdd = async () => {
    setFormError("");
    setRemoveError(null);

    if (!parsedRepo) {
      setFormError(t("skills.repo.invalidUrl"));
      return;
    }

    setIsAdding(true);
    try {
      await onAdd({
        owner: parsedRepo.owner,
        name: parsedRepo.name,
        branch: normalizedBranch,
        enabled: true,
      });

      setRepoUrl("");
      setBranch("");
    } catch (e) {
      setFormError(extractErrorMessage(e) || t("skills.repo.addFailed"));
    } finally {
      setIsAdding(false);
    }
  };

  const handleRemove = async (owner: string, name: string) => {
    const repoKey = `${owner}/${name}`;
    setFormError("");
    setRemoveError(null);
    setRemovingRepoKey(repoKey);

    try {
      await onRemove(owner, name);
    } catch (error) {
      setRemoveError({
        repoKey,
        message: extractErrorMessage(error) || t("skills.repo.removeFailed"),
      });
    } finally {
      setRemovingRepoKey((current) => (current === repoKey ? null : current));
    }
  };

  const handleOpenRepo = async (owner: string, name: string) => {
    try {
      await settingsApi.openExternal(`https://github.com/${owner}/${name}`);
    } catch (error) {
      console.error("Failed to open URL:", error);
    }
  };

  return (
    <FullScreenPanel
      isOpen={true}
      title={t("skills.repo.title")}
      onClose={onClose}
    >
      {/* 添加仓库表单 */}
      <div className="space-y-4 glass-card rounded-xl p-6">
        <h3 className="text-base font-semibold text-foreground">
          {t("skills.addRepo")}
        </h3>
        <div className="space-y-4">
          <div>
            <Label htmlFor="repo-url" className="text-foreground">
              {t("skills.repo.url")}
            </Label>
            <Input
              id="repo-url"
              placeholder={t("skills.repo.urlPlaceholder")}
              value={repoUrl}
              onChange={(e) => {
                setRepoUrl(e.target.value);
                if (formError) {
                  setFormError("");
                }
              }}
              disabled={isAdding}
              className="mt-2"
            />
          </div>
          <div>
            <Label htmlFor="branch" className="text-foreground">
              {t("skills.repo.branch")}
            </Label>
            <Input
              id="branch"
              placeholder={t("skills.repo.branchPlaceholder")}
              value={branch}
              onChange={(e) => {
                setBranch(e.target.value);
                if (formError) {
                  setFormError("");
                }
              }}
              disabled={isAdding}
              className="mt-2"
            />
          </div>
          {duplicateRepo && (
            <p
              role="status"
              className="text-sm text-amber-700 dark:text-amber-300"
            >
              {t("skills.repo.duplicateWarning", {
                owner: duplicateRepo.owner,
                name: duplicateRepo.name,
              })}
            </p>
          )}
          {formError && (
            <p role="alert" className="text-sm text-red-600 dark:text-red-400">
              {formError}
            </p>
          )}
          <Button
            onClick={handleAdd}
            className="bg-primary text-primary-foreground hover:bg-primary/90"
            type="button"
            disabled={isAdding}
          >
            <Plus className="h-4 w-4 mr-2" />
            {isAdding ? t("common.saving") : t("skills.repo.add")}
          </Button>
        </div>
      </div>

      {/* 仓库列表 */}
      <div className="space-y-4">
        <h3 className="text-base font-semibold text-foreground">
          {t("skills.repo.list")}
        </h3>
        {repos.length === 0 ? (
          <div className="text-center py-12 glass-card rounded-xl">
            <p className="text-sm text-muted-foreground">
              {t("skills.repo.empty")}
            </p>
          </div>
        ) : (
          <div className="space-y-3">
            {repos.map((repo) => {
              const repoKey = `${repo.owner}/${repo.name}`;
              const isRemoving = removingRepoKey === repoKey;

              return (
                <div key={repoKey} className="space-y-2">
                  <div className="flex items-center justify-between glass-card rounded-xl px-4 py-3">
                    <div>
                      <div className="text-sm font-medium text-foreground">
                        {repo.owner}/{repo.name}
                      </div>
                      <div className="mt-1 text-xs text-muted-foreground">
                        {t("skills.repo.branch")}: {repo.branch || "main"}
                        <span className="ml-3 inline-flex items-center rounded-full border border-border-default px-2 py-0.5 text-[11px]">
                          {t("skills.repo.skillCount", {
                            count: getSkillCount(repo),
                          })}
                        </span>
                      </div>
                    </div>
                    <div className="flex gap-2">
                      <Button
                        variant="ghost"
                        size="icon"
                        type="button"
                        onClick={() => handleOpenRepo(repo.owner, repo.name)}
                        title={t("common.view", { defaultValue: "查看" })}
                        aria-label={t("common.view")}
                        disabled={isRemoving}
                        className="hover:bg-black/5 dark:hover:bg-white/5"
                      >
                        <ExternalLink className="h-4 w-4" />
                      </Button>
                      <Button
                        variant="ghost"
                        size="icon"
                        type="button"
                        onClick={() => void handleRemove(repo.owner, repo.name)}
                        title={isRemoving ? t("common.deleting") : t("common.delete")}
                        aria-label={
                          isRemoving ? t("common.deleting") : t("common.delete")
                        }
                        disabled={isRemoving}
                        className="hover:text-red-500 hover:bg-red-100 dark:hover:text-red-400 dark:hover:bg-red-500/10"
                      >
                        <Trash2 className="h-4 w-4" />
                      </Button>
                    </div>
                  </div>
                  {removeError?.repoKey === repoKey ? (
                    <p
                      role="alert"
                      className="px-4 text-sm text-red-600 dark:text-red-400"
                    >
                      {removeError.message}
                    </p>
                  ) : null}
                </div>
              );
            })}
          </div>
        )}
      </div>
    </FullScreenPanel>
  );
}
