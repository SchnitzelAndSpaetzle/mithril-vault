import dayjs, { type ManipulateType } from "dayjs";

/**
 * Duration presets a user can pick to set an Entry's expiry. Each resolves to
 * an absolute instant computed from "now" at the moment it is picked.
 */
export type ExpiryPreset =
  | "12h"
  | "24h"
  | "1w"
  | "2w"
  | "3w"
  | "1mo"
  | "2mo"
  | "3mo"
  | "6mo"
  | "1y"
  | "2y"
  | "3y";

/** Ordered list of presets for the editor dropdown. */
export const EXPIRY_PRESETS: ExpiryPreset[] = [
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
];

const PRESET_DURATIONS: Record<
  ExpiryPreset,
  { amount: number; unit: ManipulateType }
> = {
  "12h": { amount: 12, unit: "hour" },
  "24h": { amount: 24, unit: "hour" },
  "1w": { amount: 1, unit: "week" },
  "2w": { amount: 2, unit: "week" },
  "3w": { amount: 3, unit: "week" },
  "1mo": { amount: 1, unit: "month" },
  "2mo": { amount: 2, unit: "month" },
  "3mo": { amount: 3, unit: "month" },
  "6mo": { amount: 6, unit: "month" },
  "1y": { amount: 1, unit: "year" },
  "2y": { amount: 2, unit: "year" },
  "3y": { amount: 3, unit: "year" },
};

/**
 * Resolve a preset to an absolute instant `now + duration`. Pure: a given
 * (preset, now) always yields the same instant and never mutates `now`.
 */
export function resolveExpiryPreset(preset: ExpiryPreset, now: Date): Date {
  const { amount, unit } = PRESET_DURATIONS[preset];
  return dayjs(now).add(amount, unit).toDate();
}
