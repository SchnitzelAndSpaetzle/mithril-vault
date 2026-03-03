// SPDX-License-Identifier: MIT

import { useEffect, useMemo, useState } from "react";
import { AlertTriangle, RotateCcw, Save } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Separator } from "@/components/ui/separator";
import { SettingsSection } from "@/components/settings/SettingsSection";
import { useAppPreferences } from "@/hooks/use-app-preferences";
import { useTheme } from "@/hooks/use-theme";
import { useActiveDatabase } from "@/hooks/use-active-database";
import { useDatabaseConfig } from "@/hooks/use-database-config";
import { toast } from "sonner";
import type { AppPreferences, KdfSettings, StartupBehavior } from "@/lib/types";

const THEME_OPTIONS = ["system", "light", "dark"] as const;
const FONT_SIZE_MIN = 10;
const FONT_SIZE_MAX = 24;

function isThemePreference(
  value: string
): value is AppPreferences["appearance"]["theme"] {
  return THEME_OPTIONS.includes(value as (typeof THEME_OPTIONS)[number]);
}

function parseAllowedSites(input: string): string[] {
  return input
    .split(/[\n,]/)
    .map((site) => site.trim())
    .filter((site) => site.length > 0);
}

function formatAllowedSites(sites: string[]): string {
  return sites.join("\n");
}

function formatKdf(kdf: KdfSettings): string {
  if (kdf.type === "aesKdf") {
    return `AES-KDF (${kdf.rounds} rounds)`;
  }

  return `${kdf.type} (${kdf.memory} bytes, ${kdf.iterations} iterations, ${kdf.parallelism} lanes)`;
}

interface SettingsEditorProps {
  initialPreferences: AppPreferences;
  onUpdatePreferences: (nextPreferences: AppPreferences) => Promise<void>;
  onResetPreferences: () => Promise<AppPreferences>;
  isBusy: boolean;
  dbId: string | null;
  databaseConfig: ReturnType<typeof useDatabaseConfig>["data"];
  isDatabaseConfigLoading: boolean;
  databaseConfigError: Error | null;
}

function SettingsEditor({
  initialPreferences,
  onUpdatePreferences,
  onResetPreferences,
  isBusy,
  dbId,
  databaseConfig,
  isDatabaseConfigLoading,
  databaseConfigError,
}: Readonly<SettingsEditorProps>) {
  const { setTheme } = useTheme();
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
  ) => {
    setDraft((previous) => updater(previous));
  };

  const updateStartupBehavior = (value: string) => {
    updateDraft((previous) => ({
      ...previous,
      general: {
        ...previous.general,
        startupBehavior: value as StartupBehavior,
      },
    }));
  };

  const saveChanges = async () => {
    const nextDraft: AppPreferences = {
      ...draft,
      browserIntegration: {
        ...draft.browserIntegration,
        allowedSites: parseAllowedSites(allowedSitesInput),
      },
    };

    try {
      await onUpdatePreferences(nextDraft);
      setDraft(nextDraft);
      toast.success("Settings updated");
    } catch (updateError) {
      toast.error(String(updateError));
    }
  };

  const resetToDefaults = async () => {
    try {
      const reset = await onResetPreferences();
      setDraft(reset);
      setAllowedSitesInput(
        formatAllowedSites(reset.browserIntegration.allowedSites)
      );
      setTheme(reset.appearance.theme);
      setIsResetDialogOpen(false);
      toast.success("Preferences reset to defaults");
    } catch (resetError) {
      toast.error(String(resetError));
    }
  };

  return (
    <div className="flex flex-1 flex-col gap-4 p-4 md:p-6">
      <div className="rounded-lg border bg-card p-4 flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
        <div>
          <h1 className="text-lg font-semibold">Settings</h1>
          <p className="text-sm text-muted-foreground">
            Application preferences and database configuration.
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
            Reset defaults
          </Button>
          <Button
            type="button"
            onClick={() => void saveChanges()}
            disabled={!hasChanges || isBusy}
          >
            <Save className="size-4" />
            Save changes
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
            <DialogTitle>Reset preferences?</DialogTitle>
            <DialogDescription>
              Reset all application preferences to defaults. Recent databases
              will be preserved.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={() => setIsResetDialogOpen(false)}
              disabled={isBusy}
            >
              Cancel
            </Button>
            <Button
              type="button"
              variant="destructive"
              onClick={() => void resetToDefaults()}
              disabled={isBusy}
            >
              Reset preferences
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <SettingsSection
        id="general"
        title="General"
        description="Startup and language preferences for the application."
      >
        <div className="grid gap-4 md:grid-cols-2">
          <div className="space-y-2">
            <Label htmlFor="language">Language</Label>
            <Input
              id="language"
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
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="startup-behavior">Startup behavior</Label>
            <select
              id="startup-behavior"
              className="w-full h-9 rounded-md border border-input bg-transparent px-3 text-sm"
              value={draft.general.startupBehavior}
              onChange={(event) => updateStartupBehavior(event.target.value)}
            >
              <option value="showUnlockScreen">Show unlock screen</option>
              <option value="openLastDatabase">Open last database</option>
              <option value="openDefaultDatabase">Open default database</option>
            </select>
          </div>
        </div>
        <div className="space-y-2">
          <Label htmlFor="default-database-path">Default database path</Label>
          <Input
            id="default-database-path"
            placeholder="/path/to/default.kdbx"
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
            TODO: startup flow does not yet auto-open this file.
          </p>
        </div>
      </SettingsSection>

      <SettingsSection
        id="security"
        title="Security"
        description="Clipboard and lock behavior preferences."
      >
        <div className="grid gap-4 md:grid-cols-2">
          <div className="space-y-2">
            <Label htmlFor="auto-lock-timeout">
              Auto-lock timeout (seconds)
            </Label>
            <Input
              id="auto-lock-timeout"
              type="number"
              min={30}
              value={draft.security.autoLockTimeout}
              onChange={(event) =>
                updateDraft((previous) => ({
                  ...previous,
                  security: {
                    ...previous.security,
                    autoLockTimeout: Math.max(
                      30,
                      Number(event.target.value) || 30
                    ),
                  },
                }))
              }
            />
            <p className="text-xs text-muted-foreground">
              TODO: inactivity/OS-lock event wiring is not implemented yet.
            </p>
          </div>
          <div className="space-y-2">
            <Label htmlFor="clipboard-timeout">
              Clipboard clear timeout (seconds)
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
            Show passwords by default
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
            Minimize to tray
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
            Start minimized
          </label>
        </div>
      </SettingsSection>

      <SettingsSection
        id="appearance"
        title="Appearance"
        description="Theme and visual preferences."
      >
        <div className="grid gap-4 md:grid-cols-2">
          <div className="space-y-2">
            <Label htmlFor="theme">Theme</Label>
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
                setTheme(nextTheme);
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
            <Label htmlFor="font-size">Font size</Label>
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
          <Label>Entry list columns</Label>
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
              Username
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
              URL
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
              Modified date
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
              Tags
            </label>
          </div>
          <p className="text-xs text-muted-foreground">
            TODO: entry list column toggles are stored but not yet wired into
            list rendering.
          </p>
        </div>
      </SettingsSection>

      <SettingsSection
        id="browser"
        title="Browser Integration"
        description="Extension integration and site allow-list settings."
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
          Enable browser integration
        </label>
        <div className="space-y-2">
          <Label htmlFor="allowed-sites">Allowed sites (one per line)</Label>
          <textarea
            id="allowed-sites"
            className="min-h-24 w-full rounded-md border border-input bg-transparent px-3 py-2 text-sm"
            value={allowedSitesInput}
            onChange={(event) => setAllowedSitesInput(event.target.value)}
          />
          <p className="text-xs text-muted-foreground">
            TODO: browser/native-messaging enforcement is not wired yet.
          </p>
        </div>
      </SettingsSection>

      <SettingsSection
        id="advanced"
        title="Advanced"
        description="Diagnostics and local data information."
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
          Enable debug mode
        </label>
        <div className="space-y-1">
          <Label>Data location</Label>
          <p className="text-sm text-muted-foreground break-all">
            {draft.advanced.dataLocation}
          </p>
        </div>
      </SettingsSection>

      <SettingsSection
        id="database"
        title="Database Settings"
        description="Current database cryptographic configuration (read-only)."
      >
        {!dbId ? (
          <p className="text-sm text-muted-foreground">
            Open a database to inspect its configuration.
          </p>
        ) : isDatabaseConfigLoading ? (
          <p className="text-sm text-muted-foreground">
            Loading database settings...
          </p>
        ) : databaseConfigError ? (
          <div className="rounded-md border border-destructive/40 bg-destructive/5 p-3 text-sm text-destructive">
            Failed to load database settings: {String(databaseConfigError)}
          </div>
        ) : databaseConfig ? (
          <div className="grid gap-2 text-sm">
            <div className="flex justify-between gap-4">
              <span className="text-muted-foreground">Version</span>
              <span>{databaseConfig.version}</span>
            </div>
            <div className="flex justify-between gap-4">
              <span className="text-muted-foreground">Outer cipher</span>
              <span>{databaseConfig.outerCipher}</span>
            </div>
            <div className="flex justify-between gap-4">
              <span className="text-muted-foreground">Inner cipher</span>
              <span>{databaseConfig.innerCipher}</span>
            </div>
            <div className="flex justify-between gap-4">
              <span className="text-muted-foreground">Compression</span>
              <span>{databaseConfig.compression}</span>
            </div>
            <div className="flex justify-between gap-4">
              <span className="text-muted-foreground">KDF</span>
              <span className="text-right">
                {formatKdf(databaseConfig.kdf)}
              </span>
            </div>
          </div>
        ) : (
          <p className="text-sm text-muted-foreground">
            No database configuration available.
          </p>
        )}

        <div className="rounded-md border border-amber-500/40 bg-amber-500/10 p-3 text-sm text-amber-900 dark:text-amber-200">
          <div className="flex items-center gap-2 font-medium">
            <AlertTriangle className="size-4" />
            Pending implementation
          </div>
          <ul className="mt-2 list-disc pl-5 space-y-1">
            <li>Database-level mutation controls are not implemented yet.</li>
            <li>Recycle-bin/history/security policy edits are TODO.</li>
          </ul>
        </div>
      </SettingsSection>
    </div>
  );
}

export function SettingsView() {
  const {
    preferences,
    isLoading,
    error,
    updatePreferences,
    isUpdating,
    resetPreferences,
    isResetting,
  } = useAppPreferences();
  const { dbId } = useActiveDatabase();
  const {
    data: databaseConfig,
    isLoading: isDatabaseConfigLoading,
    error: databaseConfigError,
  } = useDatabaseConfig(dbId);

  if (error) {
    return (
      <div className="p-4 md:p-6">
        <div className="rounded-lg border border-destructive/40 bg-destructive/5 p-6 text-sm text-destructive">
          Failed to load settings: {String(error)}
        </div>
      </div>
    );
  }

  if (isLoading) {
    return (
      <div className="p-4 md:p-6">
        <div className="rounded-lg border bg-card p-6 text-sm text-muted-foreground">
          Loading settings...
        </div>
      </div>
    );
  }

  if (!preferences) {
    return (
      <div className="p-4 md:p-6">
        <div className="rounded-lg border bg-card p-6 text-sm text-muted-foreground">
          Settings are unavailable.
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
      databaseConfig={databaseConfig}
      isDatabaseConfigLoading={isDatabaseConfigLoading}
      databaseConfigError={databaseConfigError ?? null}
    />
  );
}
