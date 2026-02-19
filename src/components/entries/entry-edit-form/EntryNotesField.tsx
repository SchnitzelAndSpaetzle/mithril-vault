import { Controller, type Control } from "react-hook-form";
import { Textarea } from "@/components/ui/textarea";
import { Field, FieldLabel } from "@/components/ui/field";
import type { EntryFormValues } from "@/lib/formTypes";

interface EntryNotesFieldProps {
  control: Control<EntryFormValues>;
  isPending: boolean;
}

export function EntryNotesField({ control, isPending }: EntryNotesFieldProps) {
  return (
    <Field>
      <FieldLabel htmlFor="notes">Notes</FieldLabel>
      <Controller
        name="notes"
        control={control}
        render={({ field }) => (
          <Textarea
            {...field}
            id="notes"
            placeholder="Additional notes..."
            disabled={isPending}
            rows={4}
          />
        )}
      />
    </Field>
  );
}
