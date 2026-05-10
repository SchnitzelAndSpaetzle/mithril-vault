import { createElement } from "react";
import { useTranslation } from "react-i18next";
import { type Control, Controller, useWatch } from "react-hook-form";
import { Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Field, FieldError, FieldLabel } from "@/components/ui/field";
import { getKeepassIcon } from "@/lib/keepass-icons";
import { IconPickerPopover } from "@/components/entries/IconPickerPopover";
import type { EntryFormValues } from "@/lib/formTypes";
import type { CustomIconMap } from "@/lib/types";

interface EntryTitleFieldProps {
  control: Control<EntryFormValues>;
  isPending: boolean;
  autoFocus?: boolean;
  isEditMode: boolean;
  hasCustomIcon: boolean;
  canFetchFavicon: boolean;
  isFetchingFavicon: boolean;
  isClearingCustomIcon: boolean;
  customIcons: CustomIconMap;
  onIconChange: (iconId: number) => void;
  onCustomIconChange: (iconUuid: string) => void;
  onFetchFavicon: () => Promise<void> | void;
  onClearCustomIcon: () => Promise<void> | void;
}

export function EntryTitleField({
  control,
  isPending,
  autoFocus,
  isEditMode,
  hasCustomIcon,
  canFetchFavicon,
  isFetchingFavicon,
  isClearingCustomIcon,
  customIcons,
  onIconChange,
  onCustomIconChange,
  onFetchFavicon,
  onClearCustomIcon,
}: EntryTitleFieldProps) {
  const { t } = useTranslation();
  const showFaviconActions = isEditMode;
  const iconId = useWatch({ control, name: "iconId" }) ?? 0;
  const customIconUuid = useWatch({ control, name: "customIconUuid" }) ?? null;
  const selectedCustomIcon = customIconUuid
    ? customIcons[customIconUuid]
    : null;

  return (
    <Field>
      <FieldLabel htmlFor="title">{t("entries.form.title")}</FieldLabel>
      <div className="space-y-2">
        <div className="flex items-center gap-2">
          <IconPickerPopover
            selectedIconId={iconId}
            selectedCustomIconUuid={customIconUuid}
            customIcons={customIcons}
            onSelect={onIconChange}
            onSelectCustomIcon={onCustomIconChange}
          >
            <Button
              type="button"
              variant="outline"
              size="icon-sm"
              aria-label={t("entries.form.chooseIcon")}
            >
              {selectedCustomIcon ? (
                <img
                  src={`data:${selectedCustomIcon.mimeType};base64,${selectedCustomIcon.data}`}
                  alt=""
                  className="size-4 object-contain"
                />
              ) : (
                createElement(getKeepassIcon(iconId), {
                  className: "size-4",
                })
              )}
            </Button>
          </IconPickerPopover>
          <Controller
            name="title"
            control={control}
            render={({ field, fieldState }) => (
              <div className="flex-1">
                <Input
                  {...field}
                  id="title"
                  autoFocus={autoFocus}
                  aria-invalid={fieldState.invalid}
                  placeholder={t("entries.form.titlePlaceholder")}
                  disabled={isPending}
                />
                {fieldState.error && (
                  <FieldError>{fieldState.error.message}</FieldError>
                )}
              </div>
            )}
          />
        </div>
        {showFaviconActions && (
          <div className="flex flex-wrap items-center gap-2">
            <Button
              type="button"
              variant="outline"
              size="sm"
              disabled={isPending || !canFetchFavicon || isFetchingFavicon}
              onClick={() => void onFetchFavicon()}
            >
              {isFetchingFavicon ? (
                <Loader2 className="mr-2 size-4 animate-spin" />
              ) : null}
              {hasCustomIcon
                ? t("entries.form.refreshFavicon")
                : t("entries.form.fetchFavicon")}
            </Button>
            <Button
              type="button"
              variant="outline"
              size="sm"
              disabled={isPending || !hasCustomIcon || isClearingCustomIcon}
              onClick={() => void onClearCustomIcon()}
            >
              {isClearingCustomIcon ? (
                <Loader2 className="mr-2 size-4 animate-spin" />
              ) : null}
              {t("entries.form.clearCustomIcon")}
            </Button>
          </div>
        )}
      </div>
    </Field>
  );
}
