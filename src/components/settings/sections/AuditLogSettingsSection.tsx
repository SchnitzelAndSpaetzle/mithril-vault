// SPDX-License-Identifier: MIT

import { useTranslation } from "react-i18next";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { SettingsSection } from "@/components/settings/SettingsSection";
import type { AppPreferences } from "@/lib/types";

interface AuditLogSettingsSectionProps {
  draft: AppPreferences;
  updateDraft: (updater: (previous: AppPreferences) => AppPreferences) => void;
}

const RETENTION_MIN = 1;
const RETENTION_MAX = 365;

export function AuditLogSettingsSection({
  draft,
  updateDraft,
}: Readonly<AuditLogSettingsSectionProps>) {
  const { t } = useTranslation();

  return (
    <SettingsSection
      id="audit-settings"
      title={t("settings.audit.title")}
      description={t("settings.audit.description")}
    >
      <label className="flex items-start gap-2 text-sm">
        <Checkbox
          aria-label={t("settings.audit.enabled.label")}
          checked={draft.audit.enabled}
          onCheckedChange={(checked) =>
            updateDraft((previous) => ({
              ...previous,
              audit: {
                ...previous.audit,
                enabled: checked === true,
              },
            }))
          }
        />
        <span className="flex flex-col gap-1">
          <span>{t("settings.audit.enabled.label")}</span>
          <span className="text-muted-foreground">
            {t("settings.audit.enabled.description")}
          </span>
        </span>
      </label>

      <div className="flex flex-col gap-2 text-sm">
        <label htmlFor="audit-retention-days" className="flex flex-col gap-1">
          <span>{t("settings.audit.retentionDays.label")}</span>
          <span className="text-muted-foreground">
            {t("settings.audit.retentionDays.description")}
          </span>
        </label>
        <Input
          id="audit-retention-days"
          aria-label={t("settings.audit.retentionDays.label")}
          type="number"
          inputMode="numeric"
          min={RETENTION_MIN}
          max={RETENTION_MAX}
          className="w-32"
          value={draft.audit.retentionDays}
          onChange={(event) => {
            const raw = Number.parseInt(event.target.value, 10);
            if (Number.isNaN(raw)) return;
            updateDraft((previous) => ({
              ...previous,
              audit: {
                ...previous.audit,
                retentionDays: raw,
              },
            }));
          }}
        />
      </div>
    </SettingsSection>
  );
}
