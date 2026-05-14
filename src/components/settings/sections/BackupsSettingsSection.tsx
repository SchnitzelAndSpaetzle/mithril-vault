// SPDX-License-Identifier: MIT

import { useTranslation } from "react-i18next";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { SettingsSection } from "@/components/settings/SettingsSection";
import {
  BACKUP_MAX_VERSIONS_PRESETS,
  DEFAULT_BACKUP_MAX_VERSIONS,
  type AppPreferences,
} from "@/lib/types";

interface BackupsSettingsSectionProps {
  draft: AppPreferences;
  updateDraft: (updater: (previous: AppPreferences) => AppPreferences) => void;
}

export function BackupsSettingsSection({
  draft,
  updateDraft,
}: Readonly<BackupsSettingsSectionProps>) {
  const { t } = useTranslation();

  // Use the documented presets as the canonical choice set. If a foreign
  // settings.json file has been migrated with an off-preset value we still
  // surface it so the dropdown doesn't silently rewrite the user's setting.
  const currentValue = draft.backups.maxVersions ?? DEFAULT_BACKUP_MAX_VERSIONS;
  const optionValues = BACKUP_MAX_VERSIONS_PRESETS.includes(
    currentValue as (typeof BACKUP_MAX_VERSIONS_PRESETS)[number]
  )
    ? BACKUP_MAX_VERSIONS_PRESETS
    : ([...BACKUP_MAX_VERSIONS_PRESETS, currentValue].sort(
        (a, b) => a - b
      ) as readonly number[]);

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

      <div className="flex flex-col gap-2 text-sm">
        <span>{t("settings.backups.maxVersions.label")}</span>
        <Select
          value={String(currentValue)}
          onValueChange={(next) => {
            const parsed = Number.parseInt(next, 10);
            if (Number.isNaN(parsed)) return;
            updateDraft((previous) => ({
              ...previous,
              backups: {
                ...previous.backups,
                maxVersions: parsed,
              },
            }));
          }}
        >
          <SelectTrigger
            aria-label={t("settings.backups.maxVersions.label")}
            className="w-40"
          >
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {optionValues.map((preset) => (
              <SelectItem key={preset} value={String(preset)}>
                {String(preset)}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <span className="text-muted-foreground">
          {t("settings.backups.maxVersions.description")}
        </span>
      </div>
    </SettingsSection>
  );
}
