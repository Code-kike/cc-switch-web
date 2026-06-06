import { describe, expect, it } from "vitest";
import { generateUniqueProviderCopyKey } from "@/lib/providers/providerCopyKey";

describe("generateUniqueProviderCopyKey", () => {
  it("returns the bare -copy key when it is free", () => {
    expect(generateUniqueProviderCopyKey("foo", [])).toBe("foo-copy");
    expect(generateUniqueProviderCopyKey("foo", ["bar", "baz"])).toBe(
      "foo-copy",
    );
  });

  it("appends the first free numeric suffix starting at 2", () => {
    expect(generateUniqueProviderCopyKey("foo", ["foo-copy"])).toBe(
      "foo-copy-2",
    );
    expect(
      generateUniqueProviderCopyKey("foo", ["foo-copy", "foo-copy-2"]),
    ).toBe("foo-copy-3");
  });

  it("skips occupied suffixes and picks the first available gap", () => {
    expect(
      generateUniqueProviderCopyKey("foo", ["foo-copy", "foo-copy-3"]),
    ).toBe("foo-copy-2");
  });
});
