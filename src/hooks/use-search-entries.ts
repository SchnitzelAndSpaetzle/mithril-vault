// SPDX-License-Identifier: MIT

import { useCallback, useMemo, useState } from "react";
import { useDebounce } from "@/hooks/use-debounce";
import { useEntries } from "@/hooks/use-entries";
import { searchEntries, type SearchResult } from "@/lib/search-utils";
import { entryHasTag } from "@/lib/tag-utils";

const DEBOUNCE_MS = 200;

export interface SearchState {
  query: string;
  setQuery: (query: string) => void;
  results: SearchResult[];
  isSearchActive: boolean;
  clearSearch: () => void;
}

/**
 * Build a stable scope key from groupId and tag. When the scope changes,
 * the query is automatically cleared via React's "adjusting state during
 * render" pattern (recommended over useEffect for derived resets).
 */
function scopeKey(groupId?: string | null, tag?: string | null): string {
  return `${groupId ?? ""}\0${tag ?? ""}`;
}

export function useSearchEntries(
  dbId: string | null,
  groupId?: string | null,
  tag?: string | null
): SearchState {
  const [query, setQuery] = useState("");
  const debouncedQuery = useDebounce(query, DEBOUNCE_MS);
  const { data: entries } = useEntries(dbId, groupId);

  // Auto-clear search when group or tag changes (render-time reset)
  const currentScope = scopeKey(groupId, tag);
  const [prevScope, setPrevScope] = useState(currentScope);

  if (currentScope !== prevScope) {
    setPrevScope(currentScope);
    setQuery("");
  }

  const filteredEntries = useMemo(() => {
    if (!entries) return [];
    if (!tag) return entries;
    return entries.filter((entry) => entryHasTag(entry, tag));
  }, [entries, tag]);

  const results = useMemo(
    () => searchEntries(filteredEntries, debouncedQuery),
    [filteredEntries, debouncedQuery]
  );

  const isSearchActive = query.trim().length > 0;

  const clearSearch = useCallback(() => setQuery(""), []);

  return { query, setQuery, results, isSearchActive, clearSearch };
}
