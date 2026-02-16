import type { CustomIconMap, Entry } from "@/lib/types";
import { CircleAlert } from "lucide-react";
import { createElement } from "react";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import {
  Item,
  ItemActions,
  ItemContent,
  ItemDescription,
  ItemMedia,
  ItemTitle,
} from "@/components/ui/item";
import { useNavigate } from "@tanstack/react-router";
import { useIsMobile } from "@/hooks/use-mobile.ts";
import { getKeepassIcon } from "@/lib/keepass-icons";

interface EntryListItemProps extends Entry {
  customIcons: CustomIconMap;
}

export default function EntryListItem({
  username,
  title,
  id,
  iconId,
  customIconUuid,
  customIcons,
}: EntryListItemProps) {
  const isMobile = useIsMobile();
  const navigate = useNavigate({ from: "/dashboard/entry/$id" });
  const iconComponent = getKeepassIcon(iconId ?? 0);
  const customIcon = customIconUuid ? customIcons[customIconUuid] : null;

  const handleClick = async () => {
    if (isMobile) {
      await navigate({ to: "/dashboard/entry/$id", params: { id } });
    } else {
      //TODO: switch panel for desktop
    }
  };

  return (
    <Item
      asChild
      variant="default"
      size="sm"
      className="w-full min-w-0 p-2 rounded-none flex-nowrap"
    >
      <a className="w-full min-w-0 overflow-hidden" onClick={handleClick}>
        <ItemMedia>
          <Avatar className="size-10">
            {customIcon ? (
              <AvatarImage src={`data:image/png;base64,${customIcon}`} alt="" />
            ) : null}
            <AvatarFallback>
              {createElement(iconComponent, { className: "h-4 w-4" })}
            </AvatarFallback>
          </Avatar>
        </ItemMedia>
        <ItemContent className="min-w-0 flex-1 overflow-hidden">
          <ItemTitle className="truncate w-full">{title}</ItemTitle>
          <div className="min-w-0 w-full">
            <ItemDescription className="line-clamp-1 w-full min-w-0 wrap-break-words">
              {username}
            </ItemDescription>
          </div>
        </ItemContent>
        <ItemActions className="shrink-0">
          {/* TODO: show warning icon if password is duplicated or compromised */}
          <CircleAlert className="size-4" />
        </ItemActions>
      </a>
    </Item>
  );
}
