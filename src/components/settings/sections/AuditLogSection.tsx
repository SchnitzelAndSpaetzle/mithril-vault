// SPDX-License-Identifier: MIT

import { useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useVirtualizer } from "@tanstack/react-virtual";
import { ask } from "@tauri-apps/plugin-dialog";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { SettingsSection } from "@/components/settings/SettingsSection";
import { queryKeys } from "@/lib/query-keys";
import { audit, entries as entriesIpc, settings } from "@/lib/tauri";
import type {
  AuditEvent,
  AuditEventKind,
  AuditEventsResponse,
  AuditStatus,
  Entry,
  RecentDatabase,
} from "@/lib/types";
import { AuditEventKindSchema } from "@/lib/types";
import { getFilenameFromPath } from "@/lib/utils";

interface AuditLogSectionProps {
  dbId: string | null;
  isLocked?: boolean;
}

function formatTimestamp(timestamp: string, locale: string): string {
  const parsed = new Date(timestamp);
  if (Number.isNaN(parsed.getTime())) return timestamp;
  return parsed.toLocaleString(locale, {
    dateStyle: "medium",
    timeStyle: "medium",
  });
}

// Subscribes to the entries-list query for `dbId` so the resolver
// recomputes whenever entries load or change. While the Vault is locked
// the query stays disabled (PRD US #16: locked vaults must mask entry
// rows to a UUID prefix so the on-disk log never carries titles outside
// the unlocked Vault), and the resolver short-circuits to null.
function useResolveEntryTitle(
  dbId: string | null,
  isLocked: boolean
): (entryId: string) => string | null {
  const query = useQuery<Entry[], Error>({
    queryKey: queryKeys.entries.list(dbId ?? "none", null),
    queryFn: () => (dbId ? entriesIpc.list(dbId) : Promise.resolve([])),
    enabled: Boolean(dbId) && !isLocked,
    staleTime: 30_000,
  });
  const entriesList = query.data;
  return (entryId: string) => {
    if (isLocked || !entriesList) return null;
    return entriesList.find((e) => e.id === entryId)?.title ?? null;
  };
}

const ALL_KINDS: readonly AuditEventKind[] = AuditEventKindSchema.options;

interface AuditRowProps {
  event: AuditEvent;
  resolveTitle: (entryId: string) => string | null;
  /// Inline style for absolute positioning when the row is rendered as
  /// part of a virtualizer; omitted in the static-list path so flat
  /// rendering doesn't need any positioning information.
  style?: React.CSSProperties;
  /// Forwarded to the rendered `<li>` so the virtualizer can place it
  /// alongside its other data attributes (`data-index`) when needed.
  liRef?: React.Ref<HTMLLIElement>;
}

function AuditRow({
  event,
  resolveTitle,
  style,
  liRef,
}: Readonly<AuditRowProps>) {
  const { t, i18n } = useTranslation();
  const entryId = event.entryId ?? null;
  const title = entryId ? resolveTitle(entryId) : null;
  const entryLabel = entryId ? (title ?? entryId.slice(0, 8)) : null;
  const settingName = event.settingName ?? null;
  return (
    <li
      ref={liRef}
      style={style}
      data-kind={event.kind}
      data-entry-id={entryId ?? undefined}
      data-setting-name={settingName ?? undefined}
      className="flex flex-col gap-1 rounded-md border bg-card/50 p-3 text-sm md:flex-row md:items-center md:justify-between"
    >
      <span className="font-medium">{t(`audit.kind.${event.kind}`)}</span>
      <div className="flex items-center gap-3 text-xs text-muted-foreground">
        {entryLabel ? <span>{entryLabel}</span> : null}
        {settingName ? (
          <span>{t(`audit.settingName.${settingName}`)}</span>
        ) : null}
        {typeof event.attemptCount === "number" ? (
          <span>{t("audit.attemptCount", { count: event.attemptCount })}</span>
        ) : null}
        {event.reason ? <span>{t(`audit.reason.${event.reason}`)}</span> : null}
        <span>{formatTimestamp(event.timestamp, i18n.language)}</span>
      </div>
    </li>
  );
}

const EMPTY_RESPONSE: AuditEventsResponse = { events: [], degraded: false };
const VIRTUALIZED_ROW_HEIGHT = 60;
// Above this count, switch to virtualized rendering. Below it, the
// flat list keeps testing simple and rendering cheap.
const VIRTUALIZATION_THRESHOLD = 200;

function parseDateStart(raw: string): Date | null {
  if (!raw) return null;
  const parsed = new Date(`${raw}T00:00:00.000`);
  return Number.isNaN(parsed.getTime()) ? null : parsed;
}

function parseDateEnd(raw: string): Date | null {
  if (!raw) return null;
  const parsed = new Date(`${raw}T23:59:59.999`);
  return Number.isNaN(parsed.getTime()) ? null : parsed;
}

interface FilterState {
  enabledKinds: ReadonlySet<AuditEventKind>;
  dateFrom: string;
  dateTo: string;
}

function applyFilters(
  events: AuditEvent[],
  filters: FilterState
): AuditEvent[] {
  const fromDate = parseDateStart(filters.dateFrom);
  const toDate = parseDateEnd(filters.dateTo);
  return events.filter((event) => {
    if (!filters.enabledKinds.has(event.kind)) return false;
    if (fromDate || toDate) {
      const ts = new Date(event.timestamp);
      if (Number.isNaN(ts.getTime())) return false;
      if (fromDate && ts < fromDate) return false;
      if (toDate && ts > toDate) return false;
    }
    return true;
  });
}

interface VaultPickerProps {
  recentDatabases: RecentDatabase[];
  selectedPath: string;
  onChange: (path: string) => void;
}

function VaultPicker({
  recentDatabases,
  selectedPath,
  onChange,
}: Readonly<VaultPickerProps>) {
  const { t } = useTranslation();
  // Native <select> avoids the headless-radix portal that complicates
  // role-based assertions and keyboard navigation in tests.
  return (
    <div className="flex flex-col gap-1.5">
      <Label htmlFor="audit-vault-picker">{t("audit.picker.label")}</Label>
      <select
        id="audit-vault-picker"
        className="flex h-9 w-full rounded-md border border-input bg-transparent px-3 py-1 text-sm shadow-xs focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
        value={selectedPath}
        onChange={(e) => onChange(e.target.value)}
      >
        {recentDatabases.map((db) => (
          <option key={db.path} value={db.path} data-vault-path={db.path}>
            {`${getFilenameFromPath(db.path, db.path)} — ${db.path}`}
          </option>
        ))}
      </select>
    </div>
  );
}

interface EventKindFilterProps {
  enabledKinds: ReadonlySet<AuditEventKind>;
  onToggle: (kind: AuditEventKind, enabled: boolean) => void;
}

function EventKindFilter({
  enabledKinds,
  onToggle,
}: Readonly<EventKindFilterProps>) {
  const { t } = useTranslation();
  return (
    <fieldset
      data-testid="audit-kind-filter"
      className="flex flex-col gap-2 rounded-md border bg-card/30 p-3"
    >
      <legend className="px-1 text-xs font-medium text-muted-foreground">
        {t("audit.filter.kinds")}
      </legend>
      <div className="flex flex-wrap gap-3">
        {ALL_KINDS.map((kind) => (
          <label
            key={kind}
            className="flex items-center gap-2 text-sm"
            data-kind-checkbox={kind}
          >
            <Checkbox
              checked={enabledKinds.has(kind)}
              onCheckedChange={(state) => onToggle(kind, state === true)}
              aria-label={t(`audit.kind.${kind}`)}
            />
            <span>{t(`audit.kind.${kind}`)}</span>
          </label>
        ))}
      </div>
    </fieldset>
  );
}

interface DateRangeFilterProps {
  dateFrom: string;
  dateTo: string;
  onFromChange: (value: string) => void;
  onToChange: (value: string) => void;
  invalidRange: boolean;
}

function DateRangeFilter({
  dateFrom,
  dateTo,
  onFromChange,
  onToChange,
  invalidRange,
}: Readonly<DateRangeFilterProps>) {
  const { t } = useTranslation();
  return (
    <div className="flex flex-col gap-2">
      <div className="flex flex-wrap items-end gap-3">
        <div className="flex flex-col gap-1.5">
          <Label htmlFor="audit-date-from">{t("audit.filter.from")}</Label>
          <Input
            id="audit-date-from"
            type="date"
            value={dateFrom}
            onChange={(e) => onFromChange(e.target.value)}
          />
        </div>
        <div className="flex flex-col gap-1.5">
          <Label htmlFor="audit-date-to">{t("audit.filter.to")}</Label>
          <Input
            id="audit-date-to"
            type="date"
            value={dateTo}
            onChange={(e) => onToChange(e.target.value)}
          />
        </div>
      </div>
      {invalidRange ? (
        <p role="alert" className="text-xs text-destructive">
          {t("audit.filter.invalidRange")}
        </p>
      ) : null}
    </div>
  );
}

interface VirtualizedRowsProps {
  events: AuditEvent[];
  resolveTitle: (entryId: string) => string | null;
}

function VirtualizedRows({
  events,
  resolveTitle,
}: Readonly<VirtualizedRowsProps>) {
  const scrollRef = useRef<HTMLDivElement | null>(null);
  // eslint-disable-next-line react-hooks/incompatible-library -- virtualizer is not passed to memoized components
  const virtualizer = useVirtualizer({
    count: events.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => VIRTUALIZED_ROW_HEIGHT,
    overscan: 10,
  });

  return (
    <div
      ref={scrollRef}
      className="max-h-[480px] overflow-auto"
      data-testid="audit-virtual-scroll"
    >
      <ul
        className="relative"
        style={{ height: `${virtualizer.getTotalSize()}px` }}
      >
        {virtualizer.getVirtualItems().map((virtualItem) => {
          const event = events[virtualItem.index];
          if (!event) return null;
          // The virtualizer-positioned element IS the list item — wrapping
          // it in a `<div>` would yield `<ul><div><li>` and break list
          // semantics for assistive tech.
          return (
            <AuditRow
              key={`${event.timestamp}-${virtualItem.index}`}
              event={event}
              resolveTitle={resolveTitle}
              style={{
                position: "absolute",
                left: 0,
                right: 0,
                transform: `translateY(${virtualItem.start}px)`,
              }}
            />
          );
        })}
      </ul>
    </div>
  );
}

export function AuditLogSection({
  dbId,
  isLocked = false,
}: Readonly<AuditLogSectionProps>) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();

  const recentDatabasesQuery = useQuery<RecentDatabase[], Error>({
    queryKey: queryKeys.settings.recentDatabases(),
    queryFn: () => settings.getRecentDatabases(),
    staleTime: 30_000,
  });
  const recentDatabases = recentDatabasesQuery.data ?? [];

  // The picker's *user-picked* path. `null` means "no explicit pick yet";
  // the effective Vault then falls back to the open `dbId`, then the
  // most-recent entry from `recent_databases`. Distinct from `dbId`
  // because the user may audit a Vault that is not unlocked in this
  // session (titles fall back to UUID prefixes in that case).
  const [pickedPath, setPickedPath] = useState<string | null>(null);

  const effectivePath = useMemo(() => {
    if (pickedPath) return pickedPath;
    if (dbId) return dbId;
    return recentDatabases[0]?.path ?? null;
  }, [pickedPath, dbId, recentDatabases]);

  const [enabledKinds, setEnabledKinds] = useState<ReadonlySet<AuditEventKind>>(
    () => new Set<AuditEventKind>(ALL_KINDS)
  );
  const [dateFrom, setDateFrom] = useState<string>("");
  const [dateTo, setDateTo] = useState<string>("");

  const fromDate = parseDateStart(dateFrom);
  const toDate = parseDateEnd(dateTo);
  const invalidRange = Boolean(fromDate && toDate && fromDate > toDate);

  // Title resolution must be tied to the currently-OPEN Vault, not the
  // picked one — the entries cache only contains the unlocked Vault.
  const titleResolutionDbId =
    effectivePath && effectivePath === dbId ? dbId : null;
  const resolveTitle = useResolveEntryTitle(titleResolutionDbId, isLocked);

  const query = useQuery<AuditEventsResponse, Error>({
    queryKey: queryKeys.audit.list(effectivePath ?? "none"),
    queryFn: () =>
      effectivePath
        ? audit.list(effectivePath)
        : Promise.resolve(EMPTY_RESPONSE),
    enabled: Boolean(effectivePath) && !invalidRange,
  });

  const statusQuery = useQuery<AuditStatus, Error>({
    queryKey: queryKeys.audit.status(),
    queryFn: () => audit.getStatus(),
    staleTime: Infinity,
  });

  const clearMutation = useMutation<void, Error, string>({
    mutationFn: (vaultPath) => audit.clear(vaultPath),
    onSuccess: (_data, vaultPath) => {
      void queryClient.invalidateQueries({
        queryKey: queryKeys.audit.list(vaultPath),
      });
    },
    onError: (error) => {
      toast.error(t("audit.clearError", { error: String(error) }));
    },
  });

  const data = query.data ?? EMPTY_RESPONSE;

  const filteredEvents = useMemo(
    () =>
      applyFilters(data.events, {
        enabledKinds,
        dateFrom,
        dateTo,
      }),
    [data.events, enabledKinds, dateFrom, dateTo]
  );

  async function handleClear() {
    if (!effectivePath) return;
    const confirmed = await ask(t("audit.clearConfirm"), {
      title: t("audit.clearConfirmTitle"),
      kind: "warning",
    });
    if (!confirmed) return;
    clearMutation.mutate(effectivePath);
  }

  function toggleKind(kind: AuditEventKind, enabled: boolean): void {
    setEnabledKinds((previous) => {
      const next = new Set(previous);
      if (enabled) next.add(kind);
      else next.delete(kind);
      return next;
    });
  }

  // Degraded indicator: comes from the session-wide `getStatus()` flag so
  // the header banner is visible regardless of which Vault is picked, and
  // so it persists when the per-Vault read happens to land clean. Clears
  // on app restart because the backend flag is in-memory. The response-
  // level `degraded` is the same backend flag and is OR'd in so a fresh
  // read can light up the banner without waiting for the status query.
  const sessionDegraded =
    (statusQuery.data?.degraded ?? false) || data.degraded;

  const hasPicker = recentDatabases.length > 0 && effectivePath !== null;
  const showEmptyNoVault = !effectivePath;

  function renderEventList() {
    if (showEmptyNoVault) {
      return (
        <p className="text-sm text-muted-foreground">
          {t("audit.emptyNoVault")}
        </p>
      );
    }
    if (invalidRange) return null;
    if (query.isLoading) {
      return (
        <p className="text-sm text-muted-foreground">{t("common.loading")}</p>
      );
    }
    if (query.error) {
      return (
        <p className="text-sm text-destructive">
          {t("audit.loadError", { error: String(query.error) })}
        </p>
      );
    }
    if (filteredEvents.length === 0) {
      return (
        <p className="text-sm text-muted-foreground">{t("audit.empty")}</p>
      );
    }
    if (filteredEvents.length >= VIRTUALIZATION_THRESHOLD) {
      return (
        <VirtualizedRows events={filteredEvents} resolveTitle={resolveTitle} />
      );
    }
    return (
      <ul className="flex flex-col gap-2">
        {filteredEvents.map((event, index) => (
          <AuditRow
            key={`${event.timestamp}-${index}`}
            event={event}
            resolveTitle={resolveTitle}
          />
        ))}
      </ul>
    );
  }

  return (
    <SettingsSection
      id="audit-log"
      title={t("audit.title")}
      description={t("audit.description")}
    >
      {sessionDegraded ? (
        <p
          role="status"
          className="rounded-md border border-amber-500/40 bg-amber-500/10 p-3 text-sm text-amber-900 dark:text-amber-200"
        >
          {t("audit.degradedWarning")}
        </p>
      ) : null}
      {hasPicker ? (
        <VaultPicker
          recentDatabases={recentDatabases}
          selectedPath={effectivePath ?? ""}
          onChange={setPickedPath}
        />
      ) : null}
      {effectivePath ? (
        <div className="flex justify-end">
          <Button
            type="button"
            variant="destructive"
            size="sm"
            onClick={() => {
              void handleClear();
            }}
            disabled={clearMutation.isPending}
          >
            {t("audit.clearButton")}
          </Button>
        </div>
      ) : null}
      {effectivePath ? (
        <>
          <EventKindFilter enabledKinds={enabledKinds} onToggle={toggleKind} />
          <DateRangeFilter
            dateFrom={dateFrom}
            dateTo={dateTo}
            onFromChange={setDateFrom}
            onToChange={setDateTo}
            invalidRange={invalidRange}
          />
        </>
      ) : null}
      {renderEventList()}
    </SettingsSection>
  );
}
