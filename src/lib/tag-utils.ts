// SPDX-License-Identifier: MIT

import type { Entry } from "@/lib/types";

const TAG_SPLIT_PATTERN = /[;,]/;

function splitTagValue(value: string): string[] {
  return value
    .split(TAG_SPLIT_PATTERN)
    .map((part) => part.trim())
    .filter((part) => part.length > 0);
}

export function getNormalizedEntryTags(entry: Entry): string[] {
  const tags: string[] = [];

  for (const tag of entry.tags) {
    tags.push(...splitTagValue(tag));
  }

  const customTags = entry.customFields["Tags"] ?? entry.customFields["tags"];
  if (customTags) {
    tags.push(...splitTagValue(customTags));
  }

  return tags;
}

export function entryHasTag(entry: Entry, tag: string): boolean {
  return getNormalizedEntryTags(entry).includes(tag);
}

export function collectUniqueTags(entryList: Entry[]): string[] {
  const tags = new Set<string>();

  for (const entry of entryList) {
    for (const tag of getNormalizedEntryTags(entry)) {
      tags.add(tag);
    }
  }

  return Array.from(tags).sort((a, b) =>
    a.localeCompare(b, undefined, { sensitivity: "base" })
  );
}
