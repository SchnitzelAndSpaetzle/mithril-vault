// SPDX-License-Identifier: MIT

import { createElement, memo } from "react";
import type { CustomIconMap } from "@/lib/types";
import type { SearchResult } from "@/lib/search-utils";
import { HighlightedText } from "@/components/ui/highlighted-text";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import {
  Item,
  ItemContent,
  ItemDescription,
  ItemMedia,
  ItemTitle,
} from "@/components/ui/item";
import { getKeepassIcon } from "@/lib/keepass-icons";
import { cn } from "@/lib/utils";

interface SearchResultItemProps {
  result: SearchResult;
  query: string;
  groupPath: string;
  customIcons: CustomIconMap;
  isSelected: boolean;
  onClick: (id: string) => void;
}

const SearchResultItem = memo(function SearchResultItem({
  result,
  query,
  groupPath,
  customIcons,
  isSelected,
  onClick,
}: SearchResultItemProps) {
  const { entry } = result;
  const iconComponent = getKeepassIcon(entry.iconId ?? 0);
  const customIcon = entry.customIconUuid
    ? customIcons[entry.customIconUuid]
    : null;
  const customIconSrc = customIcon
    ? `data:${customIcon.mimeType};base64,${customIcon.data}`
    : undefined;

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
      <a
        className="w-full min-w-0 overflow-hidden"
        onClick={() => onClick(entry.id)}
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
          <ItemTitle className="block truncate w-full">
            <HighlightedText text={entry.title} query={query} />
          </ItemTitle>
          <ItemDescription className="line-clamp-none truncate w-full min-w-0">
            <HighlightedText text={entry.username} query={query} />
          </ItemDescription>
          {groupPath && (
            <span className="block truncate text-xs text-muted-foreground">
              {groupPath}
            </span>
          )}
        </ItemContent>
      </a>
    </Item>
  );
});

export default SearchResultItem;
