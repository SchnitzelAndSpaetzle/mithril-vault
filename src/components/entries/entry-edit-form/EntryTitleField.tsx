import { createElement } from "react";
import { type Control, Controller } from "react-hook-form";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Field, FieldError, FieldLabel } from "@/components/ui/field";
import { getKeepassIcon } from "@/lib/keepass-icons";
import { IconPickerPopover } from "@/components/entries/IconPickerPopover";
import type { EntryFormValues } from "@/lib/formTypes";

interface EntryTitleFieldProps {
  control: Control<EntryFormValues>;
  isPending: boolean;
  autoFocus?: boolean;
}

export function EntryTitleField({
  control,
  isPending,
  autoFocus,
}: EntryTitleFieldProps) {
  return (
    <Field>
      <FieldLabel htmlFor="title">Title</FieldLabel>
      <div className="flex items-center gap-2">
        <Controller
          name="iconId"
          control={control}
          render={({ field }) => (
            <IconPickerPopover
              selectedIconId={field.value}
              onSelect={field.onChange}
            >
              <Button
                type="button"
                variant="outline"
                size="icon-sm"
                aria-label="Choose icon"
              >
                {createElement(getKeepassIcon(field.value), {
                  className: "size-4",
                })}
              </Button>
            </IconPickerPopover>
          )}
        />
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
                placeholder="Entry title"
                disabled={isPending}
              />
              {fieldState.error && (
                <FieldError>{fieldState.error.message}</FieldError>
              )}
            </div>
          )}
        />
      </div>
    </Field>
  );
}
