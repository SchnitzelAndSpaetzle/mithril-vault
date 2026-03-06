import { useTranslation } from "react-i18next";
import { type Control, Controller } from "react-hook-form";
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
  const { t } = useTranslation();

  return (
    <Field>
      <FieldLabel>{t("entries.form.tags")}</FieldLabel>
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
