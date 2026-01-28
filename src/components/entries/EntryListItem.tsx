import type { Entry } from "@/lib";
import { CircleAlert } from "lucide-react";
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

export default function EntryListItem({ username, title }: Entry) {
  const isMobile = useIsMobile();
  const navigate = useNavigate({ from: "/dashboard/entry/$id" });

  const handleClick = async () => {
    if (isMobile) {
      await navigate({ to: "/dashboard/entry/$id", params: { id: "234" } });
    } else {
      //TODO: switch panel for desktop
    }
  };

  return (
    <Item asChild variant="default" size="sm" className="p-2 rounded-none">
      <a onClick={handleClick}>
        <ItemMedia>
          <Avatar className="size-10">
            <AvatarImage src="https://github.com/shadcn.png" />
            <AvatarFallback>ER</AvatarFallback>
          </Avatar>
        </ItemMedia>
        <ItemContent className="truncate">
          <ItemTitle className="truncate">{title}</ItemTitle>
          <ItemDescription className="truncate">{username}</ItemDescription>
        </ItemContent>
        <ItemActions>
          {/* TODO: show warning icon if password is duplicated or compromised */}
          <CircleAlert className="size-4" />
        </ItemActions>
      </a>
    </Item>
  );
}
