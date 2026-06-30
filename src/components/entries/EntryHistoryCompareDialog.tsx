// SPDX-License-Identifier: MIT

import { useCallback, useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import type { TFunction } from "i18next";
import { useQuery } from "@tanstack/react-query";
import { ArrowRight, Eye, EyeOff, Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { entries as entriesApi } from "@/lib/tauri";
import { queryKeys } from "@/lib/query-keys";
import type { Entry, EntryHistoryItem } from "@/lib/types";

/**
 * The built-in field tokens the backend emits in `changedFields` (#324). Any
 * token outside this set is a user-defined custom field key.
 */
const KNOWN_FIELDS = new Set([
  "title",
  "username",
  "password",
  "url",
  "notes",
  "tags",
  "icon",
  "attachments",
  "expiry",
  "location",
]);

/**
 * The set of field tokens that differ between the version at `index` and the
 * current Entry, derived from the names-only `changedFields` signal (#324)
 * without any new backend endpoint. Each version's `changedFields` is the diff
 * against the next-newer version (the newest against the live Entry), so the
 * union from index 0 through `index` is every field that changed somewhere
 * between that version and now. It's an upper bound — a field changed and later
 * reverted still appears — which the dialog's exact value comparison corrects
 * for the text fields it can compare directly.
 */
export function changedSince(
  versions: EntryHistoryItem[],
  index: number
): string[] {
  const seen = new Set<string>();
  for (const version of versions.slice(0, index + 1)) {
    for (const field of version.changedFields) seen.add(field);
  }
  return [...seen];
}

interface EntryHistoryCompareDialogProps {
  dbId: string;
  entryId: string;
  version: EntryHistoryItem;
  /**
   * The set of field tokens that differ between this version and the current
   * Entry — the union of `changedFields` from the newest version through this
   * one. Drives which rows the comparison shows (changed-only).
   */
  changedFields: string[];
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

/**
 * Compares a historical version against the current Entry, field by field, so
 * the user can see exactly what differs before deciding to restore (#329).
 *
 * This is a pure frontend composition (ADR-0008): no backend diff endpoint
 * beyond the existing history listing and the per-version secret fetches. Only
 * the current Entry's values, the version's non-secret listing fields, and
 * on-demand secret reveals are available — so plain text fields the listing
 * carries (title/username/url) show a real `historical → current` diff while
 * fields whose historical value can't be reached show the current value with a
 * "previous value not available" note.
 */
export function EntryHistoryCompareDialog({
  dbId,
  entryId,
  version,
  changedFields,
  open,
  onOpenChange,
}: Readonly<EntryHistoryCompareDialogProps>) {
  const { t } = useTranslation();

  const { data: current } = useQuery({
    queryKey: queryKeys.entries.detail(dbId, entryId),
    queryFn: () => entriesApi.get(dbId, entryId),
    enabled: open,
  });

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-lg max-h-[80vh] overflow-auto">
        <DialogHeader>
          <DialogTitle>{t("entries.detail.compare.title")}</DialogTitle>
          <DialogDescription>
            {t("entries.detail.compare.description", {
              date: formatHistoryDate(version.modifiedAt),
            })}
          </DialogDescription>
        </DialogHeader>
        {current && (
          <CompareBody
            dbId={dbId}
            entryId={entryId}
            version={version}
            current={current}
            changedFields={changedFields}
          />
        )}
      </DialogContent>
    </Dialog>
  );
}

/**
 * The comparison rows, rendered once the current Entry has loaded. A field is
 * shown only when it actually differs (changed-only). Direct text fields the
 * history listing carries (title/username/url) are compared by exact value —
 * which also corrects the changed-field signal's false positives when a field
 * was changed and later reverted to its current value.
 */
function CompareBody({
  dbId,
  entryId,
  version,
  current,
  changedFields,
}: Readonly<{
  dbId: string;
  entryId: string;
  version: EntryHistoryItem;
  current: Entry;
  changedFields: string[];
}>) {
  const { t } = useTranslation();

  const rows: ReactNode[] = [];

  // The backend flattens built-in tokens (`password`, `tags`, …) and custom
  // field keys into one list, and a custom field may be named exactly like a
  // built-in token — the wire can't tell them apart. So on a collision we can't
  // pick a side without risking hiding a real change; instead the built-in rows
  // below render on token presence, and any token that is *also* a known custom
  // key (current meta or the version's protected set) additionally gets its own
  // custom row. Both possibilities are shown rather than silently dropping one.
  const customMetaNow = new Map(
    current.customFieldMeta.map((m) => [m.key, m.isProtected])
  );
  const customKeys = new Set([
    ...customMetaNow.keys(),
    ...version.protectedFields,
  ]);

  // Direct text fields the history listing carries for both sides: compared by
  // exact value, so a changed-then-reverted field drops out of the diff.
  for (const field of ["title", "username", "url"] as const) {
    const before = version[field] ?? "";
    const after = current[field] ?? "";
    if (changedFields.includes(field) && before !== after) {
      rows.push(
        <CompareRow
          key={field}
          label={t(`entries.detail.historyField.${field}`)}
          before={before}
          after={after}
        />
      );
    }
  }

  // Password: a secret on both sides. Revealed only on the explicit toggle,
  // which fetches the current and historical values together (ADR-0008) — the
  // historical fetch carries the version's index + fingerprint guard.
  if (changedFields.includes("password")) {
    rows.push(
      <CompareSecretRow
        key="password"
        label={t("entries.detail.historyField.password")}
        revealLabel={t("entries.detail.revealPassword")}
        hideLabel={t("entries.detail.hidePassword")}
        fetchBefore={() =>
          entriesApi.getHistoryPassword(
            dbId,
            entryId,
            version.index,
            version.fingerprint
          )
        }
        fetchAfter={() => entriesApi.getPassword(dbId, entryId)}
      />
    );
  }

  // Attachments: the listing carries the version's filenames, so we name what
  // changed instead of the generic "previous not available" row (#356). The
  // helper returns null when nothing net-differs, so the empty-state check below
  // still fires and the changed-field signal's false positives are corrected.
  const attachmentRow = changedFields.includes("attachments")
    ? attachmentCompareRow(version, current, t)
    : null;
  if (attachmentRow) rows.push(attachmentRow);

  // Value-less fields: the listing carries no historical value for these, so we
  // can only show the current value (where it's plain text) and note that the
  // previous value can't be displayed.
  for (const field of [
    "notes",
    "tags",
    "icon",
    "expiry",
    "location",
  ] as const) {
    if (changedFields.includes(field)) {
      rows.push(
        <PreviousUnavailableRow
          key={field}
          label={t(`entries.detail.historyField.${field}`)}
          current={currentSummary(field, current)}
        />
      );
    }
  }

  // Custom fields: a token gets a custom row when it's a known custom key (so a
  // collision with a built-in token still surfaces the custom field alongside
  // the built-in row above) or when it isn't a built-in token at all (e.g. a
  // since-deleted field). A field is a two-sided secret only when it's
  // protected in *both* versions — then it's revealed on demand from each side.
  // When protection differs (toggled, or the field was added/removed), one
  // protected endpoint would error, so we degrade to showing the current plain
  // value (if any) with "previous not available".
  for (const key of changedFields) {
    if (!customKeys.has(key) && KNOWN_FIELDS.has(key)) continue;
    rows.push(
      <CustomFieldCompareRow
        key={`custom:${key}`}
        dbId={dbId}
        entryId={entryId}
        version={version}
        current={current}
        fieldKey={key}
        protectedNow={customMetaNow.get(key) === true}
      />
    );
  }

  if (rows.length === 0) {
    return (
      <p className="text-sm text-muted-foreground">
        {t("entries.detail.compare.noChanges")}
      </p>
    );
  }

  return <>{rows}</>;
}

/**
 * One custom field's comparison row. It's a two-sided secret only when the
 * field is protected in *both* versions — then each side is revealed on demand.
 * When protection differs (toggled, or the field added/removed), one protected
 * endpoint would error, so it degrades to the current plain value (if any) with
 * "previous value not available".
 */
function CustomFieldCompareRow({
  dbId,
  entryId,
  version,
  current,
  fieldKey,
  protectedNow,
}: Readonly<{
  dbId: string;
  entryId: string;
  version: EntryHistoryItem;
  current: Entry;
  fieldKey: string;
  protectedNow: boolean;
}>) {
  const { t } = useTranslation();
  const protectedBoth =
    protectedNow && version.protectedFields.includes(fieldKey);

  if (protectedBoth) {
    return (
      <CompareSecretRow
        label={fieldKey}
        revealLabel={t("entries.detail.revealField", { field: fieldKey })}
        hideLabel={t("entries.detail.hideField", { field: fieldKey })}
        fetchBefore={async () =>
          (
            await entriesApi.getHistoryProtectedField(
              dbId,
              entryId,
              version.index,
              version.fingerprint,
              fieldKey
            )
          ).value
        }
        fetchAfter={async () =>
          (await entriesApi.getProtectedCustomField(dbId, entryId, fieldKey))
            .value
        }
      />
    );
  }

  return (
    <PreviousUnavailableRow
      label={fieldKey}
      current={current.customFields[fieldKey] ?? null}
    />
  );
}

/**
 * A secret field compared across both versions (the password or a protected
 * custom field). Nothing is fetched until the explicit reveal, which loads the
 * historical and current values together and shows them as `before → after`.
 * Hiding drops both values from state so neither lingers (ADR-0008).
 */
function CompareSecretRow({
  label,
  revealLabel,
  hideLabel,
  fetchBefore,
  fetchAfter,
}: Readonly<{
  label: string;
  revealLabel: string;
  hideLabel: string;
  fetchBefore: () => Promise<string>;
  fetchAfter: () => Promise<string>;
}>) {
  const [values, setValues] = useState<{
    before: string;
    after: string;
  } | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const isVisible = values !== null;

  const reveal = useCallback(async () => {
    setIsLoading(true);
    try {
      const [before, after] = await Promise.all([fetchBefore(), fetchAfter()]);
      setValues({ before, after });
    } finally {
      setIsLoading(false);
    }
  }, [fetchBefore, fetchAfter]);

  const hide = useCallback(() => setValues(null), []);

  return (
    <div className="flex flex-col gap-1 py-1">
      <small className="text-xs font-medium text-muted-foreground">
        {label}
      </small>
      <div className="flex min-w-0 items-center gap-2 text-sm">
        {isLoading ? (
          <Loader2 className="h-3 w-3 animate-spin text-muted-foreground" />
        ) : (
          <>
            <span className="min-w-0 flex-1 truncate text-muted-foreground line-through">
              {isVisible ? values.before : "••••••••"}
            </span>
            <ArrowRight className="h-3 w-3 shrink-0 text-muted-foreground" />
            <span className="min-w-0 flex-1 truncate">
              {isVisible ? values.after : "••••••••"}
            </span>
          </>
        )}
        <Button
          variant="outline"
          size="icon-xs"
          className="shrink-0"
          aria-label={isVisible ? hideLabel : revealLabel}
          onClick={isVisible ? hide : reveal}
          disabled={isLoading}
        >
          {isVisible ? (
            <EyeOff className="h-3 w-3" />
          ) : (
            <Eye className="h-3 w-3" />
          )}
        </Button>
      </div>
    </div>
  );
}

/**
 * A plain-text summary of the current Entry's value for a value-less field, or
 * `null` when the field has no meaningful one-line representation (icon,
 * attachments, expiry, location are shown by label alone).
 */
function currentSummary(
  field: "notes" | "tags" | "icon" | "expiry" | "location",
  current: Entry
): string | null {
  if (field === "notes") return current.notes ?? "";
  if (field === "tags") return current.tags.join(", ");
  return null;
}

/**
 * A field that changed but whose historical value can't be reached over IPC.
 * Shows the current value (when it's plain text) plus a note that the previous
 * value isn't available for comparison.
 */
function PreviousUnavailableRow({
  label,
  current,
}: Readonly<{ label: string; current: string | null }>) {
  const { t } = useTranslation();
  return (
    <div className="flex flex-col gap-1 py-1">
      <small className="text-xs font-medium text-muted-foreground">
        {label}
      </small>
      {current !== null && current !== "" && (
        <span className="min-w-0 truncate text-sm">{current}</span>
      )}
      <span className="text-xs italic text-muted-foreground">
        {t("entries.detail.compare.previousNotAvailable")}
      </span>
    </div>
  );
}

/**
 * Diffs the version's attachment filenames against the current Entry's and,
 * when they net-differ, returns the row naming what was added/removed (#356). A
 * rename surfaces as one removed + one added, matching the backend's
 * filename-set comparison. Returns null when nothing differs so the caller can
 * skip an empty row — names only, never bytes (ADR-0008).
 */
function attachmentCompareRow(
  version: EntryHistoryItem,
  current: Entry,
  t: TFunction
): ReactNode {
  const currentNames = new Set(current.attachments.map((a) => a.filename));
  const versionNames = new Set(version.attachmentNames);
  const added = current.attachments
    .map((a) => a.filename)
    .filter((name) => !versionNames.has(name));
  const removed = version.attachmentNames.filter(
    (name) => !currentNames.has(name)
  );
  if (added.length === 0 && removed.length === 0) return null;
  return (
    <AttachmentCompareRow
      key="attachments"
      label={t("entries.detail.historyField.attachments")}
      added={added}
      removed={removed}
    />
  );
}

/**
 * Names the attachments added and removed between a version and the current
 * Entry (#356). Names only — no size, MIME, preview, or bytes ever cross IPC
 * (ADR-0008). Removed names are struck through (gone now), added names plain
 * (present now); each is prefixed with a localized Added/Removed label.
 */
function AttachmentCompareRow({
  label,
  added,
  removed,
}: Readonly<{ label: string; added: string[]; removed: string[] }>) {
  const { t } = useTranslation();
  return (
    <div className="flex flex-col gap-1 py-1">
      <small className="text-xs font-medium text-muted-foreground">
        {label}
      </small>
      {removed.map((name) => (
        <div
          key={`removed:${name}`}
          className="flex min-w-0 items-center gap-2 text-sm"
        >
          <span className="shrink-0 text-xs font-medium text-muted-foreground">
            {t("entries.detail.compare.attachmentRemoved")}
          </span>
          <span className="min-w-0 truncate text-muted-foreground line-through">
            {name}
          </span>
        </div>
      ))}
      {added.map((name) => (
        <div
          key={`added:${name}`}
          className="flex min-w-0 items-center gap-2 text-sm"
        >
          <span className="shrink-0 text-xs font-medium text-muted-foreground">
            {t("entries.detail.compare.attachmentAdded")}
          </span>
          <span className="min-w-0 truncate">{name}</span>
        </div>
      ))}
    </div>
  );
}

// Format a version's timestamp the same way the history list does, so the
// compare dialog names the version by a date the user already recognizes.
function formatHistoryDate(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat(undefined, {
    year: "numeric",
    month: "short",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit",
    second: "2-digit",
  }).format(date);
}

/**
 * One field's `before → after` comparison row.
 */
function CompareRow({
  label,
  before,
  after,
}: Readonly<{ label: string; before: string; after: string }>) {
  return (
    <div className="flex flex-col gap-1 py-1">
      <small className="text-xs font-medium text-muted-foreground">
        {label}
      </small>
      <div className="flex min-w-0 items-center gap-2 text-sm">
        <span className="min-w-0 flex-1 truncate text-muted-foreground line-through">
          {before}
        </span>
        <ArrowRight className="h-3 w-3 shrink-0 text-muted-foreground" />
        <span className="min-w-0 flex-1 truncate">{after}</span>
      </div>
    </div>
  );
}
