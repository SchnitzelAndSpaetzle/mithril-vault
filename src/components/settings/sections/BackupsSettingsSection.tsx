// SPDX-License-Identifier: MIT

import { useTranslation } from "react-i18next";
import { Checkbox } from "@/components/ui/checkbox";
import { SettingsSection } from "@/components/settings/SettingsSection";
import type { AppPreferences } from "@/lib/types";

interface BackupsSettingsSectionProps {
  draft: AppPreferences;
  updateDraft: (updater: (previous: AppPreferences) => AppPreferences) => void;
}

export function BackupsSettingsSection({
  draft,
  updateDraft,
}: Readonly<BackupsSettingsSectionProps>) {
  const { t } = useTranslation();

  return (
    <SettingsSection
      id="backups"
      title={t("settings.backups.title")}
      description={t("settings.backups.description")}
    >
      <label className="flex items-start gap-2 text-sm">
        <Checkbox
          aria-label={t("settings.backups.enabled.label")}
          checked={draft.backups.enabled}
          onCheckedChange={(checked) =>
            updateDraft((previous) => ({
              ...previous,
              backups: {
                ...previous.backups,
                enabled: checked === true,
              },
            }))
          }
        />
        <span className="flex flex-col gap-1">
          <span>{t("settings.backups.enabled.label")}</span>
          <span className="text-muted-foreground">
            {t("settings.backups.enabled.description")}
          </span>
        </span>
      </label>
    </SettingsSection>
  );
}
