import { describe, expect, it } from "vitest";

import { isValidListenAddress } from "@/components/proxy/ProxyPanel";

describe("isValidListenAddress", () => {
  it.each(["localhost", "0.0.0.0", "127.0.0.1", "::1", "::", "2001:db8::1"])(
    "accepts %s",
    (address) => {
      expect(isValidListenAddress(address)).toBe(true);
    },
  );

  it("trims surrounding whitespace", () => {
    expect(isValidListenAddress("  ::1  ")).toBe(true);
  });

  it.each(["", "example.com", "127.0.0.256", "127.0.0", "abc:def"])(
    "rejects %s",
    (address) => {
      expect(isValidListenAddress(address)).toBe(false);
    },
  );
});
