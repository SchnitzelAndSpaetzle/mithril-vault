// SPDX-License-Identifier: MIT

import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import type { TFunction } from "i18next";
import { toast } from "sonner";

import { queryKeys } from "@/lib/query-keys";
import { database } from "@/lib/tauri.ts";
import type { MergeSummary } from "@/lib/types";

/**
 * Builds the Merge Summary toast line, e.g.
 * "2 entries added, 1 conflict — Netflix: newer version kept, other in history".
 *
 * Only non-zero counts are mentioned; the first conflict is named so the
 * user knows where to look for the preserved version (a full review
 * surface is a later slice).
 */
export function formatMergeSummary(
  summary: MergeSummary,
  t: TFunction
): string {
  const parts: string[] = [];
  if (summary.entriesAdded > 0) {
    parts.push(
      t("database.merge.toast.added", { count: summary.entriesAdded })
    );
  }
  if (summary.entriesUpdated > 0) {
    parts.push(
      t("database.merge.toast.updated", { count: summary.entriesUpdated })
    );
  }
  if (summary.entriesDeleted > 0) {
    parts.push(
      t("database.merge.toast.deleted", { count: summary.entriesDeleted })
    );
  }
  if (summary.conflicts.length > 0) {
    parts.push(
      t("database.merge.toast.conflicts", { count: summary.conflicts.length })
    );
  }
  if (parts.length === 0) {
    return t("database.merge.toast.noChanges");
  }
  const firstConflict = summary.conflicts[0];
  const conflictDetail = firstConflict
    ? ` — ${t("database.merge.toast.conflictDetail", { title: firstConflict.title })}`
    : "";
  return parts.join(", ") + conflictDetail;
}

/**
 * "Merge from file…": the backend picks the second KDBX file via the
 * native dialog, merges it into the open vault, saves (with pre-merge
 * backup), and returns the Merge Summary — rendered here as a toast.
 * Security-posture differences are never applied by the merge; they get
 * their own warning toast.
 */
export function useMergeFromFile(dbId: string | null) {
  const queryClient = useQueryClient();
  const { t } = useTranslation();

  return useMutation<MergeSummary | null, Error, void>({
    mutationFn: async () => {
      if (!dbId) {
        return null;
      }
      return database.mergeFromFile(dbId);
    },
    onSuccess: (summary) => {
      if (!summary) {
        // User cancelled the file pick — nothing happened.
        return;
      }
      void queryClient.invalidateQueries({ queryKey: queryKeys.entries.all });
      void queryClient.invalidateQueries({ queryKey: queryKeys.groups.all });
      void queryClient.invalidateQueries({ queryKey: queryKeys.database.all });
      toast.success(formatMergeSummary(summary, t));
      if (summary.securityPostureChanges.length > 0) {
        toast.warning(t("database.merge.toast.securityPosture"));
      }
    },
    onError: (error) => {
      toast.error(t("database.merge.toast.failed", { error: error.message }));
    },
  });
}
