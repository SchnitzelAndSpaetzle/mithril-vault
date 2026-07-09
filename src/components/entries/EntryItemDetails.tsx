import { createElement, useCallback, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import dayjs from "dayjs";
import { Separator } from "@/components/ui/separator.tsx";
import {
  Check,
  Copy,
  Download,
  ExternalLink,
  Eye,
  EyeOff,
  File,
  FileText,
  Image,
  Keyboard,
  Loader2,
  Paperclip,
  Trash2,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import {
  Avatar,
  AvatarFallback,
  AvatarImage,
} from "@/components/ui/avatar.tsx";
import { Skeleton } from "@/components/ui/skeleton";
import { useEntryDetail } from "@/hooks/use-entry-detail";
import { useCopyToClipboard } from "@/hooks/use-copy-to-clipboard";
import { useCustomIcons } from "@/hooks/use-custom-icons";
import { useClipboardCountdown } from "@/hooks/use-clipboard-countdown";
import { useClipboardTimeout } from "@/hooks/use-clipboard-timeout";
import { useIsMobile } from "@/hooks/use-mobile";
import { useAttachmentDrop } from "@/hooks/use-attachment-drop";
import { useQueryClient } from "@tanstack/react-query";
import { clipboard, entries as entriesApi } from "@/lib/tauri";
import { queryKeys } from "@/lib/query-keys";
import { saveWithErrorToast } from "@/lib/save-with-error-toast";
import { getKeepassIcon } from "@/lib/keepass-icons";
import { isExpired } from "@/lib/entry-expiry";
import { formatAttachmentSize } from "@/lib/entry-attachment";
import { classifyAttachment } from "@/lib/attachment-preview";
import { AttachmentPreviewModal } from "@/components/entries/AttachmentPreviewModal";
import { EntryHistorySection } from "@/components/entries/EntryHistorySection";
import { cn } from "@/lib/utils";
import { openUrl } from "@tauri-apps/plugin-opener";
import { ask, save } from "@tauri-apps/plugin-dialog";
import { toast } from "sonner";
import type { AttachmentMeta, CustomFieldMeta } from "@/lib/types";

interface EntryItemDetailsProps {
  entryId: string;
  dbId: string;
}

export default function EntryItemDetails({
  entryId,
  dbId,
}: Readonly<EntryItemDetailsProps>) {
  const { t } = useTranslation();
  const {
    entry,
    isLoading,
    password,
    isPasswordVisible,
    isPasswordLoading,
    isTransitioning,
    revealPassword,
    hidePassword,
  } = useEntryDetail(entryId, dbId);

  const { data: customIcons } = useCustomIcons(dbId);

  if (isLoading || !entry) {
    return <EntryDetailSkeleton />;
  }

  const iconComponent = getKeepassIcon(entry.iconId ?? 0);
  const customIcon = entry.customIconUuid
    ? customIcons?.[entry.customIconUuid]
    : null;
  const customIconSrc = customIcon
    ? `data:${customIcon.mimeType};base64,${customIcon.data}`
    : undefined;

  const expired = isExpired(entry, new Date());

  return (
    <>
      {/* Title section */}
      <div className="flex items-center gap-4 px-4">
        <Avatar>
          <AvatarImage src={customIconSrc} alt="" />
          <AvatarFallback>
            {createElement(iconComponent, { className: "h-4 w-4" })}
          </AvatarFallback>
        </Avatar>
        <h4
          className={cn(
            "scroll-m-20 text-xl font-semibold tracking-tight",
            expired && "line-through text-muted-foreground"
          )}
        >
          {entry.title}
        </h4>
        {expired && (
          <Badge variant="destructive">{t("entries.detail.expired")}</Badge>
        )}
      </div>

      {/* Main fields */}
      <div className="border rounded-md">
        {entry.username && (
          <>
            <EntryFieldRow
              label={t("entries.detail.userName")}
              value={entry.username}
              isDisabled={isTransitioning}
            />
            <Separator />
          </>
        )}

        {/* Password row */}
        <PasswordRow
          dbId={dbId}
          entryId={entryId}
          password={password}
          isVisible={isPasswordVisible}
          isLoading={isPasswordLoading}
          isDisabled={isTransitioning}
          onReveal={revealPassword}
          onHide={hidePassword}
        />

        {entry.url && (
          <>
            <Separator />
            <UrlRow url={entry.url} isDisabled={isTransitioning} />
          </>
        )}

        {entry.tags.length > 0 && (
          <>
            <Separator />
            <div className="flex justify-between items-center px-4 py-2">
              <small className="text-sm font-medium">
                {t("entries.detail.tags")}
              </small>
              <div className="flex flex-wrap gap-1">
                {entry.tags.map((tag) => (
                  <Badge key={tag} variant="outline">
                    {tag}
                  </Badge>
                ))}
              </div>
            </div>
          </>
        )}

        {entry.notes && (
          <>
            <Separator />
            <div className="px-4 py-2">
              <p className="whitespace-pre-wrap text-sm font-medium text-muted-foreground">
                {entry.notes}
              </p>
            </div>
          </>
        )}
      </div>

      {/* Custom fields */}
      {entry.customFieldMeta.length > 0 && (
        <div className="border rounded-md">
          {entry.customFieldMeta.map((meta, index) => (
            <div key={`${entry.id}:${meta.key}`}>
              {index > 0 && <Separator />}
              {meta.isProtected ? (
                <ProtectedCustomFieldRow
                  dbId={dbId}
                  entryId={entryId}
                  meta={meta}
                  isDisabled={isTransitioning}
                />
              ) : (
                <EntryFieldRow
                  label={meta.key}
                  value={entry.customFields[meta.key] ?? ""}
                  isDisabled={isTransitioning}
                />
              )}
            </div>
          ))}
        </div>
      )}

      {/* Attachments */}
      <AttachmentsSection
        attachments={entry.attachments}
        dbId={dbId}
        entryId={entryId}
        isDisabled={isTransitioning}
      />

      {/* Metadata */}
      <div className="border rounded-md">
        <EntryFieldBasic
          label={t("entries.detail.created")}
          value={formatDate(entry.createdAt)}
        />
        <Separator />
        <EntryFieldBasic
          label={t("entries.detail.modified")}
          value={formatDate(entry.modifiedAt)}
        />
        {entry.expires && entry.expiryTime && (
          <>
            <Separator />
            <EntryFieldBasic
              label={t("entries.detail.expires")}
              value={formatExpiry(entry.expiryTime)}
            />
          </>
        )}
      </div>

      {/* Entry History. Keyed on the *displayed* entry's id (not the entryId
          prop): useEntryDetail serves the previous entry as placeholder data
          while switching, so binding to entry.id keeps the history list
          consistent with the metadata shown above it during a transition. */}
      <EntryHistorySection dbId={dbId} entryId={entry.id} />
    </>
  );
}

function EntryDetailSkeleton() {
  return (
    <>
      <div className="flex items-center gap-4 px-4">
        <Skeleton className="h-10 w-10 rounded-full" />
        <Skeleton className="h-6 w-48" />
      </div>
      <div className="border rounded-md">
        <div className="flex justify-between items-center px-4 py-2">
          <Skeleton className="h-4 w-20" />
          <Skeleton className="h-4 w-40" />
        </div>
        <Separator />
        <div className="flex justify-between items-center px-4 py-2">
          <Skeleton className="h-4 w-20" />
          <Skeleton className="h-4 w-32" />
        </div>
        <Separator />
        <div className="flex justify-between items-center px-4 py-2">
          <Skeleton className="h-4 w-20" />
          <Skeleton className="h-4 w-56" />
        </div>
      </div>
    </>
  );
}

function PasswordRow({
  dbId,
  entryId,
  password,
  isVisible,
  isLoading,
  isDisabled,
  onReveal,
  onHide,
}: Readonly<{
  dbId: string;
  entryId: string;
  password: string | null;
  isVisible: boolean;
  isLoading: boolean;
  isDisabled: boolean;
  onReveal: () => void;
  onHide: () => void;
}>) {
  const { t } = useTranslation();
  const clipboardClearTimeout = useClipboardTimeout();
  const startCountdown = useClipboardCountdown();
  const [isCopied, setIsCopied] = useState(false);

  const handleCopy = async () => {
    if (isDisabled) return;
    try {
      await clipboard.copyPassword(dbId, entryId, clipboardClearTimeout);
      setIsCopied(true);
      toast.success(t("shortcuts.toast.passwordCopied"));
      startCountdown(clipboardClearTimeout);
      setTimeout(() => setIsCopied(false), 2000);
    } catch (error) {
      console.error("Failed to copy password:", error);
    }
  };

  let displayValue: React.ReactNode;
  if (isLoading) {
    displayValue = <Loader2 className="inline h-3 w-3 animate-spin" />;
  } else if (isVisible) {
    displayValue = password;
  } else {
    displayValue = "••••••••";
  }

  return (
    <div className="flex min-w-0 justify-between items-center px-4 py-2 gap-2">
      <small className="shrink-0 text-sm font-medium">
        {t("entries.detail.password")}
      </small>
      <div className="flex w-0 min-w-0 flex-1 items-center justify-end-safe gap-2">
        <button
          onClick={handleCopy}
          disabled={isDisabled}
          className="group flex min-w-0 max-w-full flex-1 items-center justify-end gap-2 overflow-hidden rounded-sm px-2 py-1 text-sm font-medium text-muted-foreground transition-all duration-200 hover:bg-accent"
        >
          <span className="min-w-0 truncate text-right transition-all duration-200">
            {isCopied ? t("common.copied") : displayValue}
          </span>
          {isCopied ? (
            <Check className="h-3 w-3 text-green-500 transition-all duration-200" />
          ) : (
            <Copy className="h-3 w-3 opacity-0 group-hover:opacity-100 transition-all duration-200" />
          )}
        </button>
        <Button
          variant="outline"
          size="icon-xs"
          aria-label={
            isVisible
              ? t("entries.detail.hidePassword")
              : t("entries.detail.revealPassword")
          }
          onClick={isVisible ? onHide : onReveal}
          disabled={isLoading || isDisabled}
        >
          {isVisible ? (
            <EyeOff className="h-3 w-3" />
          ) : (
            <Eye className="h-3 w-3" />
          )}
        </Button>
        <Button
          variant="outline"
          size="icon-xs"
          aria-label={t("entries.detail.autoType")}
          disabled={isDisabled}
        >
          <Keyboard className="h-3 w-3" />
        </Button>
      </div>
    </div>
  );
}

function UrlRow({
  url,
  isDisabled,
}: Readonly<{ url: string; isDisabled: boolean }>) {
  const { t } = useTranslation();
  const { copy, isCopied } = useCopyToClipboard();

  const handleCopy = async () => {
    if (isDisabled) return;
    try {
      await copy(url);
      toast.success(t("common.copied"));
    } catch (error) {
      console.error("Failed to copy URL:", error);
    }
  };

  const handleOpen = async () => {
    if (isDisabled) return;
    await openUrl(url);
  };

  return (
    <div className="flex min-w-0 justify-between items-center px-4 py-2 gap-2">
      <small className="shrink-0 text-sm font-medium">
        {t("entries.detail.url")}
      </small>
      <div className="flex w-0 min-w-0 flex-1 items-center justify-end-safe gap-2">
        <button
          onClick={handleCopy}
          disabled={isDisabled}
          className="group flex min-w-0 max-w-full flex-1 items-center justify-end gap-2 overflow-hidden rounded-sm px-2 py-1 text-sm font-medium text-muted-foreground transition-all duration-200 hover:bg-accent"
        >
          <span className="min-w-0 truncate text-right transition-all duration-200">
            {isCopied ? t("common.copied") : url}
          </span>
          {isCopied ? (
            <Check className="h-3 w-3 text-green-500 transition-all duration-200" />
          ) : (
            <Copy className="h-3 w-3 opacity-0 group-hover:opacity-100 transition-all duration-200" />
          )}
        </button>
        <Button
          variant="outline"
          size="icon-xs"
          aria-label={t("entries.detail.openUrl")}
          onClick={handleOpen}
          disabled={isDisabled}
        >
          <ExternalLink className="h-3 w-3" />
        </Button>
      </div>
    </div>
  );
}

function ProtectedCustomFieldRow({
  dbId,
  entryId,
  meta,
  isDisabled,
}: Readonly<{
  dbId: string;
  entryId: string;
  meta: CustomFieldMeta;
  isDisabled: boolean;
}>) {
  const { t } = useTranslation();
  const clipboardClearTimeout = useClipboardTimeout();
  const startCountdown = useClipboardCountdown();
  const [value, setValue] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [isCopied, setIsCopied] = useState(false);
  const isVisible = value !== null;

  const reveal = useCallback(async () => {
    if (isDisabled) return;
    setIsLoading(true);
    try {
      const result = await entriesApi.getProtectedCustomField(
        dbId,
        entryId,
        meta.key
      );
      setValue(result.value);
    } finally {
      setIsLoading(false);
    }
  }, [dbId, entryId, isDisabled, meta.key]);

  const hide = useCallback(() => setValue(null), []);

  const handleCopy = async () => {
    if (isDisabled) return;
    try {
      await clipboard.copyProtectedField(
        dbId,
        entryId,
        meta.key,
        clipboardClearTimeout
      );
      setIsCopied(true);
      toast.success(t("common.copied"));
      startCountdown(clipboardClearTimeout);
      setTimeout(() => setIsCopied(false), 2000);
    } catch (error) {
      console.error("Failed to copy protected field:", error);
    }
  };

  let displayValue: React.ReactNode;
  if (isLoading) {
    displayValue = <Loader2 className="inline h-3 w-3 animate-spin" />;
  } else if (isVisible) {
    displayValue = value;
  } else {
    displayValue = "••••••••";
  }

  return (
    <div className="flex min-w-0 justify-between items-center px-4 py-2 gap-2">
      <small className="shrink-0 text-sm font-medium">{meta.key}</small>
      <div className="flex w-0 min-w-0 flex-1 items-center justify-end-safe gap-2">
        <button
          onClick={handleCopy}
          disabled={isDisabled}
          className="group flex min-w-0 max-w-full flex-1 items-center justify-end gap-2 overflow-hidden rounded-sm px-2 py-1 text-sm font-medium text-muted-foreground transition-all duration-200 hover:bg-accent"
        >
          <span className="min-w-0 truncate text-right transition-all duration-200">
            {isCopied ? t("common.copied") : displayValue}
          </span>
          {isCopied ? (
            <Check className="h-3 w-3 text-green-500 transition-all duration-200" />
          ) : (
            <Copy className="h-3 w-3 opacity-0 group-hover:opacity-100 transition-all duration-200" />
          )}
        </button>
        <Button
          variant="outline"
          size="icon-xs"
          aria-label={
            isVisible
              ? t("entries.detail.hideField", { field: meta.key })
              : t("entries.detail.revealField", { field: meta.key })
          }
          onClick={isVisible ? hide : reveal}
          disabled={isLoading || isDisabled}
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

function EntryFieldRow({
  label,
  value,
  isDisabled,
}: Readonly<{
  label: string;
  value: string;
  isDisabled: boolean;
}>) {
  const { t } = useTranslation();
  const { copy, isCopied } = useCopyToClipboard();

  const handleCopy = async () => {
    if (isDisabled) return;
    try {
      await copy(value);
      toast.success(t("common.copied"));
    } catch (error) {
      console.error("Failed to copy field value:", error);
    }
  };

  return (
    <div className="flex min-w-0 justify-between items-center px-4 py-2 gap-2">
      <small className="shrink-0 text-sm font-medium">{label}</small>
      <div className="flex w-0 min-w-0 flex-1 items-center justify-end-safe gap-2">
        <button
          onClick={handleCopy}
          disabled={isDisabled}
          className="group flex min-w-0 max-w-full flex-1 items-center justify-end gap-2 overflow-hidden rounded-sm px-2 py-1 text-sm font-medium text-muted-foreground transition-all duration-200 hover:bg-accent"
        >
          <span className="min-w-0 truncate text-right transition-all duration-200">
            {isCopied ? t("common.copied") : value}
          </span>
          {isCopied ? (
            <Check className="h-3 w-3 text-green-500 transition-all duration-200" />
          ) : (
            <Copy className="h-3 w-3 opacity-0 group-hover:opacity-100 transition-all duration-200" />
          )}
        </button>
        <Button
          variant="outline"
          size="icon-xs"
          aria-label={t("entries.detail.autoType")}
          disabled={isDisabled}
        >
          <Keyboard className="h-3 w-3" />
        </Button>
      </div>
    </div>
  );
}

// Pick a file-type glyph from the MIME hint. Coarse on purpose: the read-only
// slice only needs images vs. text vs. everything-else; richer mapping can
// follow when preview/download land.
function attachmentIcon(mimeType: string) {
  if (mimeType.startsWith("image/")) return Image;
  if (
    mimeType.startsWith("text/") ||
    mimeType === "application/json" ||
    mimeType === "application/xml" ||
    mimeType === "application/yaml"
  ) {
    return FileText;
  }
  return File;
}

function AttachmentsSection({
  attachments,
  dbId,
  entryId,
  isDisabled,
}: Readonly<{
  attachments: AttachmentMeta[];
  dbId: string;
  entryId: string;
  isDisabled: boolean;
}>) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const isMobile = useIsMobile();
  const panelRef = useRef<HTMLDivElement>(null);
  // Guards against overlapping add gestures. Both write paths (picker, drop)
  // buffer their paths in one shared Rust-side buffer, so a second gesture
  // started while the first is still awaiting its confirmation prompt would
  // overwrite the first's paths before it commits. The ref is the synchronous
  // guard (a double-click can't slip through between renders); `isAdding` is the
  // rendered mirror that drives the spinner and disables the controls so the
  // user can see the work and can't accidentally re-trigger or interrupt it.
  const addInFlightRef = useRef(false);
  const [isAdding, setIsAdding] = useState(false);
  // The Attachment the user opened the Preview modal on, or null when the
  // modal is closed. Kept at the section level so the modal sits outside the
  // row mapping (one modal, not one per row).
  const [previewing, setPreviewing] = useState<AttachmentMeta | null>(null);

  // Sorted by filename, case-insensitively, for a stable display order (the
  // KDBX binary pool is unordered). Copy first so we never mutate props.
  const sorted = [...attachments].sort((a, b) =>
    a.filename.localeCompare(b.filename, undefined, { sensitivity: "base" })
  );

  // Preview fetches the bytes on demand via the same lazy byte-fetch the
  // download path uses (no audit event — Preview is a read inside the Vault,
  // not an export to the host filesystem).
  const fetchPreviewBytes = useCallback(
    (filename: string) =>
      entriesApi.getAttachmentBytes(dbId, entryId, filename),
    [dbId, entryId]
  );

  // Download is the only export path (ADR-0003): pick a destination with the
  // original filename pre-filled, then let the backend fetch the bytes and
  // write them in Rust so decrypted data never crosses into JS. A cancelled
  // dialog (null path) is a no-op; the audit event fires backend-side only on
  // a successful write.
  const handleDownload = async (filename: string) => {
    // Guard against a click landing mid-transition: useEntryDetail serves the
    // previous entry as placeholder data while switching, so the displayed
    // filename could belong to a different entry than the current entryId.
    if (isDisabled) return;
    try {
      const destPath = await save({ defaultPath: filename });
      if (!destPath) return;
      await entriesApi.exportAttachment(dbId, entryId, filename, destPath);
      toast.success(t("entries.detail.attachmentDownloaded", { filename }));
    } catch (error) {
      console.error("Failed to download attachment:", error);
      toast.error(t("entries.detail.attachmentDownloadFailed", { filename }));
    }
  };

  // Refreshes the entry detail and any list views after the in-memory Vault
  // changed, so added/removed attachment rows reflect backend state.
  const invalidateEntryQueries = () =>
    Promise.all([
      queryClient.invalidateQueries({
        queryKey: queryKeys.entries.detail(dbId, entryId),
      }),
      queryClient.invalidateQueries({
        predicate: (query) =>
          query.queryKey[0] === queryKeys.entries.all[0] &&
          query.queryKey[1] === dbId,
      }),
    ]);

  // Shared tail for both add paths (picker and drag-drop): the backend has
  // already read, size-capped, and auto-renamed each file, returning a batch
  // outcome. We surface one toast per failure naming the file and the backend's
  // reason (e.g. "exceeds the 25 MiB limit") so the user knows which file and
  // whether retrying could ever work, then persist once and refresh only when
  // at least one file actually landed in the in-memory Vault — so a
  // wholly-failed, cancelled, or empty batch leaves no phantom unsaved state.
  const applyAddOutcome = async (
    outcome: Awaited<ReturnType<typeof entriesApi.commitPreparedAttachments>>
  ) => {
    for (const failure of outcome.failed) {
      toast.error(
        t("entries.detail.attachmentAddFailed", {
          filename: failure.sourceName,
          reason: failure.reason,
        })
      );
    }

    if (outcome.added.length > 0) {
      await saveWithErrorToast(dbId, t);
      await invalidateEntryQueries();
      toast.success(
        t("entries.detail.attachmentsAdded", { count: outcome.added.length })
      );
    }
  };

  // Shared two-phase add flow for both write paths. Phase 1 (`prepare`) buffers
  // the picked/dropped paths in Rust and returns their size classification
  // against the configured thresholds — no bytes read, no Vault mutation. If any
  // file is over the soft threshold we prompt before committing; declining
  // aborts the whole batch (nothing is stored). Phase 2 (`commit`) drains the
  // buffer and stores the files, returning the batch outcome handled by the
  // shared tail. An empty plan (cancelled dialog, or a drop with nothing
  // buffered) is a no-op. The frontend never sees a path — the trust boundary in
  // ADR-0004.
  const runAddFlow = async (
    prepare: () => Promise<
      Awaited<ReturnType<typeof entriesApi.preparePickedAttachments>>
    >
  ) => {
    // Reject a concurrent gesture: the first one owns the shared buffer until it
    // commits or aborts, so a second prepare here would clobber its paths.
    if (addInFlightRef.current) return;
    addInFlightRef.current = true;
    setIsAdding(true);
    try {
      let plan: Awaited<ReturnType<typeof entriesApi.preparePickedAttachments>>;
      try {
        plan = await prepare();
      } catch (error) {
        console.error("Failed to prepare attachments:", error);
        toast.error(t("entries.detail.attachmentAddBatchFailed"));
        return;
      }

      // Nothing picked/dropped: a true no-op, so we never reach commit.
      if (plan.items.length === 0) return;

      if (plan.requiresConfirmation) {
        const confirmed = await ask(
          t("entries.detail.attachmentSoftWarnConfirm"),
          {
            title: t("entries.detail.attachmentSoftWarnConfirmTitle"),
            kind: "warning",
          }
        );
        // Abort the whole batch on decline: the buffered paths stay until the
        // next pick/drop overwrites them, but nothing is stored now.
        if (!confirmed) return;
      }

      let outcome: Awaited<
        ReturnType<typeof entriesApi.commitPreparedAttachments>
      >;
      try {
        // Echo the prepared batch id so a superseded batch (a later pick/drop)
        // commits nothing rather than the wrong file.
        outcome = await entriesApi.commitPreparedAttachments(
          dbId,
          entryId,
          plan.batchId
        );
      } catch (error) {
        console.error("Failed to add attachments:", error);
        toast.error(t("entries.detail.attachmentAddBatchFailed"));
        return;
      }

      await applyAddOutcome(outcome);
    } finally {
      addInFlightRef.current = false;
      setIsAdding(false);
    }
  };

  // Add is the primary write path: the picker opens the multi-select dialog in
  // Rust (ADR-0004) and buffers the chosen paths for the shared flow.
  const handleAdd = async () => {
    if (isDisabled) return;
    await runAddFlow(() => entriesApi.preparePickedAttachments());
  };

  // Drag-and-drop is the second write path (desktop only). The dropped paths
  // were captured in Rust from the native window event (ADR-0004); the drop hook
  // has already scoped the drop to this panel. From here it's identical to the
  // picker — same classification, prompt, and commit.
  const handleDrop = () => {
    if (isDisabled) return;
    void runAddFlow(() => entriesApi.prepareDroppedAttachments());
  };

  // The native drag-drop event is window-global; the hook acts only on desktop
  // and only when a drop lands inside this panel (and not mid-transition).
  // Ignore drops while an add is already in flight: the in-flight gesture owns
  // the buffer, and accepting another drop would only supersede it (and be a
  // no-op at commit anyway). Disabling here keeps the drop zone honest with the
  // busy state the user sees.
  const { isDragOver } = useAttachmentDrop({
    enabled: !isMobile && !isDisabled && !isAdding,
    panelRef,
    onDrop: handleDrop,
  });

  // Deleting an attachment is destructive and has no undo, so we confirm
  // first. On confirm the backend drops the Entry's reference (and the
  // orphaned blob), then we persist and invalidate the entry queries so the
  // row disappears. A cancelled prompt leaves the attachment intact.
  const handleDelete = async (filename: string) => {
    // Same mid-transition guard as download: the displayed filename could
    // belong to the previous entry while useEntryDetail swaps placeholders.
    if (isDisabled) return;
    const confirmed = await ask(
      t("entries.detail.deleteAttachmentConfirm", { filename }),
      {
        title: t("entries.detail.deleteAttachmentConfirmTitle"),
        kind: "warning",
      }
    );
    if (!confirmed) return;
    try {
      await entriesApi.deleteAttachment(dbId, entryId, filename);
    } catch (error) {
      console.error("Failed to delete attachment:", error);
      toast.error(t("entries.detail.attachmentDeleteFailed", { filename }));
      return;
    }
    // The reference is now gone from the in-memory Vault. Persist via the
    // shared helper, which surfaces its own toast on a disk/backup failure
    // without rejecting — mirroring the entry-mutation convention. We must
    // still invalidate either way so the UI reflects backend memory (the row
    // disappears); skipping it on save failure would leave a stale row whose
    // later actions target an attachment that no longer exists.
    await saveWithErrorToast(dbId, t);
    await invalidateEntryQueries();
    toast.success(t("entries.detail.attachmentDeleted", { filename }));
  };

  return (
    <div ref={panelRef} className="border rounded-md">
      <div className="flex items-center justify-between px-4 py-2">
        <small className="text-sm font-medium">
          {t("entries.detail.attachments")}
        </small>
        <Button
          variant="ghost"
          size="sm"
          className="h-7 gap-1.5"
          aria-label={t("entries.detail.addAttachment")}
          aria-busy={isAdding}
          disabled={isDisabled || isAdding}
          onClick={() => void handleAdd()}
        >
          {isAdding ? (
            <>
              <Loader2 className="h-4 w-4 animate-spin" />
              {t("entries.detail.addingAttachments")}
            </>
          ) : (
            <>
              <Paperclip className="h-4 w-4" />
              {t("entries.detail.addAttachment")}
            </>
          )}
        </Button>
      </div>
      <Separator />
      {sorted.length === 0 ? (
        <p className="px-4 py-2 text-sm text-muted-foreground">
          {t("entries.detail.noAttachments")}
        </p>
      ) : (
        <ul>
          {sorted.map((attachment, index) => {
            // attachmentIcon returns a stable module-level lucide component
            // (Image/FileText/File); render it via createElement so the icon
            // lookup isn't misread as a component defined during render.
            const icon = createElement(attachmentIcon(attachment.mimeType), {
              className: "h-4 w-4 shrink-0 text-muted-foreground",
            });
            // Non-previewable types (PDF, SVG, archives, opaque binaries)
            // don't get a Preview affordance at all — the spec says no
            // broken preview button.
            const isPreviewable =
              classifyAttachment(attachment).kind !== "none";
            return (
              <li key={attachment.filename}>
                {index > 0 && <Separator />}
                <div className="flex min-w-0 items-center gap-3 px-4 py-2">
                  {icon}
                  <span className="min-w-0 flex-1 truncate text-sm font-medium">
                    {attachment.filename}
                  </span>
                  <span className="shrink-0 text-sm text-muted-foreground">
                    {formatAttachmentSize(attachment.size)}
                  </span>
                  {isPreviewable && (
                    <Button
                      variant="ghost"
                      size="icon"
                      className="h-7 w-7 shrink-0"
                      aria-label={t("entries.detail.previewAttachment")}
                      disabled={isDisabled}
                      onClick={() => setPreviewing(attachment)}
                    >
                      <Eye className="h-4 w-4" />
                    </Button>
                  )}
                  <Button
                    variant="ghost"
                    size="icon"
                    className="h-7 w-7 shrink-0"
                    aria-label={t("entries.detail.downloadAttachment")}
                    disabled={isDisabled}
                    onClick={() => void handleDownload(attachment.filename)}
                  >
                    <Download className="h-4 w-4" />
                  </Button>
                  <Button
                    variant="ghost"
                    size="icon"
                    className="h-7 w-7 shrink-0 text-destructive hover:text-destructive"
                    aria-label={t("entries.detail.deleteAttachment")}
                    disabled={isDisabled}
                    onClick={() => void handleDelete(attachment.filename)}
                  >
                    <Trash2 className="h-4 w-4" />
                  </Button>
                </div>
              </li>
            );
          })}
        </ul>
      )}
      {/* Drop zone is desktop-only: mobile has no OS file-drop, so only the
          picker button above is offered there (hidden via useIsMobile). */}
      {!isMobile && (
        <>
          <Separator />
          <div className="px-4 py-2">
            <div
              className={cn(
                "flex items-center justify-center gap-2 rounded-md border border-dashed px-4 py-6 text-sm text-muted-foreground transition-colors",
                isDragOver && "border-primary bg-primary/5 text-foreground"
              )}
            >
              <Paperclip className="h-4 w-4" />
              {t("entries.detail.dropToAttach")}
            </div>
          </div>
        </>
      )}
      {previewing && (
        <AttachmentPreviewModal
          open={true}
          onOpenChange={(open) => {
            if (!open) setPreviewing(null);
          }}
          attachment={previewing}
          fetchBytes={fetchPreviewBytes}
        />
      )}
    </div>
  );
}

function EntryFieldBasic({
  label,
  value,
}: Readonly<{ label: string; value: string }>) {
  return (
    <div className="flex justify-between items-center px-4 py-2">
      <small className="text-sm font-medium">{label}</small>
      <small className="text-sm font-medium text-muted-foreground">
        {value}
      </small>
    </div>
  );
}

// The expiry instant is stored UTC; render it in the viewer's local time
// via dayjs so the displayed date matches the date picker in the editor.
function formatExpiry(isoString: string): string {
  return dayjs(isoString).format("MMM D, YYYY h:mm:ss A");
}

function formatDate(isoString: string): string {
  const date = new Date(isoString);
  return new Intl.DateTimeFormat(undefined, {
    year: "numeric",
    month: "short",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit",
    second: "2-digit",
  }).format(date);
}
