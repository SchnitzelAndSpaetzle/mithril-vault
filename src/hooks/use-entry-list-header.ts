// SPDX-License-Identifier: MIT

import { useActiveDatabase } from "@/hooks/use-active-database";
import { useEntries } from "@/hooks/use-entries";
import { useGroups } from "@/hooks/use-groups";
import { useSearch } from "@tanstack/react-router";
import { useMemo } from "react";
import type { Group } from "@/lib/types";

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

  const activeTag = (search.tag as string | undefined) ?? null;

  const entryCount = useMemo(() => {
    if (!entries) return 0;
    if (activeTag) {
      return entries.filter((e) => e.tags.includes(activeTag)).length;
    }
    return entries.length;
  }, [entries, activeTag]);

  let groupName = "All";
  if (search.groupId && groups) {
    const group = findGroupById(groups, search.groupId);
    if (group) {
      groupName = group.name;
    }
  }

  return { groupName, entryCount, activeTag };
}
