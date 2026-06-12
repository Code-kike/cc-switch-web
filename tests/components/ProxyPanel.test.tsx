import { describe, expect, it } from "vitest";

import {
  isLoopbackAddress,
  isValidListenAddress,
} from "@/components/proxy/ProxyPanel";

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

describe("isLoopbackAddress", () => {
  // Mirrors the backend D4 rule (ProxyService::ensure_loopback_listen_address_for_web):
  // web mode only allows loopback listen addresses for the routing proxy.
  it.each(["localhost", "127.0.0.1", "127.1.2.3", "::1", "  127.0.0.1  "])(
    "accepts %s",
    (address) => {
      expect(isLoopbackAddress(address)).toBe(true);
    },
  );

  it.each(["0.0.0.0", "::", "192.168.1.10", "10.0.0.1", "example.com", ""])(
    "rejects %s",
    (address) => {
      expect(isLoopbackAddress(address)).toBe(false);
    },
  );
});
