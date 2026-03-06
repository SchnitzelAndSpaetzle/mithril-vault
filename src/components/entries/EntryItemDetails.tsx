import { createElement, useCallback, useState } from "react";
import { useTranslation } from "react-i18next";
import { Separator } from "@/components/ui/separator.tsx";
import {
  Check,
  Copy,
  ExternalLink,
  Eye,
  EyeOff,
  Keyboard,
  Loader2,
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
import { useClipboardTimeout } from "@/hooks/use-clipboard-timeout";
import { clipboard, entries as entriesApi } from "@/lib/tauri";
import { getKeepassIcon } from "@/lib/keepass-icons";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { CustomFieldMeta } from "@/lib/types";

interface EntryItemDetailsProps {
  entryId: string;
  dbId: string;
}

export default function EntryItemDetails({
  entryId,
  dbId,
}: EntryItemDetailsProps) {
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

  return (
    <>
      {/* Title section */}
      <div className="flex items-center gap-4 px-4">
        <Avatar>
          <AvatarImage
            src={customIcon ? `data:image/png;base64,${customIcon}` : undefined}
            alt=""
          />
          <AvatarFallback>
            {createElement(iconComponent, { className: "h-4 w-4" })}
          </AvatarFallback>
        </Avatar>
        <h4 className="scroll-m-20 text-xl font-semibold tracking-tight">
          {entry.title}
        </h4>
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
      </div>
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
}: {
  dbId: string;
  entryId: string;
  password: string | null;
  isVisible: boolean;
  isLoading: boolean;
  isDisabled: boolean;
  onReveal: () => void;
  onHide: () => void;
}) {
  const { t } = useTranslation();
  const clipboardClearTimeout = useClipboardTimeout();
  const [isCopied, setIsCopied] = useState(false);

  const handleCopy = async () => {
    if (isDisabled) return;
    await clipboard.copyPassword(dbId, entryId, clipboardClearTimeout);
    setIsCopied(true);
    setTimeout(() => setIsCopied(false), 2000);
  };

  const displayValue = isLoading ? (
    <Loader2 className="inline h-3 w-3 animate-spin" />
  ) : isVisible ? (
    password
  ) : (
    "••••••••"
  );

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

function UrlRow({ url, isDisabled }: { url: string; isDisabled: boolean }) {
  const { t } = useTranslation();
  const { copy, isCopied } = useCopyToClipboard();

  const handleCopy = () => {
    if (isDisabled) return;
    void copy(url);
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
}: {
  dbId: string;
  entryId: string;
  meta: CustomFieldMeta;
  isDisabled: boolean;
}) {
  const { t } = useTranslation();
  const [value, setValue] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);
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

  return (
    <div className="flex min-w-0 justify-between items-center px-4 py-2 gap-2">
      <small className="shrink-0 text-sm font-medium">{meta.key}</small>
      <div className="flex w-0 min-w-0 flex-1 items-center justify-end-safe gap-2">
        <span className="min-w-0 truncate text-right text-sm font-medium text-muted-foreground">
          {isLoading ? (
            <Loader2 className="inline h-3 w-3 animate-spin" />
          ) : isVisible ? (
            value
          ) : (
            "••••••••"
          )}
        </span>
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
}: {
  label: string;
  value: string;
  isDisabled: boolean;
}) {
  const { t } = useTranslation();
  const { copy, isCopied } = useCopyToClipboard();

  const handleCopy = () => {
    if (isDisabled) return;
    void copy(value);
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

function EntryFieldBasic({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex justify-between items-center px-4 py-2">
      <small className="text-sm font-medium">{label}</small>
      <small className="text-sm font-medium text-muted-foreground">
        {value}
      </small>
    </div>
  );
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
