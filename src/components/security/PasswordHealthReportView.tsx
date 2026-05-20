// SPDX-License-Identifier: MIT

import { OctagonAlert, TriangleAlert } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "@tanstack/react-router";

import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { useActiveDatabase } from "@/hooks/use-active-database";
import { useEntries } from "@/hooks/use-entries";
import { usePasswordHealthReport } from "@/hooks/use-password-health";
import {
  reuseGroupsForSection,
  severityOf,
  type Severity,
} from "@/lib/password-health";
import { useDatabaseTabs } from "@/stores/database-tabs";
import type { Entry, Finding, PasswordHealthReport } from "@/lib/types";
import { ReusedGroupRow } from "./ReusedGroupRow";

interface PasswordHealthReportViewProps {
  dbId: string;
}

export function PasswordHealthReportView({
  dbId,
}: Readonly<PasswordHealthReportViewProps>) {
  const { t } = useTranslation();
  const { data: report, isLoading, error } = usePasswordHealthReport(dbId);
  const { data: entries } = useEntries(dbId, null);

  if (error) {
    return (
      <div className="p-6 text-sm text-destructive">
        {error instanceof Error ? error.message : String(error)}
      </div>
    );
  }

  if (isLoading || !report) {
    return (
      <div className="p-6 text-sm text-muted-foreground">
        {t("passwordHealth.loading")}
      </div>
    );
  }

  return (
    // The dashboard SidebarInset sets `overflow-hidden`, so this view
    // must own its own scroll container. Without `flex-1 min-h-0
    // overflow-auto` the page silently clips on Vaults with many
    // findings — the user can't reach the tail of the list.
    <div className="flex flex-1 min-h-0 flex-col gap-6 overflow-auto p-6">
      <header className="flex flex-col gap-1">
        <h1 className="text-2xl font-semibold tracking-tight">
          {t("passwordHealth.title")}
        </h1>
        <p className="text-sm text-muted-foreground">
          {t("passwordHealth.subtitle")}
        </p>
      </header>

      <ScoreAndTotals report={report} />

      <FindingsSection
        severity="critical"
        report={report}
        entries={entries ?? []}
      />
      <FindingsSection
        severity="high"
        report={report}
        entries={entries ?? []}
      />
    </div>
  );
}

function ScoreAndTotals({
  report,
}: Readonly<{ report: PasswordHealthReport }>) {
  const { t } = useTranslation();
  const isEmpty = report.totals.total === 0;
  const scoreText =
    report.score === null ? t("passwordHealth.noScore") : String(report.score);

  return (
    <div className="grid gap-4 sm:grid-cols-5">
      <Card className="sm:col-span-2">
        <CardHeader className="pb-2">
          <CardDescription>{t("passwordHealth.scoreLabel")}</CardDescription>
          <CardTitle className="text-4xl tabular-nums">{scoreText}</CardTitle>
        </CardHeader>
        {isEmpty && (
          <CardContent className="pt-0 text-sm text-muted-foreground">
            {t("passwordHealth.emptyState")}
          </CardContent>
        )}
      </Card>

      <TotalsCell
        label={t("passwordHealth.totals.critical")}
        value={report.totals.critical}
      />
      <TotalsCell
        label={t("passwordHealth.totals.high")}
        value={report.totals.high}
        {...(report.totals.high > 0 ? { emphasis: "amber" as const } : {})}
      />
      <TotalsCell
        label={t("passwordHealth.totals.healthy")}
        value={report.totals.healthy}
      />
    </div>
  );
}

function TotalsCell({
  label,
  value,
  emphasis,
}: Readonly<{
  label: string;
  value: number;
  emphasis?: "amber";
}>) {
  return (
    <Card>
      <CardHeader className="pb-2">
        <CardDescription>{label}</CardDescription>
        <CardTitle
          className={`text-3xl tabular-nums ${
            emphasis === "amber" ? "text-amber-600 dark:text-amber-500" : ""
          }`}
        >
          {value}
        </CardTitle>
      </CardHeader>
    </Card>
  );
}

// Bucket each Entry by its highest-severity Finding so an Entry that
// is both Very Weak and Expired surfaces only in the Critical section,
// not both. Mirrors the totals-strip rule in the backend analyzer.
//
// `Reused` Findings are not surfaced as per-Entry rows — they live
// in the dedicated `ReusedGroupRow` (one row per shared password,
// inline-expandable) so the High section doesn't duplicate the group
// membership. An Entry that is Reused-only therefore has no
// per-Entry row at all; an Entry that is Reused + Very Weak still
// appears in the Critical section because Very Weak wins on
// severity and carries its own remediation message.
function bucketEntriesBySeverity(
  findings: Finding[]
): Map<Severity, Map<string, Finding[]>> {
  const findingsExcludingReused = findings.filter(
    (f) => f.kind !== "password.reused"
  );
  const highestPerEntry = new Map<string, Severity>();
  for (const finding of findingsExcludingReused) {
    const severity = severityOf(finding.kind);
    const prior = highestPerEntry.get(finding.entryId);
    if (prior === "critical") continue;
    if (severity === "critical" || prior === undefined) {
      highestPerEntry.set(finding.entryId, severity);
    }
  }

  const buckets = new Map<Severity, Map<string, Finding[]>>([
    ["critical", new Map()],
    ["high", new Map()],
  ]);
  for (const finding of findingsExcludingReused) {
    const bucket = highestPerEntry.get(finding.entryId);
    if (!bucket) continue;
    const list = buckets.get(bucket)?.get(finding.entryId) ?? [];
    list.push(finding);
    buckets.get(bucket)?.set(finding.entryId, list);
  }
  return buckets;
}

interface SectionChrome {
  Icon: typeof TriangleAlert;
  iconClass: string;
  title: string;
  description: string;
}

function FindingsSection({
  severity,
  report,
  entries,
}: Readonly<{
  severity: Severity;
  report: PasswordHealthReport;
  entries: Entry[];
}>) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { tab } = useActiveDatabase();
  const updateTabState = useDatabaseTabs((s) => s.updateTabState);

  const byEntry = bucketEntriesBySeverity(report.findings).get(severity);
  // Reused groups default to High (per ADR 0002), but a group whose
  // every member is also Very Weak is promoted to Critical — those
  // entries are bucketed Critical, so leaving the group in High
  // would mean "High 0" plus a reused-password row. The helper
  // partitions the report's `reuseGroups` between the two sections
  // so every emitted group renders exactly once.
  const reuseGroups = reuseGroupsForSection(report, severity);
  const perEntryCount = byEntry?.size ?? 0;
  if (perEntryCount === 0 && reuseGroups.length === 0) {
    return null;
  }

  const entryById = new Map(entries.map((e) => [e.id, e]));

  // The entry detail view's Edit action reads `tab.selectedEntryId`,
  // not the URL param — so we have to mirror the EntryList click
  // pattern (update tab state, then navigate) instead of navigating
  // directly. The route's `beforeLoad` already activated this tab via
  // `requireUnlockedTab`, so `useActiveDatabase` resolves to it.
  const openEntry = (entryId: string) => {
    if (tab) {
      updateTabState(tab.id, { selectedEntryId: entryId });
    }
    void navigate({ to: "/dashboard/entry/$id", params: { id: entryId } });
  };

  const chrome: SectionChrome =
    severity === "critical"
      ? {
          Icon: OctagonAlert,
          iconClass: "text-red-600 dark:text-red-500",
          title: t("passwordHealth.sections.critical"),
          description: t("passwordHealth.sections.criticalDescription"),
        }
      : {
          Icon: TriangleAlert,
          iconClass: "text-amber-600 dark:text-amber-500",
          title: t("passwordHealth.sections.high"),
          description: t("passwordHealth.sections.highDescription"),
        };

  return (
    <section className="flex flex-col gap-2">
      <div className="flex items-center gap-2">
        <chrome.Icon className={`size-4 ${chrome.iconClass}`} />
        <h2 className="text-lg font-medium">{chrome.title}</h2>
      </div>
      <p className="text-sm text-muted-foreground">{chrome.description}</p>
      <ul className="divide-y rounded-md border bg-card">
        {reuseGroups.map((group, index) => (
          <ReusedGroupRow
            key={`reuse-${index}-${group.entryIds[0]}`}
            entryIds={group.entryIds}
            entries={entries}
            onOpenEntry={openEntry}
          />
        ))}
        {byEntry &&
          Array.from(byEntry.entries()).map(([entryId, findings]) => {
            const entry = entryById.get(entryId);
            return (
              <li
                key={entryId}
                className="flex items-center justify-between gap-3 px-4 py-3"
              >
                <div className="flex flex-col gap-0.5 min-w-0">
                  <span className="truncate font-medium">
                    {entry?.title ?? entryId}
                  </span>
                  <span className="text-xs text-muted-foreground">
                    {findings
                      .map((f) => t(`passwordHealth.findings.${f.kind}`))
                      .join(" · ")}
                  </span>
                </div>
                <Button
                  size="sm"
                  variant="outline"
                  onClick={() => openEntry(entryId)}
                >
                  {t("passwordHealth.actions.openEntry")}
                </Button>
              </li>
            );
          })}
      </ul>
    </section>
  );
}
