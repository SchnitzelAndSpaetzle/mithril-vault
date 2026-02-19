import { Controller, type Control } from "react-hook-form";
import { Input } from "@/components/ui/input";
import { Field, FieldError, FieldLabel } from "@/components/ui/field";
import type { EntryFormValues } from "@/lib/formTypes";

interface EntryUrlFieldProps {
  control: Control<EntryFormValues>;
  isPending: boolean;
}

export function EntryUrlField({ control, isPending }: EntryUrlFieldProps) {
  return (
    <Field>
      <FieldLabel htmlFor="url">URL</FieldLabel>
      <Controller
        name="url"
        control={control}
        render={({ field, fieldState }) => (
          <>
            <Input
              {...field}
              id="url"
              aria-invalid={fieldState.invalid}
              placeholder="https://example.com"
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
