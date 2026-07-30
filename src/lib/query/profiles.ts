import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { profilesApi, providersApi } from "@/lib/api";
import { isWebMode } from "@/lib/api/adapter";
import type { ProfileScope } from "@/lib/api/profiles";
import { extractErrorMessage } from "@/utils/errorUtils";

export const profileKeys = {
  all: ["profiles"] as const,
};

const updateTrayMenuSafely = async () => {
  // The standalone Web runtime has no tray. Avoid invoking a deliberately
  // unsupported command after an otherwise-successful profile mutation.
  if (isWebMode()) return;
  try {
    await providersApi.updateTrayMenu();
  } catch (trayError) {
    console.error("Failed to update tray menu after profile change", trayError);
  }
};

export const useProfilesQuery = () => {
  return useQuery({
    queryKey: profileKeys.all,
    queryFn: () => profilesApi.list(),
  });
};

export const useCreateProfileMutation = () => {
  const queryClient = useQueryClient();
  const { t } = useTranslation();

  return useMutation({
    mutationFn: ({ name, scope }: { name: string; scope: ProfileScope }) =>
      profilesApi.create(name, scope),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: profileKeys.all });
      await updateTrayMenuSafely();
      toast.success(t("profiles.createSuccess"), { closeButton: true });
    },
    onError: (error: Error) => {
      const detail = extractErrorMessage(error) || t("common.unknown");
      toast.error(t("profiles.createFailed", { detail }), {
        closeButton: true,
      });
    },
  });
};

export const useUpdateProfileMutation = () => {
  const queryClient = useQueryClient();
  const { t } = useTranslation();

  return useMutation({
    mutationFn: ({
      id,
      name,
      resnapshot,
      scope,
    }: {
      id: string;
      name?: string;
      resnapshot?: boolean;
      scope?: ProfileScope;
    }) => profilesApi.update(id, { name, resnapshot, scope }),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: profileKeys.all });
      await updateTrayMenuSafely();
      toast.success(t("profiles.updateSuccess"), { closeButton: true });
    },
    onError: (error: Error) => {
      const detail = extractErrorMessage(error) || t("common.unknown");
      toast.error(t("profiles.updateFailed", { detail }), {
        closeButton: true,
      });
    },
  });
};

export const useDeleteProfileMutation = () => {
  const queryClient = useQueryClient();
  const { t } = useTranslation();

  return useMutation({
    mutationFn: (id: string) => profilesApi.delete(id),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: profileKeys.all });
      await updateTrayMenuSafely();
      toast.success(t("profiles.deleteSuccess"), { closeButton: true });
    },
    onError: (error: Error) => {
      const detail = extractErrorMessage(error) || t("common.unknown");
      toast.error(t("profiles.deleteFailed", { detail }), {
        closeButton: true,
      });
    },
  });
};

export const useClearProfileMutation = () => {
  const queryClient = useQueryClient();
  const { t } = useTranslation();

  return useMutation({
    mutationFn: (scope: ProfileScope) => profilesApi.clearCurrent(scope),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: profileKeys.all });
      await updateTrayMenuSafely();
      toast.success(t("profiles.clearSuccess"), { closeButton: true });
    },
    onError: (error: Error) => {
      const detail = extractErrorMessage(error) || t("common.unknown");
      toast.error(t("profiles.applyFailed", { detail }), {
        closeButton: true,
      });
    },
  });
};

export const useApplyProfileMutation = () => {
  const queryClient = useQueryClient();
  const { t } = useTranslation();

  return useMutation({
    mutationFn: ({ id, scope }: { id: string; scope: ProfileScope }) =>
      profilesApi.apply(id, scope),
    onSuccess: async (warnings, { scope }) => {
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: profileKeys.all }),
        queryClient.invalidateQueries({ queryKey: ["providers", scope] }),
        queryClient.invalidateQueries({ queryKey: ["mcp", "all"] }),
        queryClient.invalidateQueries({ queryKey: ["skills"] }),
        queryClient.invalidateQueries({ queryKey: ["proxyTakeoverStatus"] }),
        queryClient.invalidateQueries({ queryKey: ["proxyStatus"] }),
      ]);
      await updateTrayMenuSafely();

      if (warnings.length > 0) {
        toast.warning(
          t("profiles.applyWarnings", {
            warningCount: warnings.length,
            details: warnings.join("\n"),
          }),
          { closeButton: true, duration: 10000 },
        );
      } else {
        toast.success(t("profiles.applySuccess"), { closeButton: true });
      }
    },
    onError: (error: Error) => {
      const detail = extractErrorMessage(error) || t("common.unknown");
      toast.error(t("profiles.applyFailed", { detail }), {
        closeButton: true,
      });
    },
  });
};
