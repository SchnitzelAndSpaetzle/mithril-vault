// SPDX-License-Identifier: MIT

import { TriangleAlert } from "lucide-react";
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
import { useEntries } from "@/hooks/use-entries";
import { usePasswordHealthReport } from "@/hooks/use-password-health";
import type { Entry, Finding, PasswordHealthReport } from "@/lib/types";

interface PasswordHealthReportViewProps {
  dbId: string;
}

export function PasswordHealthReportView({
  dbId,
}: PasswordHealthReportViewProps) {
  const { t } = useTranslation();
  const { data: report, isLoading } = usePasswordHealthReport(dbId);
  const { data: entries } = useEntries(dbId, null);

  if (isLoading || !report) {
    return (
      <div className="p-6 text-sm text-muted-foreground">
        {t("passwordHealth.loading")}
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-6 p-6">
      <header className="flex flex-col gap-1">
        <h1 className="text-2xl font-semibold tracking-tight">
          {t("passwordHealth.title")}
        </h1>
        <p className="text-sm text-muted-foreground">
          {t("passwordHealth.subtitle")}
        </p>
      </header>

      <ScoreAndTotals report={report} />

      <HighFindingsSection report={report} entries={entries ?? []} />
    </div>
  );
}

function ScoreAndTotals({ report }: { report: PasswordHealthReport }) {
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
}: {
  label: string;
  value: number;
  emphasis?: "amber";
}) {
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

function HighFindingsSection({
  report,
  entries,
}: {
  report: PasswordHealthReport;
  entries: Entry[];
}) {
  const { t } = useTranslation();
  const navigate = useNavigate();

  // Group findings by entry so the row collapses an Entry that hits
  // multiple Findings of the same severity into a single visual row.
  const byEntry = new Map<string, Finding[]>();
  for (const finding of report.findings) {
    const list = byEntry.get(finding.entryId) ?? [];
    list.push(finding);
    byEntry.set(finding.entryId, list);
  }

  if (byEntry.size === 0) {
    return null;
  }

  const entryById = new Map(entries.map((e) => [e.id, e]));

  return (
    <section className="flex flex-col gap-2">
      <div className="flex items-center gap-2">
        <TriangleAlert className="size-4 text-amber-600 dark:text-amber-500" />
        <h2 className="text-lg font-medium">
          {t("passwordHealth.sections.high")}
        </h2>
      </div>
      <p className="text-sm text-muted-foreground">
        {t("passwordHealth.sections.highDescription")}
      </p>
      <ul className="divide-y rounded-md border bg-card">
        {Array.from(byEntry.entries()).map(([entryId, findings]) => {
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
                onClick={() =>
                  void navigate({
                    to: "/dashboard/entry/$id",
                    params: { id: entryId },
                  })
                }
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
