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

export default function DropdownMenuOpenDatabase() {
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
          <DropdownMenuItem>Local file</DropdownMenuItem>
        </DropdownMenuGroup>
        <DropdownMenuSeparator />
        <DropdownMenuLabel>Online</DropdownMenuLabel>
        <DropdownMenuGroup>
          <DropdownMenuItem>Google Drive</DropdownMenuItem>
          <DropdownMenuItem>OneDrive</DropdownMenuItem>
          <DropdownMenuItem>DropBox</DropdownMenuItem>
          <DropdownMenuItem>WebDAV</DropdownMenuItem>
        </DropdownMenuGroup>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
