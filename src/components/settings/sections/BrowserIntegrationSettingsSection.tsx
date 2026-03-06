// SPDX-License-Identifier: MIT

import { useTranslation } from "react-i18next";
import { Checkbox } from "@/components/ui/checkbox";
import { Label } from "@/components/ui/label";
import { SettingsSection } from "@/components/settings/SettingsSection";
import type { AppPreferences } from "@/lib/types";

interface BrowserIntegrationSettingsSectionProps {
  draft: AppPreferences;
  allowedSitesInput: string;
  setAllowedSitesInput: (value: string) => void;
  updateDraft: (updater: (previous: AppPreferences) => AppPreferences) => void;
}

export function BrowserIntegrationSettingsSection({
  draft,
  allowedSitesInput,
  setAllowedSitesInput,
  updateDraft,
}: Readonly<BrowserIntegrationSettingsSectionProps>) {
  const { t } = useTranslation();

  return (
    <SettingsSection
      id="browser"
      title={t("settings.browser.title")}
      description={t("settings.browser.description")}
    >
      <label className="flex items-center gap-2 text-sm">
        <Checkbox
          checked={draft.browserIntegration.enabled}
          onCheckedChange={(checked) =>
            updateDraft((previous) => ({
              ...previous,
              browserIntegration: {
                ...previous.browserIntegration,
                enabled: checked === true,
              },
            }))
          }
        />
        {t("settings.browser.enableIntegration")}
      </label>
      <div className="space-y-2">
        <Label htmlFor="allowed-sites">
          {t("settings.browser.allowedSites")}
        </Label>
        <textarea
          id="allowed-sites"
          className="min-h-24 w-full rounded-md border border-input bg-transparent px-3 py-2 text-sm"
          value={allowedSitesInput}
          onChange={(event) => setAllowedSitesInput(event.target.value)}
        />
        <p className="text-xs text-muted-foreground">
          {t("settings.browser.allowedSitesNote")}
        </p>
      </div>
    </SettingsSection>
  );
}
