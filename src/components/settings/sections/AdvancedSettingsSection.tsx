// SPDX-License-Identifier: MIT

import { useTranslation } from "react-i18next";
import { Checkbox } from "@/components/ui/checkbox";
import { Label } from "@/components/ui/label";
import { SettingsSection } from "@/components/settings/SettingsSection";
import type { AppPreferences } from "@/lib/types";

interface AdvancedSettingsSectionProps {
  draft: AppPreferences;
  updateDraft: (updater: (previous: AppPreferences) => AppPreferences) => void;
}

export function AdvancedSettingsSection({
  draft,
  updateDraft,
}: Readonly<AdvancedSettingsSectionProps>) {
  const { t } = useTranslation();

  return (
    <SettingsSection
      id="advanced"
      title={t("settings.advanced.title")}
      description={t("settings.advanced.description")}
    >
      <label className="flex items-center gap-2 text-sm">
        <Checkbox
          checked={draft.advanced.debugMode}
          onCheckedChange={(checked) =>
            updateDraft((previous) => ({
              ...previous,
              advanced: {
                ...previous.advanced,
                debugMode: checked === true,
              },
            }))
          }
        />
        {t("settings.advanced.enableDebugMode")}
      </label>
      <div className="space-y-1">
        <Label>{t("settings.advanced.dataLocation")}</Label>
        <p className="text-sm text-muted-foreground break-all">
          {draft.advanced.dataLocation}
        </p>
      </div>
    </SettingsSection>
  );
}
