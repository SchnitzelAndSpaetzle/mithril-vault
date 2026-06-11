import type { CustomIconMap, Entry, Finding } from "@/lib/types";
import { OctagonAlert, Paperclip, TriangleAlert } from "lucide-react";
import { createElement, memo } from "react";
import { useTranslation } from "react-i18next";
import { severityOf } from "@/lib/password-health";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import {
  Item,
  ItemActions,
  ItemContent,
  ItemDescription,
  ItemMedia,
  ItemTitle,
} from "@/components/ui/item";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { getKeepassIcon } from "@/lib/keepass-icons";
import { isExpired } from "@/lib/entry-expiry";
import { cn } from "@/lib/utils";

interface EntryListItemProps extends Entry {
  customIcons: CustomIconMap;
  isSelected?: boolean;
  onClick?: (id: string) => void;
  /// Password Health Findings scoped to this Entry. The list comes
  /// from `useEntryFindings(dbId, entry.id)` on the parent. An empty
  /// array renders no icon; one or more Findings render the warning.
  findings?: Finding[];
  /// Size of the reuse group this Entry belongs to, when applicable
  /// (≥ 2). Used to fill the "Reused (N entries)" tooltip — the
  /// number is not derivable from the Finding alone, so the parent
  /// passes it in. `undefined` when the Entry has no reused finding
  /// or the report isn't loaded yet; falsy renders the plain kind
  /// label without the count.
  reusedGroupSize?: number | undefined;
}

const EntryListItem = memo(function EntryListItem({
  username,
  title,
  id,
  iconId,
  customIconUuid,
  customIcons,
  isSelected,
  onClick,
  findings,
  reusedGroupSize,
  expires,
  expiryTime,
  attachments,
}: EntryListItemProps) {
  const { t } = useTranslation();
  const iconComponent = getKeepassIcon(iconId ?? 0);
  const expired = isExpired({ expires, expiryTime }, new Date());
  const customIcon = customIconUuid ? customIcons[customIconUuid] : null;
  const customIconSrc = customIcon
    ? `data:${customIcon.mimeType};base64,${customIcon.data}`
    : undefined;

  const handleClick = () => {
    onClick?.(id);
  };

  return (
    <Item
      asChild
      variant="default"
      size="sm"
      className={cn(
        "w-full min-w-0 p-2 rounded-none flex-nowrap",
        isSelected && "bg-accent"
      )}
    >
      <button
        type="button"
        className="w-full min-w-0 overflow-hidden text-left"
        onClick={handleClick}
      >
        <ItemMedia>
          <Avatar className="size-10">
            <AvatarImage src={customIconSrc} alt="" />
            <AvatarFallback>
              {createElement(iconComponent, { className: "h-4 w-4" })}
            </AvatarFallback>
          </Avatar>
        </ItemMedia>
        <ItemContent className="min-w-0 flex-1 overflow-hidden">
          <ItemTitle
            className={cn(
              "block truncate w-full",
              expired && "line-through text-muted-foreground"
            )}
          >
            {title}
          </ItemTitle>
          <ItemDescription className="line-clamp-none truncate w-full min-w-0">
            {username}
          </ItemDescription>
        </ItemContent>
        <ItemActions className="shrink-0">
          {attachments.length > 0 && (
            <Paperclip
              className="size-4 text-muted-foreground"
              aria-label={t("entries.attachmentIndicator")}
            />
          )}
          {findings && findings.length > 0 && (
            <FindingsIndicator
              findings={findings}
              reusedGroupSize={reusedGroupSize}
            />
          )}
        </ItemActions>
      </button>
    </Item>
  );
});

/// Renders the per-Entry warning icon for the EntryList. Critical
/// findings (e.g. very_weak / empty password) use a red OctagonAlert;
/// High-only findings keep the amber TriangleAlert. The tooltip and
/// aria-label spell out each Finding Kind in plain language so screen
/// readers don't read out enum identifiers.
function FindingsIndicator({
  findings,
  reusedGroupSize,
}: Readonly<{ findings: Finding[]; reusedGroupSize?: number | undefined }>) {
  const { t } = useTranslation();
  const hasCritical = findings.some((f) => severityOf(f.kind) === "critical");
  // De-duplicate Finding Kinds before formatting — an Entry with two
  // expired-style Findings (theoretical) should still show one row.
  const uniqueKinds = Array.from(new Set(findings.map((f) => f.kind)));
  const label = uniqueKinds
    .map((kind) => {
      // Reused gets the member-count suffix from the parent. Without
      // the count we fall back to the plain kind label so the
      // indicator still renders when the report is still loading.
      if (kind === "password.reused" && reusedGroupSize) {
        return t("passwordHealth.reused.tooltip", { count: reusedGroupSize });
      }
      return t(`passwordHealth.findings.${kind}`);
    })
    .join(" · ");

  const Icon = hasCritical ? OctagonAlert : TriangleAlert;
  const colorClass = hasCritical
    ? "text-red-600 dark:text-red-500"
    : "text-amber-600 dark:text-amber-500";

  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <Icon className={cn("size-4", colorClass)} aria-label={label} />
      </TooltipTrigger>
      <TooltipContent>{label}</TooltipContent>
    </Tooltip>
  );
}

export default EntryListItem;
