import { useTranslation } from "react-i18next";
import { type Control, useController } from "react-hook-form";
import { Checkbox } from "@/components/ui/checkbox";
import { DateTimePicker } from "@/components/ui/date-time-picker";
import { Field, FieldError, FieldLabel } from "@/components/ui/field";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  EXPIRY_PRESETS,
  type ExpiryPreset,
  resolveExpiryPreset,
} from "@/lib/entry-expiry";
import type { EntryFormValues } from "@/lib/formTypes";

interface EntryExpiryFieldProps {
  control: Control<EntryFormValues>;
  isPending: boolean;
}

export function EntryExpiryField({
  control,
  isPending,
}: Readonly<EntryExpiryFieldProps>) {
  const { t } = useTranslation();
  const { field: expiresField } = useController({ name: "expires", control });
  const { field: expiryTimeField, fieldState } = useController({
    name: "expiryTime",
    control,
  });

  function handleToggle(checked: boolean) {
    expiresField.onChange(checked);
    // First time expiry is enabled, seed a sensible starting value (1 year).
    // This is a transient field value, not a persisted preference.
    if (checked && expiryTimeField.value === null) {
      expiryTimeField.onChange(resolveExpiryPreset("1y", new Date()));
    }
  }

  function handlePresetChange(preset: string) {
    expiryTimeField.onChange(
      resolveExpiryPreset(preset as ExpiryPreset, new Date())
    );
  }

  return (
    <Field>
      <div className="flex items-center gap-2">
        <Checkbox
          id="expires"
          checked={expiresField.value}
          onCheckedChange={(checked) => handleToggle(checked === true)}
          disabled={isPending}
          aria-label={t("entries.form.expiry.label")}
        />
        <FieldLabel htmlFor="expires">
          {t("entries.form.expiry.label")}
        </FieldLabel>
      </div>
      {expiresField.value && (
        <div className="flex flex-col gap-2">
          <Select onValueChange={handlePresetChange} disabled={isPending}>
            <SelectTrigger
              className="w-full"
              aria-label={t("entries.form.expiry.presetLabel")}
            >
              <SelectValue
                placeholder={t("entries.form.expiry.presetPlaceholder")}
              />
            </SelectTrigger>
            <SelectContent>
              {EXPIRY_PRESETS.map((preset) => (
                <SelectItem key={preset} value={preset}>
                  {t(`entries.form.expiry.presets.${preset}`)}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <DateTimePicker
            id="expiryTime"
            value={expiryTimeField.value ?? undefined}
            onChange={(date) => expiryTimeField.onChange(date ?? null)}
            placeholder={t("entries.form.expiry.pickPlaceholder")}
            disabled={isPending}
          />
          {fieldState.error && (
            <FieldError>{fieldState.error.message}</FieldError>
          )}
        </div>
      )}
    </Field>
  );
}
