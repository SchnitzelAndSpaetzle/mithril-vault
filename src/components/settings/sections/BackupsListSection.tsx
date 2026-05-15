// SPDX-License-Identifier: MIT

import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { listen } from "@tauri-apps/api/event";
import { ask } from "@tauri-apps/plugin-dialog";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { SettingsSection } from "@/components/settings/SettingsSection";
import { queryKeys } from "@/lib/query-keys";
import { backups } from "@/lib/tauri";
import type { BackupListEntry, BackupKind } from "@/lib/types";

interface BackupsListSectionProps {
  dbId: string | null;
  /**
   * Mirrors `draft.backups.enabled` from the parent settings form. The
   * "Create backup now" button gates on it: even though the manual-backup
   * command itself ignores the toggle, the user has explicitly turned auto
   * backups off — surfacing a clickable button here would feel inconsistent.
   * Defaults to `true` for tests / callers that don't pass it.
   */
  backupsEnabled?: boolean;
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
}

function formatTimestamp(timestamp: string, locale: string): string {
  const parsed = new Date(timestamp);
  if (Number.isNaN(parsed.getTime())) return timestamp;
  return parsed.toLocaleString(locale, {
    dateStyle: "medium",
    timeStyle: "medium",
  });
}

function BackupRow({
  entry,
  isConfirming,
  onRequestDelete,
  onConfirmDelete,
  onCancelDelete,
  onRestore,
  isDeleting,
  isRestoring,
}: Readonly<{
  entry: BackupListEntry;
  isConfirming: boolean;
  onRequestDelete: () => void;
  onConfirmDelete: () => void;
  onCancelDelete: () => void;
  onRestore: () => void;
  isDeleting: boolean;
  isRestoring: boolean;
}>) {
  const { t, i18n } = useTranslation();
  const kindKey: BackupKind = entry.kind;

  return (
    <li className="flex flex-col gap-2 rounded-md border bg-card/50 p-3 text-sm md:flex-row md:items-center md:justify-between">
      <div className="flex flex-col gap-1">
        <span className="font-medium">
          {formatTimestamp(entry.timestamp, i18n.language)}
        </span>
        <div className="flex items-center gap-2 text-xs text-muted-foreground">
          <span>{formatBytes(entry.sizeBytes)}</span>
          <span
            className="rounded-sm border px-1.5 py-0.5 uppercase"
            data-kind={kindKey}
          >
            {t(`settings.backups.list.kind.${kindKey}`)}
          </span>
        </div>
      </div>
      <div className="flex shrink-0 gap-2">
        {isConfirming ? (
          <>
            <Button
              type="button"
              variant="outline"
              size="sm"
              onClick={onCancelDelete}
              disabled={isDeleting}
            >
              {t("common.cancel")}
            </Button>
            <Button
              type="button"
              variant="destructive"
              size="sm"
              onClick={onConfirmDelete}
              disabled={isDeleting}
            >
              {t("settings.backups.list.deleteConfirm")}
            </Button>
          </>
        ) : (
          <>
            <Button
              type="button"
              variant="outline"
              size="sm"
              onClick={onRestore}
              disabled={isRestoring}
            >
              {t("settings.backups.list.restore.button")}
            </Button>
            <Button
              type="button"
              variant="outline"
              size="sm"
              onClick={onRequestDelete}
            >
              {t("settings.backups.list.delete")}
            </Button>
          </>
        )}
      </div>
    </li>
  );
}

/**
 * Internal sentinel so the dialog-cancelled path settles the mutation
 * without flashing a toast. Plain `Error` would be picked up by the
 * generic onError handler.
 */
class RestoreCancelled extends Error {
  constructor() {
    super("restore-cancelled");
    this.name = "RestoreCancelled";
  }
}

export function BackupsListSection({
  dbId,
  backupsEnabled = true,
}: Readonly<BackupsListSectionProps>) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [confirmingPath, setConfirmingPath] = useState<string | null>(null);

  const createManualMutation = useMutation<unknown, Error, string>({
    mutationFn: (path) => backups.createManual(path),
    onSuccess: () => {
      toast.success(t("settings.backups.list.createNow.success"));
    },
    onError: (error) => {
      toast.error(String(error));
    },
  });

  const query = useQuery<BackupListEntry[], Error>({
    queryKey: queryKeys.backups.list(dbId ?? "none"),
    queryFn: () => (dbId ? backups.list(dbId) : Promise.resolve([])),
    enabled: Boolean(dbId),
  });

  useEffect(() => {
    if (!dbId) return;
    const queryKey = queryKeys.backups.list(dbId);
    const invalidate = () => {
      void queryClient.invalidateQueries({ queryKey });
    };
    const unlistenCreated = listen("backup-created", invalidate);
    const unlistenDeleted = listen("backup-deleted", invalidate);
    const onFocus = () => invalidate();
    window.addEventListener("focus", onFocus);
    return () => {
      void unlistenCreated.then((dispose) => dispose());
      void unlistenDeleted.then((dispose) => dispose());
      window.removeEventListener("focus", onFocus);
    };
  }, [dbId, queryClient]);

  const deleteMutation = useMutation<void, Error, string>({
    mutationFn: (path) => backups.delete(path),
    onSuccess: () => {
      setConfirmingPath(null);
    },
    onError: (error) => {
      toast.error(String(error));
    },
  });

  const restoreMutation = useMutation<void, Error, string>({
    mutationFn: async (path) => {
      // Master-password caveat: a backup encrypted with a different master
      // password than the one currently in use will fail to unlock after
      // restore. The dialog warns the user about this AND the automatic
      // pre-restore snapshot so the action is never silently destructive.
      const confirmed = await ask(
        t("settings.backups.list.restore.confirmBody"),
        {
          title: t("settings.backups.list.restore.confirmTitle"),
          kind: "warning",
        }
      );
      if (!confirmed) {
        // Surface the cancellation as a no-op so the mutation settles
        // cleanly without flashing an error toast.
        throw new RestoreCancelled();
      }
      await backups.restore(path);
    },
    onSuccess: () => {
      toast.success(t("settings.backups.list.restore.success"));
    },
    onError: (error) => {
      if (error instanceof RestoreCancelled) return;
      toast.error(String(error));
    },
  });

  const canCreateManual =
    Boolean(dbId) && backupsEnabled && !createManualMutation.isPending;

  return (
    <SettingsSection
      id="backups-list"
      title={t("settings.backups.list.title")}
      description={t("settings.backups.list.description")}
    >
      <div className="flex justify-end">
        <Button
          type="button"
          variant="secondary"
          size="sm"
          disabled={!canCreateManual}
          onClick={() => {
            if (!dbId) return;
            createManualMutation.mutate(dbId);
          }}
        >
          {t("settings.backups.list.createNow.button")}
        </Button>
      </div>
      {dbId ? (
        <BackupsListBody
          entries={query.data ?? []}
          isLoading={query.isLoading}
          error={query.error}
          confirmingPath={confirmingPath}
          onRequestDelete={(path) => setConfirmingPath(path)}
          onCancelDelete={() => setConfirmingPath(null)}
          onConfirmDelete={(path) => deleteMutation.mutate(path)}
          onRestore={(path) => restoreMutation.mutate(path)}
          isDeleting={deleteMutation.isPending}
          isRestoring={restoreMutation.isPending}
          pendingRestorePath={
            restoreMutation.isPending
              ? (restoreMutation.variables ?? null)
              : null
          }
        />
      ) : (
        <p className="text-sm text-muted-foreground">
          {t("settings.backups.list.emptyNoVault")}
        </p>
      )}
    </SettingsSection>
  );
}

function BackupsListBody({
  entries,
  isLoading,
  error,
  confirmingPath,
  onRequestDelete,
  onCancelDelete,
  onConfirmDelete,
  onRestore,
  isDeleting,
  isRestoring,
  pendingRestorePath,
}: Readonly<{
  entries: BackupListEntry[];
  isLoading: boolean;
  error: Error | null;
  confirmingPath: string | null;
  onRequestDelete: (path: string) => void;
  onCancelDelete: () => void;
  onConfirmDelete: (path: string) => void;
  onRestore: (path: string) => void;
  isDeleting: boolean;
  isRestoring: boolean;
  pendingRestorePath: string | null;
}>) {
  const { t } = useTranslation();

  if (isLoading) {
    return (
      <p className="text-sm text-muted-foreground">{t("common.loading")}</p>
    );
  }
  if (error) {
    return (
      <p className="text-sm text-destructive">
        {t("settings.backups.list.loadError", { error: String(error) })}
      </p>
    );
  }
  if (entries.length === 0) {
    return (
      <p className="text-sm text-muted-foreground">
        {t("settings.backups.list.emptyNoBackups")}
      </p>
    );
  }
  return (
    <ul className="flex flex-col gap-2">
      {entries.map((entry) => (
        <BackupRow
          key={entry.path}
          entry={entry}
          isConfirming={confirmingPath === entry.path}
          onRequestDelete={() => onRequestDelete(entry.path)}
          onConfirmDelete={() => onConfirmDelete(entry.path)}
          onCancelDelete={onCancelDelete}
          onRestore={() => onRestore(entry.path)}
          isDeleting={isDeleting && confirmingPath === entry.path}
          isRestoring={isRestoring && pendingRestorePath === entry.path}
        />
      ))}
    </ul>
  );
}
