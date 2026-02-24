# Research: Global Search Implementation — Codebase Analysis

## Current State of Search

### SearchForm Stub (`src/components/search-form.tsx`)

A purely visual stub — renders a `SidebarInput` with a `Search` icon but has **zero state, zero handlers, zero filtering
logic**. It's placed in two locations:

- Desktop: `src/components/layout/drag-region.tsx` line 182 (left panel, between header and EntryList)
- Mobile: `src/views/MobileContentArea.tsx` line 44 (bottom sticky bar)

### No Backend Search

No `search_entries` or `filter_entries` Tauri command exists. The only listing command is
`list_entries(db_id, group_id?)`:

- `group_id = None` → `collect_all_entries()` — recursively walks entire tree, returns every entry
- `group_id = Some(id)` → direct children of that group only (non-recursive)

The full `Entry` struct is returned (not `EntryListItem`), including: title, username, url, notes, tags, customFields,
timestamps.

### No Search State

- Zustand store (`src/stores/database-tabs.ts`): holds `selectedGroupId`, `selectedEntryId`, `expandedGroupIds` — no
  search query
- URL search params (`DashboardSearchSchema` in `src/routes/dashboard/index.$dbId.tsx`): `groupId`, `sortBy`,
  `sortOrder`, `tag` — no `q` param
- No search-related query key in `src/lib/query-keys.ts`

### No Search Keyboard Shortcut

Existing shortcut infrastructure: `useCreateEntryShortcut` (`src/hooks/use-create-entry-shortcut.ts`) registers
`Ctrl/Cmd+N` via `window.addEventListener("keydown", ...)`. This is the exact pattern to replicate for search shortcuts.

---

## Data Model Analysis

### Entry Type (`src/lib/types.ts`)

```typescript
{
  id: string; groupId: string; title: string; username: string;
  url: string | null | undefined; notes: string | null | undefined;
  iconId?: number; customIconUuid?: string | null;
  tags: string[]; customFields: Record<string, string>;
  customFieldMeta: CustomFieldMeta[];
  createdAt: string; modifiedAt: string; accessedAt: string;
}
```

**Searchable fields**: title, username, url, notes, tags. Notes that `customFields` contains only non-protected values —
protected fields are excluded (security pattern).

### Group Type (`src/lib/types.ts`)

```typescript
{
  id: string; parentId: string | null; name: string;
  icon: string | null; customIconUuid: string | null;
  children: Group[];  // recursive tree
}
```

Groups form a recursive tree. `list_groups(dbId)` returns one root with all children nested. The `findGroupById` utility
exists in `src/hooks/use-entry-list-header.ts` line 11 but only returns the group, not the full path.

### Tag Handling (`src/lib/tag-utils.ts`)

Tags can be semicolon or comma-separated. `getNormalizedEntryTags(entry)` splits `entry.tags` and also checks
`customFields["Tags"]`/`customFields["tags"]`. Search should use normalized tags.

---

## Frontend Architecture

### Layout Structure

```
__root__ → App (ThemeProvider + Toaster)
  DatabaseTabBar (root-level, shown for 2+ tabs)
  /dashboard/route → SidebarProvider
    AppSidebar (groups, tags, nav)
    SidebarInset → /dashboard/index/$dbId
      useIsMobile() →
        DesktopContentArea → DragRegion (two resizable panels)
        MobileContentArea (single column)
```

### Desktop Layout (`DragRegion` — `src/components/layout/drag-region.tsx`)

Two-panel `ResizablePanelGroup`:

- **Left panel**: Header (SidebarTrigger + group name + tag badge) → SearchForm + SortDropdown → EntryList
- **Right panel**: EntryActions toolbar → EntryItemDetails | EntryEditForm | EntryItemDetailsEmpty

State machine: `editMode: "view" | "edit" | "create"` with unsaved-change guard via `ask()` dialog.

The search integration point is the left panel body (line 186-188) where `EntryList` renders. When search is active,
`SearchResultsList` replaces `EntryList`.

### Mobile Layout (`MobileContentArea` — `src/views/MobileContentArea.tsx`)

Single column: NavEntries header → EntryList → sticky bottom bar (Plus + SearchForm + SortDropdown).

### Entry List (`src/components/entries/EntryList.tsx`)

- Fetches entries via `useEntries(dbId, search.groupId)`
- Sorts via `useSortedEntries(entries, sortBy, sortOrder)`
- Filters by tag via `entryHasTag()`
- Virtualized via `@tanstack/react-virtual` (`useVirtualizer`, estimated 65px rows, overscan 10)
- Wrapped in `ScrollArea` with `viewportRef` for the virtualizer's scroll element
- Keyboard navigation via `useEntryListKeyboard`

### Entry List Item (`src/components/entries/EntryListItem.tsx`)

- `memo`-ized functional component
- Uses Item primitives from `src/components/ui/item.tsx`: `Item`, `ItemMedia`, `ItemContent`, `ItemTitle`,
  `ItemDescription`, `ItemActions`
- Shows KeePass icon (custom base64 or default) in Avatar, title, username
- Props extend `Entry` with `customIcons`, `isSelected`, `onClick`

### Query Keys (`src/lib/query-keys.ts`)

```typescript
entries.list(dbId, groupId)  // existing
entries.detail(dbId, id)     // existing
// No search-specific key needed — search is computed from cached entries.list(dbId, null)
```

### Hooks Pattern

- `useEntries(dbId, groupId)` — `useQuery` with `staleTime: 30_000`
- `useGroups(dbId)` — `useQuery` with `staleTime: 30_000`
- `useCustomIcons(dbId)` — `useQuery` for base64 icon map
- `useSortedEntries(entries, sortBy, sortOrder)` — pure `useMemo` sort
- `useEntryListKeyboard(...)` — Arrow/Home/End/Enter key handler returning `{ onKeyDown }`
- `useCreateEntryShortcut(callback, enabled)` — global keydown listener in `useEffect`

---

## Design Decisions

### Client-side filtering (no backend search)

`list_entries(dbId, null)` already loads all entries into react-query cache. For typical KeePass databases (hundreds to
low thousands of entries), client-side filtering is instant (<1ms). A backend `search_entries` command would add
complexity with negligible benefit.

### Local state over URL params

Search is ephemeral — refreshing should not preserve a half-typed query. Using component-local state (lifted to
`DragRegion`/`MobileContentArea`) is simpler than adding a `q` URL param and avoids polluting the URL.

### Replacing EntryList vs overlaying

When search is active, `SearchResultsList` replaces `EntryList` in the same DOM slot. This avoids z-index/overlay
complexity and feels natural — the list area shows either group entries or search results.

### Group path building

A `buildGroupPathMap(groups)` utility traverses the tree once and returns `Map<groupId, pathString>`. This is `O(n)`
where n = number of groups, computed once per render via `useMemo`. Individual `SearchResultItem` components look up
their path in O(1).

### Reusing existing virtualizer pattern

`SearchResultsList` follows the exact same `useVirtualizer` + `ScrollArea` + absolute positioning pattern as
`EntryList.tsx`. Same estimated height, same overscan, same `measureElement` callback.

---

## Testing Strategy

### Priority 1 (must have, >70% coverage):

- `src/lib/__tests__/search-utils.test.ts` — Pure function tests for `searchEntries`, `buildGroupPathMap`,
  `highlightMatches`. Target 100%.
- `src/hooks/__tests__/use-debounce.test.ts` — Timer-based tests.
- `src/components/__tests__/search-form.test.tsx` — Controlled input behavior, clear button, Escape key.

### Priority 2 (important):

- `src/hooks/__tests__/use-search-entries.test.ts` — Hook integration with mocked useEntries.
- `src/components/search/__tests__/SearchResultItem.test.tsx` — Rendering, highlighting, click.
- `src/components/search/__tests__/SearchResultsList.test.tsx` — Empty state, results, selection.

### Testing patterns from codebase:

- vitest + @testing-library/react
- No shared test-utils wrapper — tests create their own `QueryClientProvider` inline
- Mock `@tauri-apps/plugin-dialog` at module level
- `fireEvent.change` works for controlled inputs but NOT for RHF dirty state detection
- Mock `PasswordStrengthIndicator` to avoid zxcvbn dictionary loading

---

## Performance Considerations

- **200ms debounce** prevents filtering on every keystroke
- **`useMemo`** on search results recomputes only when entries or debounced query change
- **`useEntries(dbId, null)`** reuses react-query cache (`staleTime: 30_000`) — no extra network call
- **Virtual scrolling** handles large result sets efficiently
- **`memo` on `SearchResultItem`** prevents re-renders of unchanged rows
- **Group path map** computed once per group tree change, O(1) per lookup
- **Linear scan** of entries for search is adequate — sub-1ms for <10k entries
