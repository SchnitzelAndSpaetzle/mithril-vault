// SPDX-License-Identifier: MIT

import { useTranslation } from "react-i18next";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { SettingsSection } from "@/components/settings/SettingsSection";
import {
  LOCALE_LABELS,
  SUPPORTED_LOCALES,
  type SupportedLocale,
} from "@/lib/i18n-constants";
import type { AppPreferences } from "@/lib/types";

interface GeneralSettingsSectionProps {
  draft: AppPreferences;
  updateDraft: (updater: (previous: AppPreferences) => AppPreferences) => void;
  onStartupBehaviorChange: (value: string) => void;
}

export function GeneralSettingsSection({
  draft,
  updateDraft,
  onStartupBehaviorChange,
}: Readonly<GeneralSettingsSectionProps>) {
  const { t } = useTranslation();

  return (
    <SettingsSection
      id="general"
      title={t("settings.general.title")}
      description={t("settings.general.description")}
    >
      <div className="grid gap-4 md:grid-cols-2">
        <div className="space-y-2">
          <Label htmlFor="language">{t("settings.general.language")}</Label>
          <select
            id="language"
            className="w-full h-9 rounded-md border border-input bg-transparent px-3 text-sm"
            value={draft.general.language}
            onChange={(event) =>
              updateDraft((previous) => ({
                ...previous,
                general: {
                  ...previous.general,
                  language: event.target.value,
                },
              }))
            }
          >
            {SUPPORTED_LOCALES.map((locale) => (
              <option key={locale} value={locale}>
                {LOCALE_LABELS[locale as SupportedLocale]}
              </option>
            ))}
          </select>
        </div>
        <div className="space-y-2">
          <Label htmlFor="startup-behavior">
            {t("settings.general.startupBehavior")}
          </Label>
          <select
            id="startup-behavior"
            className="w-full h-9 rounded-md border border-input bg-transparent px-3 text-sm"
            value={draft.general.startupBehavior}
            onChange={(event) => onStartupBehaviorChange(event.target.value)}
          >
            <option value="showUnlockScreen">
              {t("settings.general.startupOptions.showUnlockScreen")}
            </option>
            <option value="openLastDatabase">
              {t("settings.general.startupOptions.openLastDatabase")}
            </option>
            <option value="openDefaultDatabase">
              {t("settings.general.startupOptions.openDefaultDatabase")}
            </option>
          </select>
        </div>
      </div>
      <div className="space-y-2">
        <Label htmlFor="default-database-path">
          {t("settings.general.defaultDatabasePath")}
        </Label>
        <Input
          id="default-database-path"
          placeholder={t("settings.general.defaultDatabasePathPlaceholder")}
          value={draft.general.defaultDatabasePath ?? ""}
          onChange={(event) =>
            updateDraft((previous) => ({
              ...previous,
              general: {
                ...previous.general,
                defaultDatabasePath:
                  event.target.value.trim().length > 0
                    ? event.target.value
                    : null,
              },
            }))
          }
        />
        <p className="text-xs text-muted-foreground">
          {t("settings.general.defaultDatabasePathNote")}
        </p>
      </div>
    </SettingsSection>
  );
}
