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
