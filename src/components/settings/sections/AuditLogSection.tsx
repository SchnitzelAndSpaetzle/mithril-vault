// SPDX-License-Identifier: MIT

import { useTranslation } from "react-i18next";
import { useQuery } from "@tanstack/react-query";
import { SettingsSection } from "@/components/settings/SettingsSection";
import { queryKeys } from "@/lib/query-keys";
import { audit } from "@/lib/tauri";
import type { AuditEvent } from "@/lib/types";

interface AuditLogSectionProps {
  dbId: string | null;
}

function formatTimestamp(timestamp: string, locale: string): string {
  const parsed = new Date(timestamp);
  if (Number.isNaN(parsed.getTime())) return timestamp;
  return parsed.toLocaleString(locale, {
    dateStyle: "medium",
    timeStyle: "medium",
  });
}

function AuditRow({ event }: Readonly<{ event: AuditEvent }>) {
  const { t, i18n } = useTranslation();
  // Today only `vaultUnlockFailed` exists; rendering is keyed on `kind` so
  // future kinds can plug in without rearranging the row layout.
  return (
    <li
      data-kind={event.kind}
      className="flex flex-col gap-1 rounded-md border bg-card/50 p-3 text-sm md:flex-row md:items-center md:justify-between"
    >
      <span className="font-medium">{t(`audit.kind.${event.kind}`)}</span>
      <div className="flex items-center gap-3 text-xs text-muted-foreground">
        {typeof event.attemptCount === "number" ? (
          <span>{t("audit.attemptCount", { count: event.attemptCount })}</span>
        ) : null}
        <span>{formatTimestamp(event.timestamp, i18n.language)}</span>
      </div>
    </li>
  );
}

export function AuditLogSection({ dbId }: Readonly<AuditLogSectionProps>) {
  const { t } = useTranslation();

  const query = useQuery<AuditEvent[], Error>({
    queryKey: queryKeys.audit.list(dbId ?? "none"),
    queryFn: () => (dbId ? audit.list(dbId) : Promise.resolve([])),
    enabled: Boolean(dbId),
  });

  return (
    <SettingsSection
      id="audit-log"
      title={t("audit.title")}
      description={t("audit.description")}
    >
      {!dbId ? (
        <p className="text-sm text-muted-foreground">
          {t("audit.emptyNoVault")}
        </p>
      ) : query.isLoading ? (
        <p className="text-sm text-muted-foreground">{t("common.loading")}</p>
      ) : query.error ? (
        <p className="text-sm text-destructive">
          {t("audit.loadError", { error: String(query.error) })}
        </p>
      ) : (query.data ?? []).length === 0 ? (
        <p className="text-sm text-muted-foreground">{t("audit.empty")}</p>
      ) : (
        <ul className="flex flex-col gap-2">
          {(query.data ?? []).map((event, index) => (
            <AuditRow key={`${event.timestamp}-${index}`} event={event} />
          ))}
        </ul>
      )}
    </SettingsSection>
  );
}
