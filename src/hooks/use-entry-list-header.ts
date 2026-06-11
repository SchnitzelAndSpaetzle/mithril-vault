// SPDX-License-Identifier: MIT

import { useActiveDatabase } from "@/hooks/use-active-database";
import { useEntries } from "@/hooks/use-entries";
import { useGroups } from "@/hooks/use-groups";
import { useSearch } from "@tanstack/react-router";
import { useMemo } from "react";
import type { Group } from "@/lib/types";
import { filterEntries } from "@/lib/entry-filters";

function findGroupById(groups: Group[], id: string): Group | null {
  for (const group of groups) {
    if (group.id === id) return group;
    const found = findGroupById(group.children, id);
    if (found) return found;
  }
  return null;
}

export function useEntryListHeader() {
  const { dbId } = useActiveDatabase();
  const search = useSearch({ strict: false });
  const { data: entries } = useEntries(dbId, search.groupId);
  const { data: groups } = useGroups(dbId);

  const activeTag = search.tag ?? null;
  const hasAttachments = search.hasAttachments === true;

  // Count what the list actually shows: reuse the shared filter so the
  // header count stays in lockstep with the visible (tag + attachment)
  // filtered entries.
  const entryCount = useMemo(() => {
    if (!entries) return 0;
    return filterEntries(entries, { tag: activeTag, hasAttachments }).length;
  }, [entries, activeTag, hasAttachments]);

  let groupName = "All";
  if (search.groupId && groups) {
    const group = findGroupById(groups, search.groupId);
    if (group) {
      groupName = group.name;
    }
  }

  return { groupName, entryCount, activeTag };
}
