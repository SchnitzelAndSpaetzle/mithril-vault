import { useTranslation } from "react-i18next";
import { type Control, Controller } from "react-hook-form";
import { Input } from "@/components/ui/input";
import { Field, FieldError, FieldLabel } from "@/components/ui/field";
import type { EntryFormValues } from "@/lib/formTypes";

interface EntryUrlFieldProps {
  control: Control<EntryFormValues>;
  isPending: boolean;
}

export function EntryUrlField({ control, isPending }: EntryUrlFieldProps) {
  const { t } = useTranslation();

  return (
    <Field>
      <FieldLabel htmlFor="url">{t("entries.form.url")}</FieldLabel>
      <Controller
        name="url"
        control={control}
        render={({ field, fieldState }) => (
          <>
            <Input
              {...field}
              id="url"
              aria-invalid={fieldState.invalid}
              placeholder={t("entries.form.urlPlaceholder")}
              disabled={isPending}
            />
            {fieldState.error && (
              <FieldError>{fieldState.error.message}</FieldError>
            )}
          </>
        )}
      />
    </Field>
  );
}
