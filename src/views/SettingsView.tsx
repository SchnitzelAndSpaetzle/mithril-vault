// SPDX-License-Identifier: MIT

import { useTranslation } from "react-i18next";
import { SettingsEditor } from "@/components/settings/SettingsEditor";
import { useAppPreferences } from "@/hooks/use-app-preferences";
import { useActiveDatabase } from "@/hooks/use-active-database";
import { useDatabaseConfig } from "@/hooks/use-database-config";

export function SettingsView() {
  const { t } = useTranslation();
  const {
    preferences,
    isLoading,
    error,
    updatePreferences,
    isUpdating,
    resetPreferences,
    isResetting,
  } = useAppPreferences();
  const { dbId, isLocked } = useActiveDatabase();
  const {
    data: databaseConfig,
    isLoading: isDatabaseConfigLoading,
    error: databaseConfigError,
  } = useDatabaseConfig(dbId);

  if (error) {
    return (
      <div className="p-4 md:p-6">
        <div className="rounded-lg border border-destructive/40 bg-destructive/5 p-6 text-sm text-destructive">
          {t("errors.failedToLoadSettings", { error: String(error) })}
        </div>
      </div>
    );
  }

  if (isLoading) {
    return (
      <div className="p-4 md:p-6">
        <div className="rounded-lg border bg-card p-6 text-sm text-muted-foreground">
          {t("common.loading")}
        </div>
      </div>
    );
  }

  if (!preferences) {
    return (
      <div className="p-4 md:p-6">
        <div className="rounded-lg border bg-card p-6 text-sm text-muted-foreground">
          {t("errors.settingsUnavailable")}
        </div>
      </div>
    );
  }

  return (
    <SettingsEditor
      key={JSON.stringify(preferences)}
      initialPreferences={preferences}
      onUpdatePreferences={updatePreferences}
      onResetPreferences={resetPreferences}
      isBusy={isUpdating || isResetting}
      dbId={dbId}
      isLocked={isLocked}
      databaseConfig={databaseConfig ?? null}
      isDatabaseConfigLoading={isDatabaseConfigLoading}
      databaseConfigError={databaseConfigError ?? null}
    />
  );
}
