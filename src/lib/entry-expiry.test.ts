import { describe, expect, it } from "vitest";
import dayjs from "dayjs";
import {
  EXPIRY_PRESETS,
  type ExpiryPreset,
  isExpired,
  resolveExpiryPreset,
} from "@/lib/entry-expiry";

const HOUR_MS = 60 * 60 * 1000;
const DAY_MS = 24 * HOUR_MS;

// Fixed reference instant, built in local time so calendar assertions are
// timezone-independent.
const NOW = new Date(2026, 0, 15, 10, 30, 0, 0);

describe("resolveExpiryPreset", () => {
  it("exposes all twelve presets in order", () => {
    expect(EXPIRY_PRESETS).toEqual([
      "12h",
      "24h",
      "1w",
      "2w",
      "3w",
      "1mo",
      "2mo",
      "3mo",
      "6mo",
      "1y",
      "2y",
      "3y",
    ]);
  });

  it.each<[ExpiryPreset, number]>([
    ["12h", 12 * HOUR_MS],
    ["24h", 24 * HOUR_MS],
    ["1w", 7 * DAY_MS],
    ["2w", 14 * DAY_MS],
    ["3w", 21 * DAY_MS],
  ])("resolves %s as a fixed duration from now", (preset, deltaMs) => {
    const result = resolveExpiryPreset(preset, NOW);
    expect(result.getTime() - NOW.getTime()).toBe(deltaMs);
  });

  it.each<[ExpiryPreset, number, number]>([
    // preset, expected calendar year, expected calendar month (0-based)
    ["1mo", 2026, 1],
    ["2mo", 2026, 2],
    ["3mo", 2026, 3],
    ["6mo", 2026, 6],
    ["1y", 2027, 0],
    ["2y", 2028, 0],
    ["3y", 2029, 0],
  ])(
    "resolves %s by advancing the local calendar, keeping time-of-day",
    (preset, year, month) => {
      const result = dayjs(resolveExpiryPreset(preset, NOW));
      expect(result.year()).toBe(year);
      expect(result.month()).toBe(month);
      expect(result.date()).toBe(15);
      expect(result.hour()).toBe(10);
      expect(result.minute()).toBe(30);
    }
  );

  it("is pure: same now produces the same instant", () => {
    const a = resolveExpiryPreset("1y", NOW);
    const b = resolveExpiryPreset("1y", NOW);
    expect(a.getTime()).toBe(b.getTime());
  });

  it("does not mutate the provided now", () => {
    const before = NOW.getTime();
    resolveExpiryPreset("6mo", NOW);
    expect(NOW.getTime()).toBe(before);
  });
});

describe("isExpired", () => {
  const past = dayjs(NOW).subtract(1, "day").toISOString();
  const future = dayjs(NOW).add(1, "day").toISOString();

  it("is true when the flag is set and the expiry is in the past", () => {
    expect(isExpired({ expires: true, expiryTime: past }, NOW)).toBe(true);
  });

  it("is false when the flag is set but the expiry is in the future", () => {
    expect(isExpired({ expires: true, expiryTime: future }, NOW)).toBe(false);
  });

  it("is false when the flag is not set, even with a past expiry", () => {
    expect(isExpired({ expires: false, expiryTime: past }, NOW)).toBe(false);
  });

  it("is false when the expiry timestamp is missing", () => {
    expect(isExpired({ expires: true, expiryTime: null }, NOW)).toBe(false);
    expect(isExpired({ expires: true, expiryTime: undefined }, NOW)).toBe(
      false
    );
  });
});
