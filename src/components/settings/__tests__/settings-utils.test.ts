// SPDX-License-Identifier: MIT

import { describe, expect, it } from "vitest";
import {
  formatAllowedSites,
  formatKdf,
  isThemePreference,
  parseAllowedSites,
} from "@/components/settings/settings-utils";

describe("settings-utils", () => {
  it("parses and normalizes allowed sites", () => {
    expect(parseAllowedSites(" example.com,foo.bar\n  \nsub.domain ")).toEqual([
      "example.com",
      "foo.bar",
      "sub.domain",
    ]);
  });

  it("formats allowed sites for textarea", () => {
    expect(formatAllowedSites(["example.com", "foo.bar"])).toBe(
      "example.com\nfoo.bar"
    );
  });

  it("formats AES-KDF settings", () => {
    expect(
      formatKdf({
        type: "aesKdf",
        rounds: 60000,
      })
    ).toBe("AES-KDF (60000 rounds)");
  });

  it("formats Argon2 KDF settings", () => {
    expect(
      formatKdf({
        type: "argon2id",
        memory: 65536,
        iterations: 4,
        parallelism: 2,
      })
    ).toBe("argon2id (65536 bytes, 4 iterations, 2 lanes)");
  });

  it("validates theme preferences", () => {
    expect(isThemePreference("system")).toBe(true);
    expect(isThemePreference("light")).toBe(true);
    expect(isThemePreference("dark")).toBe(true);
    expect(isThemePreference("sepia")).toBe(false);
  });
});
