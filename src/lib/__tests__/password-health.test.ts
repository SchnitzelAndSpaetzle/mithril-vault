// SPDX-License-Identifier: MIT

import { describe, expect, it } from "vitest";

import type { PasswordHealthReport } from "@/lib/types";
import { findingsForEntry, severityOf, summarize } from "@/lib/password-health";

const emptyReport: PasswordHealthReport = {
  score: null,
  findings: [],
  totals: { critical: 0, high: 0, healthy: 0, total: 0 },
  reuseGroups: [],
};

const noFindingsReport: PasswordHealthReport = {
  score: 100,
  findings: [],
  totals: { critical: 0, high: 0, healthy: 4, total: 4 },
  reuseGroups: [],
};

const allHighReport: PasswordHealthReport = {
  score: 0,
  findings: [
    { entryId: "a", kind: "password.expired" },
    { entryId: "b", kind: "password.expired" },
    { entryId: "c", kind: "password.expired" },
  ],
  totals: { critical: 0, high: 3, healthy: 0, total: 3 },
  reuseGroups: [],
};

describe("summarize", () => {
  it("returns zero unhealthy and null severity for an empty report", () => {
    expect(summarize(emptyReport)).toEqual({
      totalUnhealthy: 0,
      highestSeverity: null,
    });
  });

  it("returns zero unhealthy and null severity when there are no findings", () => {
    expect(summarize(noFindingsReport)).toEqual({
      totalUnhealthy: 0,
      highestSeverity: null,
    });
  });

  it("returns the un-healthy Entry count and highest severity when every Finding is High", () => {
    expect(summarize(allHighReport)).toEqual({
      totalUnhealthy: 3,
      highestSeverity: "high",
    });
  });

  // The sidebar passes `report` straight from React Query; a not-yet-
  // loaded report is `null` / `undefined`. The sidebar reads
  // `totalUnhealthy === 0` to hide the badge, so this code path has
  // to behave like an empty report rather than throwing.
  it("handles a null/undefined report as zero-unhealthy", () => {
    expect(summarize(null)).toEqual({
      totalUnhealthy: 0,
      highestSeverity: null,
    });
    expect(summarize(undefined)).toEqual({
      totalUnhealthy: 0,
      highestSeverity: null,
    });
  });
});

describe("severityOf", () => {
  it("maps password.expired to high", () => {
    expect(severityOf("password.expired")).toBe("high");
  });

  it("maps password.very_weak to critical", () => {
    expect(severityOf("password.very_weak")).toBe("critical");
  });

  it("maps password.weak to high", () => {
    expect(severityOf("password.weak")).toBe("high");
  });

  // Pinning the wire identifier *and* the severity here catches both
  // a backend rename (the Zod schema would reject the parse) and a
  // future bucket reshuffle that would put reused into Critical and
  // bring it to parity with very_weak in the report-view layout.
  it("maps password.reused to high", () => {
    expect(severityOf("password.reused")).toBe("high");
  });
});

describe("summarize with critical findings", () => {
  // The sidebar badge picks its colour from `highestSeverity`. The
  // moment any Critical Finding shows up in the report, the colour
  // flips from amber to red. Pin the rule independently of how many
  // High Findings the Vault also has — Critical wins.
  const criticalAndHighReport: PasswordHealthReport = {
    score: 50,
    findings: [
      { entryId: "a", kind: "password.very_weak" },
      { entryId: "b", kind: "password.expired" },
    ],
    totals: { critical: 1, high: 1, healthy: 0, total: 2 },
    reuseGroups: [],
  };

  // An Entry that is both very_weak and expired (its own two
  // Findings) must count once in `totalUnhealthy` and must be
  // labelled Critical because that is the more severe of its two
  // Findings. This is the per-Entry tie-breaker.
  const sameEntryBothFindings: PasswordHealthReport = {
    score: 0,
    findings: [
      { entryId: "a", kind: "password.expired" },
      { entryId: "a", kind: "password.very_weak" },
    ],
    totals: { critical: 1, high: 0, healthy: 0, total: 1 },
    reuseGroups: [],
  };

  it("returns highestSeverity 'critical' when any critical finding is present", () => {
    expect(summarize(criticalAndHighReport)).toEqual({
      totalUnhealthy: 2,
      highestSeverity: "critical",
    });
  });

  it("collapses two findings on one entry into one un-healthy entry labelled critical", () => {
    expect(summarize(sameEntryBothFindings)).toEqual({
      totalUnhealthy: 1,
      highestSeverity: "critical",
    });
  });
});

describe("findingsForEntry", () => {
  it("returns only findings scoped to the given entry id", () => {
    expect(findingsForEntry(allHighReport, "b")).toEqual([
      { entryId: "b", kind: "password.expired" },
    ]);
  });

  it("returns an empty array when the report is null or empty", () => {
    expect(findingsForEntry(null, "a")).toEqual([]);
    expect(findingsForEntry(emptyReport, "a")).toEqual([]);
  });
});
