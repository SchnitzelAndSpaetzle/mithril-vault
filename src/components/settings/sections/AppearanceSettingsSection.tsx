// SPDX-License-Identifier: MIT

import { useTranslation } from "react-i18next";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { SettingsSection } from "@/components/settings/SettingsSection";
import {
  FONT_SIZE_MAX,
  FONT_SIZE_MIN,
  isThemePreference,
  THEME_OPTIONS,
} from "@/components/settings/settings-utils";
import type { AppPreferences } from "@/lib/types";

interface AppearanceSettingsSectionProps {
  draft: AppPreferences;
  updateDraft: (updater: (previous: AppPreferences) => AppPreferences) => void;
  onThemePreview: (theme: AppPreferences["appearance"]["theme"]) => void;
}

export function AppearanceSettingsSection({
  draft,
  updateDraft,
  onThemePreview,
}: Readonly<AppearanceSettingsSectionProps>) {
  const { t } = useTranslation();

  return (
    <SettingsSection
      id="appearance"
      title={t("settings.appearance.title")}
      description={t("settings.appearance.description")}
    >
      <div className="grid gap-4 md:grid-cols-2">
        <div className="space-y-2">
          <Label htmlFor="theme">{t("settings.appearance.theme")}</Label>
          <select
            id="theme"
            className="w-full h-9 rounded-md border border-input bg-transparent px-3 text-sm"
            value={draft.appearance.theme}
            onChange={(event) => {
              const nextTheme = event.target.value;
              if (!isThemePreference(nextTheme)) {
                return;
              }

              updateDraft((previous) => ({
                ...previous,
                appearance: {
                  ...previous.appearance,
                  theme: nextTheme,
                },
              }));
              onThemePreview(nextTheme);
            }}
          >
            {THEME_OPTIONS.map((option) => (
              <option key={option} value={option}>
                {option}
              </option>
            ))}
          </select>
        </div>
        <div className="space-y-2">
          <Label htmlFor="font-size">{t("settings.appearance.fontSize")}</Label>
          <Input
            id="font-size"
            type="number"
            min={FONT_SIZE_MIN}
            max={FONT_SIZE_MAX}
            value={draft.appearance.fontSize}
            onChange={(event) =>
              updateDraft((previous) => ({
                ...previous,
                appearance: {
                  ...previous.appearance,
                  fontSize: Math.min(
                    FONT_SIZE_MAX,
                    Math.max(
                      FONT_SIZE_MIN,
                      Number(event.target.value) || FONT_SIZE_MIN
                    )
                  ),
                },
              }))
            }
          />
        </div>
      </div>

      <div className="space-y-2">
        <Label>{t("settings.appearance.entryListColumns")}</Label>
        <div className="grid gap-2 md:grid-cols-2">
          <label className="flex items-center gap-2 text-sm">
            <Checkbox
              checked={draft.appearance.entryListColumns.username}
              onCheckedChange={(checked) =>
                updateDraft((previous) => ({
                  ...previous,
                  appearance: {
                    ...previous.appearance,
                    entryListColumns: {
                      ...previous.appearance.entryListColumns,
                      username: checked === true,
                    },
                  },
                }))
              }
            />
            {t("settings.appearance.columns.username")}
          </label>
          <label className="flex items-center gap-2 text-sm">
            <Checkbox
              checked={draft.appearance.entryListColumns.url}
              onCheckedChange={(checked) =>
                updateDraft((previous) => ({
                  ...previous,
                  appearance: {
                    ...previous.appearance,
                    entryListColumns: {
                      ...previous.appearance.entryListColumns,
                      url: checked === true,
                    },
                  },
                }))
              }
            />
            {t("settings.appearance.columns.url")}
          </label>
          <label className="flex items-center gap-2 text-sm">
            <Checkbox
              checked={draft.appearance.entryListColumns.modifiedAt}
              onCheckedChange={(checked) =>
                updateDraft((previous) => ({
                  ...previous,
                  appearance: {
                    ...previous.appearance,
                    entryListColumns: {
                      ...previous.appearance.entryListColumns,
                      modifiedAt: checked === true,
                    },
                  },
                }))
              }
            />
            {t("settings.appearance.columns.modifiedAt")}
          </label>
          <label className="flex items-center gap-2 text-sm">
            <Checkbox
              checked={draft.appearance.entryListColumns.tags}
              onCheckedChange={(checked) =>
                updateDraft((previous) => ({
                  ...previous,
                  appearance: {
                    ...previous.appearance,
                    entryListColumns: {
                      ...previous.appearance.entryListColumns,
                      tags: checked === true,
                    },
                  },
                }))
              }
            />
            {t("settings.appearance.columns.tags")}
          </label>
        </div>
        <p className="text-xs text-muted-foreground">
          {t("settings.appearance.columnsNote")}
        </p>
      </div>
    </SettingsSection>
  );
}
