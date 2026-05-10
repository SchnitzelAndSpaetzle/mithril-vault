import { createElement, type ReactNode, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import { getKeepassIcon, KEEPASS_ICON_MAP } from "@/lib/keepass-icons";
import type { CustomIconMap } from "@/lib/types";
import { cn } from "@/lib/utils";

interface IconPickerPopoverProps {
  selectedIconId: number;
  selectedCustomIconUuid?: string | null;
  customIcons?: CustomIconMap;
  onSelect: (iconId: number) => void;
  onSelectCustomIcon?: (iconUuid: string) => void;
  children: ReactNode;
}

const ICON_IDS = Object.keys(KEEPASS_ICON_MAP).map(Number);

export function IconPickerPopover({
  selectedIconId,
  selectedCustomIconUuid = null,
  customIcons = {},
  onSelect,
  onSelectCustomIcon,
  children,
}: IconPickerPopoverProps) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const customIconEntries = Object.entries(customIcons);

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>{children}</PopoverTrigger>
      <PopoverContent className="w-80 p-2" align="start">
        <div className="mb-2 text-sm font-medium">
          {t("iconPicker.chooseIcon")}
        </div>
        {customIconEntries.length > 0 && (
          <>
            <div className="mb-1 text-xs font-medium text-muted-foreground">
              {t("iconPicker.customIcons")}
            </div>
            <div className="mb-3 grid grid-cols-10 gap-1">
              {customIconEntries.map(([uuid, icon]) => (
                <button
                  key={uuid}
                  type="button"
                  aria-label={t("iconPicker.customIconLabel", { uuid })}
                  onClick={() => {
                    onSelectCustomIcon?.(uuid);
                    setOpen(false);
                  }}
                  className={cn(
                    "flex size-7 items-center justify-center overflow-hidden rounded-md transition-colors hover:bg-accent",
                    uuid === selectedCustomIconUuid &&
                      "bg-accent ring-2 ring-primary"
                  )}
                >
                  <img
                    src={`data:${icon.mimeType};base64,${icon.data}`}
                    alt=""
                    className="size-4 object-contain"
                  />
                </button>
              ))}
            </div>
            <div className="mb-1 text-xs font-medium text-muted-foreground">
              {t("iconPicker.builtInIcons")}
            </div>
          </>
        )}
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
                id === selectedIconId &&
                  selectedCustomIconUuid === null &&
                  "bg-accent ring-2 ring-primary"
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
