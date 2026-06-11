// SPDX-License-Identifier: MIT

import { useCallback, useMemo, useState } from "react";
import { useDebounce } from "@/hooks/use-debounce";
import { useEntries } from "@/hooks/use-entries";
import { searchEntries, type SearchResult } from "@/lib/search-utils";
import { filterEntries } from "@/lib/entry-filters";

const DEBOUNCE_MS = 200;

export interface SearchState {
  query: string;
  setQuery: (query: string) => void;
  results: SearchResult[];
  isSearchActive: boolean;
  clearSearch: () => void;
}

/**
 * Build a stable scope key from groupId, tag, and the has-attachments
 * filter. When the scope changes, the query is automatically cleared via
 * React's "adjusting state during render" pattern (recommended over
 * useEffect for derived resets).
 */
function scopeKey(
  groupId?: string | null,
  tag?: string | null,
  hasAttachments?: boolean
): string {
  return `${groupId ?? ""}\0${tag ?? ""}\0${hasAttachments ? "1" : ""}`;
}

export function useSearchEntries(
  dbId: string | null,
  groupId?: string | null,
  tag?: string | null,
  hasAttachments?: boolean
): SearchState {
  const [query, setQuery] = useState("");
  const debouncedQuery = useDebounce(query, DEBOUNCE_MS);
  const { data: entries } = useEntries(dbId, groupId);

  // Auto-clear search when the scope (group / tag / attachments) changes
  const currentScope = scopeKey(groupId, tag, hasAttachments);
  const [prevScope, setPrevScope] = useState(currentScope);

  if (currentScope !== prevScope) {
    setPrevScope(currentScope);
    setQuery("");
  }

  // Filters stack: narrow by tag and by has-attachments before the
  // text search runs, so the active filters keep applying while the
  // user searches (shares `filterEntries` with the entry list).
  const filteredEntries = useMemo(
    () => (entries ? filterEntries(entries, { tag, hasAttachments }) : []),
    [entries, tag, hasAttachments]
  );

  const results = useMemo(
    () => searchEntries(filteredEntries, debouncedQuery),
    [filteredEntries, debouncedQuery]
  );

  const isSearchActive = query.trim().length > 0;

  const clearSearch = useCallback(() => setQuery(""), []);

  return { query, setQuery, results, isSearchActive, clearSearch };
}
