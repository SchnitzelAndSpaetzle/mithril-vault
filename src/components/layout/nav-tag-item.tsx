import { EllipsisVertical, Pencil, Trash2 } from "lucide-react";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  SidebarMenuButton,
  SidebarMenuSubItem,
} from "@/components/ui/sidebar.tsx";

interface NavTagItemProps {
  tag: string;
  isActive: boolean;
  onSelect: (tag: string) => void;
  onRename: (tag: string) => void;
  onDelete: (tag: string) => void;
}

export function NavTagItem({
  tag,
  isActive,
  onSelect,
  onRename,
  onDelete,
}: NavTagItemProps) {
  return (
    <SidebarMenuSubItem>
      <SidebarMenuButton
        isActive={isActive}
        onClick={() => onSelect(tag)}
        className="cursor-pointer"
      >
        {tag}
      </SidebarMenuButton>
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <button
            type="button"
            aria-label={`Actions for tag ${tag}`}
            className="absolute right-1 top-1/2 -translate-y-1/2 rounded-sm p-0.5 opacity-0 hover:bg-sidebar-accent group-hover/menu-sub-item:opacity-100 focus-visible:opacity-100"
          >
            <EllipsisVertical className="size-4" />
          </button>
        </DropdownMenuTrigger>
        <DropdownMenuContent side="right" align="start">
          <DropdownMenuItem onClick={() => onRename(tag)}>
            <Pencil className="mr-2 size-4" />
            Rename
          </DropdownMenuItem>
          <DropdownMenuItem
            onClick={() => onDelete(tag)}
            className="text-destructive focus:text-destructive"
          >
            <Trash2 className="mr-2 size-4" />
            Delete
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    </SidebarMenuSubItem>
  );
}
