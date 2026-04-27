// SPDX-License-Identifier: MIT

import { useTranslation } from "react-i18next";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Separator } from "@/components/ui/separator";
import { SettingsSection } from "@/components/settings/SettingsSection";
import type { AppPreferences } from "@/lib/types";

interface SecuritySettingsSectionProps {
  draft: AppPreferences;
  updateDraft: (updater: (previous: AppPreferences) => AppPreferences) => void;
}

export function SecuritySettingsSection({
  draft,
  updateDraft,
}: Readonly<SecuritySettingsSectionProps>) {
  const { t } = useTranslation();

  return (
    <SettingsSection
      id="security"
      title={t("settings.security.title")}
      description={t("settings.security.description")}
    >
      <div className="grid gap-4 md:grid-cols-2">
        <div className="space-y-2">
          <Label htmlFor="auto-lock-timeout">
            {t("settings.security.autoLockTimeout")}
          </Label>
          <Input
            id="auto-lock-timeout"
            type="number"
            min={0}
            value={draft.security.autoLockTimeout}
            onChange={(event) => {
              const value = Number(event.target.value);
              updateDraft((previous) => ({
                ...previous,
                security: {
                  ...previous.security,
                  autoLockTimeout: value === 0 ? 0 : Math.max(30, value || 30),
                },
              }));
            }}
          />
          <p className="text-xs text-muted-foreground">
            {t("settings.security.autoLockNote")}
          </p>
        </div>
        <div className="space-y-2">
          <Label htmlFor="clipboard-timeout">
            {t("settings.security.clipboardTimeout")}
          </Label>
          <Input
            id="clipboard-timeout"
            type="number"
            min={1}
            value={draft.security.clipboardClearTimeout}
            onChange={(event) =>
              updateDraft((previous) => ({
                ...previous,
                security: {
                  ...previous.security,
                  clipboardClearTimeout: Math.max(
                    1,
                    Number(event.target.value) || 1
                  ),
                },
              }))
            }
          />
        </div>
      </div>

      <Separator />

      <div className="grid gap-4 md:grid-cols-3">
        <label className="flex items-center gap-2 text-sm">
          <Checkbox
            checked={draft.security.clearClipboardOnLock}
            onCheckedChange={(checked) =>
              updateDraft((previous) => ({
                ...previous,
                security: {
                  ...previous.security,
                  clearClipboardOnLock: checked === true,
                },
              }))
            }
          />
          {t("settings.security.clearClipboardOnLock")}
        </label>
        <label className="flex items-center gap-2 text-sm">
          <Checkbox
            checked={draft.security.showClipboardCountdown}
            onCheckedChange={(checked) =>
              updateDraft((previous) => ({
                ...previous,
                security: {
                  ...previous.security,
                  showClipboardCountdown: checked === true,
                },
              }))
            }
          />
          {t("settings.security.showClipboardCountdown")}
        </label>
        <label className="flex items-center gap-2 text-sm">
          <Checkbox
            checked={draft.security.showPasswordByDefault}
            onCheckedChange={(checked) =>
              updateDraft((previous) => ({
                ...previous,
                security: {
                  ...previous.security,
                  showPasswordByDefault: checked === true,
                },
              }))
            }
          />
          {t("settings.security.showPasswordByDefault")}
        </label>
        <label className="flex items-center gap-2 text-sm">
          <Checkbox
            checked={draft.security.minimizeToTray}
            onCheckedChange={(checked) =>
              updateDraft((previous) => ({
                ...previous,
                security: {
                  ...previous.security,
                  minimizeToTray: checked === true,
                },
              }))
            }
          />
          {t("settings.security.minimizeToTray")}
        </label>
        <label className="flex items-center gap-2 text-sm">
          <Checkbox
            checked={draft.security.startMinimized}
            onCheckedChange={(checked) =>
              updateDraft((previous) => ({
                ...previous,
                security: {
                  ...previous.security,
                  startMinimized: checked === true,
                },
              }))
            }
          />
          {t("settings.security.startMinimized")}
        </label>
      </div>
    </SettingsSection>
  );
}
