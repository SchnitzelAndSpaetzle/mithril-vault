import {
  Item,
  ItemActions,
  ItemContent,
  ItemDescription,
  ItemMedia,
  ItemTitle,
} from "@/components/ui/item.tsx";
import { ChevronRightIcon, FolderOpen } from "lucide-react";
import { useNavigate } from "@tanstack/react-router";
import type { RecentDatabase } from "@/lib/types.ts";

function getFilenameFromPath(path: string): string {
  const parts = path.split(/[/\\]/);
  return parts[parts.length - 1] || path;
}

interface RecentOpenedDatabaseItemsProps {
  recentDatabases: RecentDatabase[];
}

export default function RecentOpenedDatabaseItems({
  recentDatabases,
}: RecentOpenedDatabaseItemsProps) {
  const navigate = useNavigate();

  if (recentDatabases.length === 0) {
    return (
      <div className="flex w-full max-w-md flex-col items-center justify-center py-8 text-muted-foreground">
        <FolderOpen className="size-8 mb-2" />
        <p className="text-sm">No recent databases</p>
        <p className="text-xs">Open a database to see it here</p>
      </div>
    );
  }

  return (
    <div className="flex w-full max-w-md flex-col gap-2">
      {recentDatabases.map((item) => (
        <Item key={item.path} variant="outline" size="sm" asChild>
          <button
            type="button"
            className="w-full text-left"
            onClick={() => {
              void navigate({ to: "/unlock", search: { path: item.path } });
            }}
          >
            <ItemMedia>
              <FolderOpen className="size-5" />
            </ItemMedia>
            <ItemContent>
              <ItemTitle>{getFilenameFromPath(item.path)}</ItemTitle>
              <ItemDescription className="line-clamp-1">
                {item.path}
              </ItemDescription>
            </ItemContent>
            <ItemActions>
              <ChevronRightIcon className="size-4" />
            </ItemActions>
          </button>
        </Item>
      ))}
    </div>
  );
}
