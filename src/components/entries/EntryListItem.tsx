import type { CustomIconMap, Entry } from "@/lib/types";
import { CircleAlert } from "lucide-react";
import { createElement, memo } from "react";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import {
  Item,
  ItemActions,
  ItemContent,
  ItemDescription,
  ItemMedia,
  ItemTitle,
} from "@/components/ui/item";
import { getKeepassIcon } from "@/lib/keepass-icons";
import { cn } from "@/lib/utils";

interface EntryListItemProps extends Entry {
  customIcons: CustomIconMap;
  isSelected?: boolean;
  onClick?: (id: string) => void;
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
}: EntryListItemProps) {
  const iconComponent = getKeepassIcon(iconId ?? 0);
  const customIcon = customIconUuid ? customIcons[customIconUuid] : null;

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
            <AvatarImage
              src={
                customIcon ? `data:image/png;base64,${customIcon}` : undefined
              }
              alt=""
            />
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
          {/* TODO: show warning icon if password is duplicated or compromised */}
          <CircleAlert className="size-4" />
        </ItemActions>
      </a>
    </Item>
  );
});

export default EntryListItem;
