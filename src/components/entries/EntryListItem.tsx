import type { CustomIconMap, Entry, Finding } from "@/lib/types";
import { TriangleAlert } from "lucide-react";
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
import { cn } from "@/lib/utils";

interface EntryListItemProps extends Entry {
  customIcons: CustomIconMap;
  isSelected?: boolean;
  onClick?: (id: string) => void;
  /// Password Health Findings scoped to this Entry. The list comes
  /// from `useEntryFindings(dbId, entry.id)` on the parent. An empty
  /// array renders no icon; one or more Findings render the warning.
  findings?: Finding[];
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
}: EntryListItemProps) {
  const { t } = useTranslation();
  const iconComponent = getKeepassIcon(iconId ?? 0);
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
      <a className="w-full min-w-0 overflow-hidden" onClick={handleClick}>
        <ItemMedia>
          <Avatar className="size-10">
            <AvatarImage src={customIconSrc} alt="" />
            <AvatarFallback>
              {createElement(iconComponent, { className: "h-4 w-4" })}
            </AvatarFallback>
          </Avatar>
        </ItemMedia>
        <ItemContent className="min-w-0 flex-1 overflow-hidden">
          <ItemTitle className="block truncate w-full">{title}</ItemTitle>
          <ItemDescription className="line-clamp-none truncate w-full min-w-0">
            {username}
          </ItemDescription>
        </ItemContent>
        <ItemActions className="shrink-0">
          {findings && findings.length > 0 && (
            <Tooltip>
              <TooltipTrigger asChild>
                <TriangleAlert
                  className={cn(
                    "size-4",
                    findings.some((f) => severityOf(f.kind) === "critical")
                      ? "text-red-600 dark:text-red-500"
                      : "text-amber-600 dark:text-amber-500"
                  )}
                  aria-label={t("passwordHealth.icons.warning")}
                />
              </TooltipTrigger>
              <TooltipContent>
                {t("passwordHealth.icons.warning")}
              </TooltipContent>
            </Tooltip>
          )}
        </ItemActions>
      </a>
    </Item>
  );
});

export default EntryListItem;
