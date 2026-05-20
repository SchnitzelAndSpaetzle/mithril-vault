// SPDX-License-Identifier: MIT

//! Frontend-side selectors and helpers for the Password Health report.
//!
//! The IPC wrapper itself lives in `lib/tauri.ts`; this module owns the
//! pure derivations (highest-severity per Entry, sidebar summary) that
//! the route view and sidebar badge read.

import type {
  Finding,
  FindingKind,
  PasswordHealthReport,
  ReuseGroup,
} from "./types";

/// Severity bucket a Finding Kind belongs to. Mirrors the backend's
/// `analyzer::Severity`: `very_weak` is Critical, every other Finding
/// Kind (`weak`, `reused`, `expired`) is High. Healthy Entries have no
/// Findings and therefore no severity.
export type Severity = "critical" | "high";

export function severityOf(kind: FindingKind): Severity {
  if (kind === "password.very_weak") return "critical";
  if (
    kind === "password.weak" ||
    kind === "password.reused" ||
    kind === "password.expired"
  )
    return "high";
  // Exhaustiveness lives at the union boundary — adding a new Finding
  // Kind in `types.ts` makes this `never` cast fail to compile.
  const exhaustive: never = kind;
  return exhaustive;
}

/// Summary the sidebar Security item renders next to its label. The
/// badge shows the un-healthy Entry count (not the Finding count — an
/// Entry with two Findings counts once) and the colour is picked from
/// `highestSeverity`. `highestSeverity` is `null` when there are no
/// Findings; the sidebar hides the badge entirely in that case.
export interface PasswordHealthSummary {
  totalUnhealthy: number;
  highestSeverity: Severity | null;
}

export function summarize(
  report: PasswordHealthReport | null | undefined
): PasswordHealthSummary {
  if (!report) {
    return { totalUnhealthy: 0, highestSeverity: null };
  }

  // Bucket each Entry by its highest-severity Finding so the badge
  // count agrees with the report-view totals strip (which also
  // counts distinct Entries, not Findings).
  const seen = new Map<string, Severity>();
  for (const finding of report.findings) {
    const severity = severityOf(finding.kind);
    const prior = seen.get(finding.entryId);
    if (prior === "critical") continue;
    if (severity === "critical" || prior === undefined) {
      seen.set(finding.entryId, severity);
    }
  }

  let highest: Severity | null = null;
  for (const severity of seen.values()) {
    if (severity === "critical") {
      highest = "critical";
      break;
    }
    highest = "high";
  }

  return {
    totalUnhealthy: seen.size,
    highestSeverity: highest,
  };
}

/// Reuse groups that should render in the High section of the report
/// view. Drops groups whose every member is already in the Critical
/// bucket (e.g. two Entries sharing a zxcvbn score-0 password) — the
/// backend's `high` total intentionally subtracts those Entries, so
/// rendering a "Reused" row in High when `High: 0` would mislead the
/// reader. The Critical per-Entry rows already surface the reuse
/// signal via their Very-Weak finding.
///
/// A member is in the Critical bucket iff it has at least one
/// non-`password.reused` Finding whose kind maps to `critical` —
/// matches the bucketing rule in `PasswordHealthReportView`.
export function reuseGroupsForHighSection(
  report: PasswordHealthReport | null | undefined
): ReuseGroup[] {
  if (!report) return [];
  const criticalEntryIds = new Set<string>();
  for (const finding of report.findings) {
    if (finding.kind === "password.reused") continue;
    if (severityOf(finding.kind) === "critical") {
      criticalEntryIds.add(finding.entryId);
    }
  }
  return report.reuseGroups.filter((group) =>
    group.entryIds.some((id) => !criticalEntryIds.has(id))
  );
}

/// Returns every Finding scoped to a given Entry id, in the order the
/// report emitted them. The list is filtered, not mapped — callers
/// rely on the raw `Finding` shape for severity-and-kind UI.
export function findingsForEntry(
  report: PasswordHealthReport | null | undefined,
  entryId: string
): Finding[] {
  if (!report) return [];
  return report.findings.filter((f) => f.entryId === entryId);
}
