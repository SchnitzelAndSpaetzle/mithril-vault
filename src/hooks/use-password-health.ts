// SPDX-License-Identifier: MIT

import { useQuery } from "@tanstack/react-query";

import {
  findingsForEntry,
  summarize,
  type PasswordHealthSummary,
} from "@/lib/password-health";
import { queryKeys } from "@/lib/query-keys";
import { passwordHealth } from "@/lib/tauri";
import type { Finding, PasswordHealthReport } from "@/lib/types";

/// React Query wrapper around `passwordHealth.getReport`. The backend
/// caches on `(dbId, generation)`, so this query's cache is mostly a
/// freshness mirror — the backend is the source of truth. Per-Entry
/// progressive events will piggyback on this query's cache via
/// `queryClient.setQueryData` in a follow-up cycle; the wire shape and
/// React Query key are deliberately stable so that wiring is additive.
export function usePasswordHealthReport(dbId: string | null | undefined) {
  return useQuery<PasswordHealthReport>({
    queryKey: dbId
      ? queryKeys.passwordHealth.report(dbId)
      : ["password-health", "idle"],
    queryFn: () => passwordHealth.getReport(dbId as string),
    enabled: Boolean(dbId),
  });
}

/// Pure-selector hook reading the report-as-cached and deriving the
/// sidebar badge inputs. Returning an idle summary when the report
/// isn't loaded yet keeps the sidebar render path simple — the badge
/// hides on `totalUnhealthy === 0`, which also matches the idle case.
export function usePasswordHealthSummary(
  dbId: string | null | undefined
): PasswordHealthSummary {
  const { data } = usePasswordHealthReport(dbId);
  return summarize(data);
}

/// Returns the Findings scoped to a single Entry. The `EntryListItem`
/// reads this to decide whether to render the warning icon and which
/// severity colour to use. Returns an empty array when the report
/// isn't loaded yet so the list renders icon-less rather than blocking.
export function useEntryFindings(
  dbId: string | null | undefined,
  entryId: string
): Finding[] {
  const { data } = usePasswordHealthReport(dbId);
  return findingsForEntry(data, entryId);
}
