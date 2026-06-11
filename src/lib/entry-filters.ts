// SPDX-License-Identifier: MIT

import type { Entry } from "@/lib/types";
import { entryHasTag } from "@/lib/tag-utils";
import { entryHasAttachments } from "@/lib/attachment-utils";

export interface EntryFilters {
  /// Restrict to entries carrying this tag. Falsy disables the filter.
  tag?: string | null | undefined;
  /// Restrict to entries with at least one attachment. Falsy disables it.
  hasAttachments?: boolean | undefined;
}

/// Apply the stackable entry-list filters. Each active filter is
/// AND-combined, so an entry must satisfy every set filter to pass.
/// Shared by the entry list and the search hook so "filters stack"
/// has a single definition.
export function filterEntries(
  entries: Entry[],
  filters: EntryFilters
): Entry[] {
  const { tag, hasAttachments } = filters;
  return entries.filter(
    (entry) =>
      (!tag || entryHasTag(entry, tag)) &&
      (!hasAttachments || entryHasAttachments(entry))
  );
}

/// Why the (already-filtered) entry list is empty, so the UI can pick
/// an accurate empty-state message. When more than one filter is active
/// neither single-filter reason is truthful — a tag may have entries
/// while none carry an attachment — so it reports the combined
/// `"filters"` reason instead.
export type EmptyEntryListReason = "none" | "tag" | "attachments" | "filters";

export function emptyEntryListReason(
  filters: EntryFilters
): EmptyEntryListReason {
  const hasTag = Boolean(filters.tag);
  const hasAttachments = filters.hasAttachments === true;
  if (hasTag && hasAttachments) return "filters";
  if (hasTag) return "tag";
  if (hasAttachments) return "attachments";
  return "none";
}
