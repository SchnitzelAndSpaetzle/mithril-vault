// SPDX-License-Identifier: MIT

import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { RotateCcw, Save } from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { useTheme } from "@/hooks/use-theme";
import type {
  AppPreferences,
  DatabaseConfig,
  StartupBehavior,
} from "@/lib/types";
import {
  formatAllowedSites,
  parseAllowedSites,
} from "@/components/settings/settings-utils";
import { AdvancedSettingsSection } from "@/components/settings/sections/AdvancedSettingsSection";
import { AppearanceSettingsSection } from "@/components/settings/sections/AppearanceSettingsSection";
import { BrowserIntegrationSettingsSection } from "@/components/settings/sections/BrowserIntegrationSettingsSection";
import { DatabaseSettingsSection } from "@/components/settings/sections/DatabaseSettingsSection";
import { GeneralSettingsSection } from "@/components/settings/sections/GeneralSettingsSection";
import { SecuritySettingsSection } from "@/components/settings/sections/SecuritySettingsSection";

interface SettingsEditorProps {
  initialPreferences: AppPreferences;
  onUpdatePreferences: (nextPreferences: AppPreferences) => Promise<void>;
  onResetPreferences: () => Promise<AppPreferences>;
  isBusy: boolean;
  dbId: string | null;
  databaseConfig: DatabaseConfig | null;
  isDatabaseConfigLoading: boolean;
  databaseConfigError: Error | null;
}

interface UseSettingsEditorStateArgs {
  initialPreferences: AppPreferences;
  onUpdatePreferences: (nextPreferences: AppPreferences) => Promise<void>;
  onResetPreferences: () => Promise<AppPreferences>;
}

function useSettingsEditorState({
  initialPreferences,
  onUpdatePreferences,
  onResetPreferences,
}: Readonly<UseSettingsEditorStateArgs>) {
  const { t } = useTranslation();
  const { setTheme, setThemePreview } = useTheme();
  const [isResetDialogOpen, setIsResetDialogOpen] = useState(false);
  const [draft, setDraft] = useState<AppPreferences>(initialPreferences);
  const [allowedSitesInput, setAllowedSitesInput] = useState<string>(() =>
    formatAllowedSites(initialPreferences.browserIntegration.allowedSites)
  );

  useEffect(() => {
    setTheme(initialPreferences.appearance.theme);
  }, [initialPreferences.appearance.theme, setTheme]);

  const hasChanges = useMemo(() => {
    const normalizedDraft: AppPreferences = {
      ...draft,
      browserIntegration: {
        ...draft.browserIntegration,
        allowedSites: parseAllowedSites(allowedSitesInput),
      },
    };

    return (
      JSON.stringify(initialPreferences) !== JSON.stringify(normalizedDraft)
    );
  }, [allowedSitesInput, draft, initialPreferences]);

  const updateDraft = (
    updater: (previous: AppPreferences) => AppPreferences
  ): void => {
    setDraft((previous) => updater(previous));
  };

  const updateStartupBehavior = (value: string): void => {
    updateDraft((previous) => ({
      ...previous,
      general: {
        ...previous.general,
        startupBehavior: value as StartupBehavior,
      },
    }));
  };

  const saveChanges = async (): Promise<void> => {
    const nextDraft: AppPreferences = {
      ...draft,
      browserIntegration: {
        ...draft.browserIntegration,
        allowedSites: parseAllowedSites(allowedSitesInput),
      },
    };

    try {
      await onUpdatePreferences(nextDraft);
      setTheme(nextDraft.appearance.theme);
      setDraft(nextDraft);
      toast.success(t("settings.toast.updated"));
    } catch (updateError) {
      toast.error(String(updateError));
    }
  };

  const resetToDefaults = async (): Promise<void> => {
    try {
      const reset = await onResetPreferences();
      setDraft(reset);
      setAllowedSitesInput(
        formatAllowedSites(reset.browserIntegration.allowedSites)
      );
      setTheme(reset.appearance.theme);
      setIsResetDialogOpen(false);
      toast.success(t("settings.toast.reset"));
    } catch (resetError) {
      toast.error(String(resetError));
    }
  };

  return {
    allowedSitesInput,
    draft,
    hasChanges,
    isResetDialogOpen,
    previewTheme: setThemePreview,
    resetToDefaults,
    saveChanges,
    setAllowedSitesInput,
    setIsResetDialogOpen,
    updateDraft,
    updateStartupBehavior,
  };
}

export function SettingsEditor({
  initialPreferences,
  onUpdatePreferences,
  onResetPreferences,
  isBusy,
  dbId,
  databaseConfig,
  isDatabaseConfigLoading,
  databaseConfigError,
}: Readonly<SettingsEditorProps>) {
  const { t } = useTranslation();
  const {
    allowedSitesInput,
    draft,
    hasChanges,
    isResetDialogOpen,
    previewTheme,
    resetToDefaults,
    saveChanges,
    setAllowedSitesInput,
    setIsResetDialogOpen,
    updateDraft,
    updateStartupBehavior,
  } = useSettingsEditorState({
    initialPreferences,
    onUpdatePreferences,
    onResetPreferences,
  });

  return (
    <div className="flex flex-1 flex-col gap-4 p-4 md:p-6">
      <div className="rounded-lg border bg-card p-4 flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
        <div>
          <h1 className="text-lg font-semibold">{t("settings.title")}</h1>
          <p className="text-sm text-muted-foreground">
            {t("settings.description")}
          </p>
        </div>
        <div className="flex items-center gap-2">
          <Button
            type="button"
            variant="outline"
            onClick={() => setIsResetDialogOpen(true)}
            disabled={isBusy}
          >
            <RotateCcw className="size-4" />
            {t("settings.resetDefaults")}
          </Button>
          <Button
            type="button"
            onClick={() => void saveChanges()}
            disabled={!hasChanges || isBusy}
          >
            <Save className="size-4" />
            {t("settings.saveChanges")}
          </Button>
        </div>
      </div>
      <Dialog
        open={isResetDialogOpen}
        onOpenChange={(open) => {
          if (!isBusy) {
            setIsResetDialogOpen(open);
          }
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t("settings.resetDialog.title")}</DialogTitle>
            <DialogDescription>
              {t("settings.resetDialog.description")}
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={() => setIsResetDialogOpen(false)}
              disabled={isBusy}
            >
              {t("common.cancel")}
            </Button>
            <Button
              type="button"
              variant="destructive"
              onClick={() => void resetToDefaults()}
              disabled={isBusy}
            >
              {t("settings.resetPreferences")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <GeneralSettingsSection
        draft={draft}
        updateDraft={updateDraft}
        onStartupBehaviorChange={updateStartupBehavior}
      />
      <SecuritySettingsSection draft={draft} updateDraft={updateDraft} />
      <AppearanceSettingsSection
        draft={draft}
        updateDraft={updateDraft}
        onThemePreview={previewTheme}
      />
      <BrowserIntegrationSettingsSection
        draft={draft}
        allowedSitesInput={allowedSitesInput}
        setAllowedSitesInput={setAllowedSitesInput}
        updateDraft={updateDraft}
      />
      <AdvancedSettingsSection draft={draft} updateDraft={updateDraft} />
      <DatabaseSettingsSection
        dbId={dbId}
        databaseConfig={databaseConfig}
        isDatabaseConfigLoading={isDatabaseConfigLoading}
        databaseConfigError={databaseConfigError}
      />
    </div>
  );
}
