import { beforeEach, describe, expect, it, vi } from "vitest";

const invokeMock = vi.hoisted(() => vi.fn());
const isWebModeMock = vi.hoisted(() => vi.fn(() => true));
const pickWebFileMock = vi.hoisted(() => vi.fn());
const webUploadMock = vi.hoisted(() => vi.fn());
const webDownloadMock = vi.hoisted(() => vi.fn());
const downloadBlobMock = vi.hoisted(() => vi.fn());

vi.mock("@/lib/api/adapter", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api/adapter")>(
    "@/lib/api/adapter",
  );

  return {
    ...actual,
    invoke: (...args: unknown[]) => invokeMock(...args),
    isWebMode: () => isWebModeMock(),
    pickWebFile: (...args: unknown[]) => pickWebFileMock(...args),
    webUpload: (...args: unknown[]) => webUploadMock(...args),
    webDownload: (...args: unknown[]) => webDownloadMock(...args),
    downloadBlob: (...args: unknown[]) => downloadBlobMock(...args),
  };
});

import { promptsApi } from "@/lib/api/prompts";
import { settingsApi } from "@/lib/api/settings";
import { skillsApi } from "@/lib/api/skills";

describe("web file replacement APIs", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    isWebModeMock.mockReset();
    isWebModeMock.mockReturnValue(true);
    pickWebFileMock.mockReset();
    webUploadMock.mockReset();
    webDownloadMock.mockReset();
    downloadBlobMock.mockReset();
  });

  it("uses browser file picking for settings SQL import selection", async () => {
    const sqlFile = new File(["select 1;"], "backup.sql", {
      type: "application/sql",
    });
    pickWebFileMock.mockResolvedValue(sqlFile);

    const result = await settingsApi.openFileDialog();

    expect(result).toEqual({ name: "backup.sql", file: sqlFile });
    expect(pickWebFileMock).toHaveBeenCalledWith(
      ".sql,text/sql,application/sql",
    );
    expect(invokeMock).not.toHaveBeenCalled();
  });

  it("uses browser upload for settings SQL import in web mode", async () => {
    const sqlFile = new File(["select 1;"], "backup.sql", {
      type: "application/sql",
    });
    webUploadMock.mockResolvedValue({
      success: true,
      message: "imported",
      backupId: "backup-123",
    });

    const result = await settingsApi.importConfigFromFile({
      name: sqlFile.name,
      file: sqlFile,
    });

    expect(webUploadMock).toHaveBeenCalledWith(
      "/api/config/import-config-upload",
      expect.any(FormData),
    );
    const formData = webUploadMock.mock.calls[0]?.[1] as FormData;
    expect(formData.get("file")).toBe(sqlFile);
    expect(result).toEqual({
      success: true,
      message: "imported",
      backupId: "backup-123",
    });
    expect(invokeMock).not.toHaveBeenCalled();
  });

  it("uses browser download for settings SQL export in web mode", async () => {
    const blob = new Blob(["backup"], { type: "application/sql" });
    webDownloadMock.mockResolvedValue(blob);

    const result = await settingsApi.exportConfigToFile(
      "cc-switch-export.sql",
    );

    expect(await settingsApi.saveFileDialog("cc-switch-export.sql")).toBe(
      "cc-switch-export.sql",
    );
    expect(webDownloadMock).toHaveBeenCalledWith(
      "/api/config/export-config-download",
    );
    expect(downloadBlobMock).toHaveBeenCalledWith(blob, "cc-switch-export.sql");
    expect(result).toEqual({
      success: true,
      message: "SQL exported successfully",
      filePath: "cc-switch-export.sql",
    });
    expect(invokeMock).not.toHaveBeenCalled();
  });

  it("uses browser upload for prompt import in web mode", async () => {
    const promptFile = new File(["# prompt"], "PROMPT.md", {
      type: "text/markdown",
    });
    pickWebFileMock.mockResolvedValue(promptFile);
    webUploadMock.mockResolvedValue("imported-prompt-id");

    const result = await promptsApi.importFromFile("gemini");

    expect(pickWebFileMock).toHaveBeenCalledWith(
      ".md,text/markdown,text/plain",
    );
    expect(webUploadMock).toHaveBeenCalledWith(
      "/api/prompts/import-prompt-upload?app=gemini",
      expect.any(FormData),
    );
    const formData = webUploadMock.mock.calls[0]?.[1] as FormData;
    expect(formData.get("file")).toBe(promptFile);
    expect(result).toBe("imported-prompt-id");
    expect(invokeMock).not.toHaveBeenCalled();
  });

  it("returns null for prompt import cancellation in web mode", async () => {
    pickWebFileMock.mockResolvedValue(null);

    const result = await promptsApi.importFromFile("claude");

    expect(result).toBeNull();
    expect(webUploadMock).not.toHaveBeenCalled();
    expect(invokeMock).not.toHaveBeenCalled();
  });

  it("uses browser file picking and upload for skills ZIP install", async () => {
    const zipFile = new File(["zip"], "skills.zip", {
      type: "application/zip",
    });
    pickWebFileMock.mockResolvedValue(zipFile);
    webUploadMock.mockResolvedValue([
      { id: "skill-1", name: "Skill 1" },
      { id: "skill-2", name: "Skill 2" },
    ]);

    const picked = await skillsApi.openZipFileDialog();
    const installed = await skillsApi.installFromZip(zipFile, "openclaw");

    expect(picked).toBe(zipFile);
    expect(pickWebFileMock).toHaveBeenCalledWith(".zip,.skill,application/zip");
    expect(webUploadMock).toHaveBeenCalledWith(
      "/api/skills/install-skills-upload?app=openclaw",
      expect.any(FormData),
    );
    const formData = webUploadMock.mock.calls[0]?.[1] as FormData;
    expect(formData.get("file")).toBe(zipFile);
    expect(installed).toEqual([
      { id: "skill-1", name: "Skill 1" },
      { id: "skill-2", name: "Skill 2" },
    ]);
    expect(invokeMock).not.toHaveBeenCalled();
  });

  it("does not fall back to desktop ZIP install when the value is not a browser File", async () => {
    const installed = await skillsApi.installFromZip("/tmp/skills.zip", "claude");

    expect(installed).toEqual([]);
    expect(webUploadMock).not.toHaveBeenCalled();
    expect(invokeMock).not.toHaveBeenCalled();
  });
});
