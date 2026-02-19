import { Controller, type Control } from "react-hook-form";
import { TagInput } from "@/components/entries/TagInput";
import { Field, FieldLabel } from "@/components/ui/field";
import type { EntryFormValues } from "@/lib/formTypes";

interface EntryTagsFieldProps {
  control: Control<EntryFormValues>;
  isPending: boolean;
  availableTags: string[] | undefined;
}

export function EntryTagsField({
  control,
  isPending,
  availableTags,
}: EntryTagsFieldProps) {
  return (
    <Field>
      <FieldLabel>Tags</FieldLabel>
      <Controller
        name="tags"
        control={control}
        render={({ field }) => (
          <TagInput
            value={field.value}
            onChange={field.onChange}
            disabled={isPending}
            suggestions={availableTags ?? []}
          />
        )}
      />
    </Field>
  );
}
