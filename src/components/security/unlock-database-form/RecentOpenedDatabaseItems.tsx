import {
  Item,
  ItemActions,
  ItemContent,
  ItemDescription,
  ItemMedia,
  ItemTitle,
} from "@/components/ui/item.tsx";
import { ChevronRightIcon, FolderOpen, Loader2 } from "lucide-react";
import { Link } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import { settings } from "@/lib/tauri.ts";
import type { RecentDatabase } from "@/lib/types.ts";

function getFilenameFromPath(path: string): string {
  const parts = path.split(/[/\\]/);
  return parts[parts.length - 1] || path;
}

export default function RecentOpenedDatabaseItems() {
  const [recentDatabases, setRecentDatabases] = useState<RecentDatabase[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    async function loadRecentDatabases() {
      try {
        const appSettings = await settings.get();
        setRecentDatabases(appSettings.recentDatabases);
      } catch (e) {
        setError(String(e));
      } finally {
        setIsLoading(false);
      }
    }
    void loadRecentDatabases();
  }, []);

  if (isLoading) {
    return (
      <div className="flex w-full max-w-md items-center justify-center py-8">
        <Loader2 className="size-6 animate-spin text-muted-foreground" />
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex w-full max-w-md flex-col items-center justify-center py-8 text-muted-foreground">
        <p className="text-sm">Failed to load recent databases</p>
      </div>
    );
  }

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
          <Link to="/unlock" search={{ path: item.path }}>
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
          </Link>
        </Item>
      ))}
    </div>
  );
}
