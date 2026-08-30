import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { generateUUID } from "@/utils/uuid";

/**
 * 复现环境：通过局域网 IP 以 HTTP 访问 Web 界面（非安全上下文）。
 * 此时 `globalThis.crypto.randomUUID` 为 undefined（Web 平台仅在
 * HTTPS/localhost 等 secure context 暴露），任何裸调用都会抛
 * `TypeError: crypto.randomUUID is not a function`。
 */
function removeRandomUUID(): () => void {
  const cryptoRef = globalThis.crypto as unknown as {
    randomUUID?: unknown;
  };
  const original = cryptoRef.randomUUID;
  Object.defineProperty(globalThis.crypto, "randomUUID", {
    value: undefined,
    configurable: true,
    writable: true,
  });
  return () => {
    if (typeof original !== "undefined") {
      Object.defineProperty(globalThis.crypto, "randomUUID", {
        value: original,
        configurable: true,
        writable: true,
      });
    }
  };
}

describe("generateUUID in non-secure contexts", () => {
  let restore: () => void;
  beforeEach(() => {
    restore = removeRandomUUID();
  });
  afterEach(() => restore());

  it("still produces a well-formed UUID v4 when crypto.randomUUID is unavailable", () => {
    const id = generateUUID();
    expect(id).toMatch(
      /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/,
    );
    expect(generateUUID()).not.toBe(id);
  });

  it("uses the native implementation when available", () => {
    restore(); // 恢复 secure-context 形态
    const id = generateUUID();
    expect(id).toMatch(/^[0-9a-f]{8}-[0-9a-f]{4}-4/);
  });
});
