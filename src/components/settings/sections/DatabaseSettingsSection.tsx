// SPDX-License-Identifier: MIT

import { useCallback } from "react";
import { AlertTriangle, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { ask } from "@tauri-apps/plugin-dialog";
import { SettingsSection } from "@/components/settings/SettingsSection";
import { VaultHistorySettingsControl } from "@/components/settings/sections/VaultHistorySettingsControl";
import { formatKdf } from "@/components/settings/settings-utils";
import { useVaultHistorySettings } from "@/hooks/use-vault-history-settings";
import { Button } from "@/components/ui/button";
import { Separator } from "@/components/ui/separator.tsx";
import type { DatabaseConfig } from "@/lib/types";
import type { JSX } from "react/jsx-runtime";

interface DatabaseSettingsSectionProps {
  dbId: string | null;
  databaseConfig: DatabaseConfig | null;
  isDatabaseConfigLoading: boolean;
  databaseConfigError: Error | null;
}

export function DatabaseSettingsSection({
  dbId,
  databaseConfig,
  isDatabaseConfigLoading,
  databaseConfigError,
}: Readonly<DatabaseSettingsSectionProps>) {
  const { t } = useTranslation();
  const {
    settings: historySettings,
    update: updateHistorySettings,
    isUpdating: isUpdatingHistory,
    clearAll: clearAllHistory,
    isClearing: isClearingHistory,
  } = useVaultHistorySettings(dbId);

  // Empties every Entry's history across the Vault after an explicit
  // confirmation (#327). Destructive and irreversible, so it is gated behind a
  // warning dialog; the hook persists to disk and refreshes any open history.
  const handleClearAll = useCallback(async () => {
    const confirmed = await ask(
      t("settings.database.history.clearAllConfirm"),
      {
        title: t("settings.database.history.clearAllConfirmTitle"),
        kind: "warning",
      }
    );
    if (confirmed) clearAllHistory();
  }, [clearAllHistory, t]);

  let databaseSectionContent: JSX.Element = (
    <p className="text-sm text-muted-foreground">
      {t("settings.database.noConfig")}
    </p>
  );

  if (!dbId) {
    databaseSectionContent = (
      <p className="text-sm text-muted-foreground">
        {t("settings.database.openPrompt")}
      </p>
    );
  } else if (isDatabaseConfigLoading) {
    databaseSectionContent = (
      <p className="text-sm text-muted-foreground">
        {t("settings.database.loadingSettings")}
      </p>
    );
  } else if (databaseConfigError) {
    databaseSectionContent = (
      <div className="rounded-md border border-destructive/40 bg-destructive/5 p-3 text-sm text-destructive">
        {t("settings.database.loadError", {
          error: String(databaseConfigError),
        })}
      </div>
    );
  } else if (databaseConfig) {
    databaseSectionContent = (
      <div className="grid gap-2 text-sm">
        <div className="flex justify-between gap-4">
          <span className="text-muted-foreground">
            {t("settings.database.version")}
          </span>
          <span>{databaseConfig.version}</span>
        </div>
        <div className="flex justify-between gap-4">
          <span className="text-muted-foreground">
            {t("settings.database.outerCipher")}
          </span>
          <span>{databaseConfig.outerCipher}</span>
        </div>
        <div className="flex justify-between gap-4">
          <span className="text-muted-foreground">
            {t("settings.database.innerCipher")}
          </span>
          <span>{databaseConfig.innerCipher}</span>
        </div>
        <div className="flex justify-between gap-4">
          <span className="text-muted-foreground">
            {t("settings.database.compression")}
          </span>
          <span>{databaseConfig.compression}</span>
        </div>
        <div className="flex justify-between gap-4">
          <span className="text-muted-foreground">
            {t("settings.database.kdf")}
          </span>
          <span className="text-right">{formatKdf(databaseConfig.kdf)}</span>
        </div>
      </div>
    );
  }

  return (
    <SettingsSection
      id="database"
      title={t("settings.database.title")}
      description={t("settings.database.description")}
    >
      {databaseSectionContent}

      {dbId && historySettings && (
        <>
          <Separator />
          <VaultHistorySettingsControl
            maxItems={historySettings.maxItems}
            onChange={updateHistorySettings}
            disabled={isUpdatingHistory}
          />
          <div className="flex flex-col gap-1">
            <Button
              variant="outline"
              size="sm"
              className="w-fit gap-2 text-destructive"
              onClick={handleClearAll}
              disabled={isClearingHistory}
            >
              <Trash2 className="size-4" />
              {t("settings.database.history.clearAll")}
            </Button>
            <p className="text-sm text-muted-foreground">
              {t("settings.database.history.clearAllDescription")}
            </p>
          </div>
        </>
      )}

      <div className="rounded-md border border-amber-500/40 bg-amber-500/10 p-3 text-sm text-amber-900 dark:text-amber-200">
        <div className="flex items-center gap-2 font-medium">
          <AlertTriangle className="size-4" />
          {t("settings.database.pendingTitle")}
        </div>
        <ul className="mt-2 list-disc pl-5 space-y-1">
          <li>{t("settings.database.pendingItems.mutations")}</li>
          <li>{t("settings.database.pendingItems.recycleBin")}</li>
        </ul>
      </div>
    </SettingsSection>
  );
}
