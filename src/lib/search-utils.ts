// SPDX-License-Identifier: MIT

import type { Entry, Group } from "@/lib/types";
import { getNormalizedEntryTags } from "@/lib/tag-utils";

export type MatchedField = "title" | "username" | "url" | "notes" | "tags";

export interface SearchResult {
  entry: Entry;
  matchedFields: MatchedField[];
}

export interface TextSegment {
  text: string;
  highlighted: boolean;
}

/**
 * Search entries by matching query against title, username, url, notes, and tags.
 * Results are sorted by relevance (title > username > url > notes/tags), then alphabetically.
 * Returns empty array for empty/whitespace-only queries.
 */
export function searchEntries(entries: Entry[], query: string): SearchResult[] {
  const trimmed = query.trim().toLowerCase();
  if (!trimmed) return [];

  const results: SearchResult[] = [];

  for (const entry of entries) {
    const matchedFields: MatchedField[] = [];

    if (entry.title.toLowerCase().includes(trimmed)) {
      matchedFields.push("title");
    }
    if (entry.username.toLowerCase().includes(trimmed)) {
      matchedFields.push("username");
    }
    if (entry.url?.toLowerCase().includes(trimmed)) {
      matchedFields.push("url");
    }
    if (entry.notes?.toLowerCase().includes(trimmed)) {
      matchedFields.push("notes");
    }
    if (
      getNormalizedEntryTags(entry).some((tag) =>
        tag.toLowerCase().includes(trimmed)
      )
    ) {
      matchedFields.push("tags");
    }

    if (matchedFields.length > 0) {
      results.push({ entry, matchedFields });
    }
  }

  return results.sort((a, b) => {
    const scoreA = getRelevanceScore(a.matchedFields);
    const scoreB = getRelevanceScore(b.matchedFields);
    if (scoreA !== scoreB) return scoreB - scoreA;
    return a.entry.title.localeCompare(b.entry.title);
  });
}

const FIELD_SCORES: Record<MatchedField, number> = {
  title: 100,
  username: 50,
  url: 25,
  notes: 10,
  tags: 10,
};

function getRelevanceScore(fields: MatchedField[]): number {
  let score = 0;
  for (const field of fields) {
    score += FIELD_SCORES[field];
  }
  return score;
}

/**
 * Build a map from group ID to full path string (e.g. "Root > Sub > Leaf").
 * Traverses the recursive Group tree once.
 */
export function buildGroupPathMap(groups: Group[]): Map<string, string> {
  const map = new Map<string, string>();

  function traverse(group: Group, parentPath: string) {
    const path = parentPath ? `${parentPath} > ${group.name}` : group.name;
    map.set(group.id, path);
    for (const child of group.children) {
      traverse(child, path);
    }
  }

  for (const group of groups) {
    traverse(group, "");
  }

  return map;
}

/**
 * Split text into segments, marking portions that match the query as highlighted.
 * Case-insensitive matching. Returns a single unhighlighted segment for empty queries.
 */
export function highlightMatches(text: string, query: string): TextSegment[] {
  const trimmed = query.trim();
  if (!trimmed || !text) {
    return [{ text, highlighted: false }];
  }

  const segments: TextSegment[] = [];
  const lowerText = text.toLowerCase();
  const lowerQuery = trimmed.toLowerCase();
  let lastIndex = 0;

  let index = lowerText.indexOf(lowerQuery);
  while (index !== -1) {
    if (index > lastIndex) {
      segments.push({
        text: text.slice(lastIndex, index),
        highlighted: false,
      });
    }
    segments.push({
      text: text.slice(index, index + trimmed.length),
      highlighted: true,
    });
    lastIndex = index + trimmed.length;
    index = lowerText.indexOf(lowerQuery, lastIndex);
  }

  if (lastIndex < text.length) {
    segments.push({ text: text.slice(lastIndex), highlighted: false });
  }

  return segments;
}
