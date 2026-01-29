import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { FolderOpen } from "lucide-react";
import { open } from "@tauri-apps/plugin-dialog";
import { useNavigate } from "@tanstack/react-router";

export default function DropdownMenuOpenDatabase() {
  const navigate = useNavigate();

  async function handleSelectLocalFile() {
    try {
      const file = await open({
        title: "Open Database",
        filters: [{ name: "KeePass Database", extensions: ["kdbx"] }],
      });
      if (file) {
        await navigate({ to: "/unlock", search: { path: file as string } });
      }
    } catch {
      // User cancelled or error - ignore
    }
  }

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="outline">
          <FolderOpen />
          Open
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent className="w-56" align="start">
        <DropdownMenuLabel>Offline</DropdownMenuLabel>
        <DropdownMenuGroup>
          <DropdownMenuItem onSelect={handleSelectLocalFile}>
            Local file
          </DropdownMenuItem>
        </DropdownMenuGroup>
        <DropdownMenuSeparator />
        <DropdownMenuLabel>Online</DropdownMenuLabel>
        <DropdownMenuGroup>
          <DropdownMenuItem disabled>Google Drive</DropdownMenuItem>
          <DropdownMenuItem disabled>OneDrive</DropdownMenuItem>
          <DropdownMenuItem disabled>DropBox</DropdownMenuItem>
          <DropdownMenuItem disabled>WebDAV</DropdownMenuItem>
        </DropdownMenuGroup>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
