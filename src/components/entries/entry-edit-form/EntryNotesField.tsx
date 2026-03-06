import { useTranslation } from "react-i18next";
import { type Control, Controller } from "react-hook-form";
import { Textarea } from "@/components/ui/textarea";
import { Field, FieldLabel } from "@/components/ui/field";
import type { EntryFormValues } from "@/lib/formTypes";

interface EntryNotesFieldProps {
  control: Control<EntryFormValues>;
  isPending: boolean;
}

export function EntryNotesField({ control, isPending }: EntryNotesFieldProps) {
  const { t } = useTranslation();

  return (
    <Field>
      <FieldLabel htmlFor="notes">{t("entries.form.notes")}</FieldLabel>
      <Controller
        name="notes"
        control={control}
        render={({ field }) => (
          <Textarea
            {...field}
            id="notes"
            placeholder={t("entries.form.notesPlaceholder")}
            disabled={isPending}
            rows={4}
          />
        )}
      />
    </Field>
  );
}
