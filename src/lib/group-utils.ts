// SPDX-License-Identifier: MIT

import type { Group } from "./types";

export interface FlatGroup {
  id: string;
  name: string;
  depth: number;
}

export function flattenGroups(groups: Group[], depth = 0): FlatGroup[] {
  const result: FlatGroup[] = [];
  for (const group of groups) {
    result.push({ id: group.id, name: group.name, depth });
    if (group.children.length > 0) {
      result.push(...flattenGroups(group.children, depth + 1));
    }
  }
  return result;
}

/**
 * Recursively sum entry counts for a group and all its descendants.
 */
export function sumGroupEntryCounts(
  group: Group,
  entryCounts: Record<string, number>
): number {
  let total = entryCounts[group.id] ?? 0;
  for (const child of group.children) {
    total += sumGroupEntryCounts(child, entryCounts);
  }
  return total;
}

/**
 * Collect all descendant group IDs (not including the group itself).
 */
export function getDescendantIds(group: Group): string[] {
  const ids: string[] = [];
  for (const child of group.children) {
    ids.push(child.id);
    ids.push(...getDescendantIds(child));
  }
  return ids;
}
