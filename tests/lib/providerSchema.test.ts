import { describe, it, expect } from "vitest";
import {
  providerSchema,
  providerFormSchema,
} from "@/lib/schemas/provider";

describe("providerSchema name requirement (M40)", () => {
  it("rejects an empty name", () => {
    const result = providerSchema.safeParse({
      name: "",
      settingsConfig: "{}",
    });
    expect(result.success).toBe(false);
    if (result.success) return;
    expect(result.error.issues.some((i) => i.path[0] === "name")).toBe(true);
  });

  it("rejects a whitespace-only name", () => {
    const result = providerSchema.safeParse({
      name: "   ",
      settingsConfig: "{}",
    });
    expect(result.success).toBe(false);
    if (result.success) return;
    expect(result.error.issues.some((i) => i.path[0] === "name")).toBe(true);
  });

  it("accepts a non-empty name", () => {
    const result = providerSchema.safeParse({
      name: "My Provider",
      settingsConfig: "{}",
    });
    expect(result.success).toBe(true);
  });

  it("does not mutate (trim) the submitted name value", () => {
    const result = providerSchema.safeParse({
      name: "  Padded  ",
      settingsConfig: "{}",
    });
    expect(result.success).toBe(true);
    if (!result.success) return;
    // 仅校验 trim 后非空，不改写原值（去空白由 handleSubmit 显式完成）
    expect(result.data.name).toBe("  Padded  ");
  });
});

describe("providerFormSchema (react-hook-form resolver) keeps name soft", () => {
  it("accepts an empty name so the soft-confirm flow can run", () => {
    const result = providerFormSchema.safeParse({
      name: "",
      settingsConfig: "{}",
    });
    expect(result.success).toBe(true);
  });

  it("still enforces settingsConfig rules (must be a JSON object)", () => {
    const result = providerFormSchema.safeParse({
      name: "",
      settingsConfig: "[1,2,3]",
    });
    expect(result.success).toBe(false);
  });
});

describe("providerSchema settingsConfig validation", () => {
  const withConfig = (settingsConfig: string) =>
    providerSchema.safeParse({ name: "Valid", settingsConfig });

  it("rejects an empty settingsConfig", () => {
    expect(withConfig("").success).toBe(false);
  });

  it("rejects invalid JSON", () => {
    expect(withConfig("{not json").success).toBe(false);
  });

  it("rejects a JSON array", () => {
    expect(withConfig("[1,2,3]").success).toBe(false);
  });

  it("rejects a JSON primitive", () => {
    expect(withConfig("123").success).toBe(false);
  });

  it("accepts a JSON object", () => {
    expect(withConfig('{"env":{}}').success).toBe(true);
  });
});
