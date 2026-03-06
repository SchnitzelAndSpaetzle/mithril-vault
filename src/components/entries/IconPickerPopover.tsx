import { createElement, type ReactNode, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import { getKeepassIcon, KEEPASS_ICON_MAP } from "@/lib/keepass-icons";
import { cn } from "@/lib/utils";

interface IconPickerPopoverProps {
  selectedIconId: number;
  onSelect: (iconId: number) => void;
  children: ReactNode;
}

const ICON_IDS = Object.keys(KEEPASS_ICON_MAP).map(Number);

export function IconPickerPopover({
  selectedIconId,
  onSelect,
  children,
}: IconPickerPopoverProps) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>{children}</PopoverTrigger>
      <PopoverContent className="w-80 p-2" align="start">
        <div className="mb-2 text-sm font-medium">
          {t("iconPicker.chooseIcon")}
        </div>
        <div className="grid grid-cols-10 gap-1">
          {ICON_IDS.map((id) => (
            <button
              key={id}
              type="button"
              aria-label={`Icon ${id}`}
              onClick={() => {
                onSelect(id);
                setOpen(false);
              }}
              className={cn(
                "flex size-7 items-center justify-center rounded-md transition-colors hover:bg-accent",
                id === selectedIconId && "bg-accent ring-2 ring-primary"
              )}
            >
              {createElement(getKeepassIcon(id), {
                className: "size-4 text-muted-foreground",
              })}
            </button>
          ))}
        </div>
      </PopoverContent>
    </Popover>
  );
}
