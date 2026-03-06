import { useTranslation } from "react-i18next";
import { type Control, Controller } from "react-hook-form";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Field, FieldLabel } from "@/components/ui/field";
import { useGroups } from "@/hooks/use-groups";
import type { EntryFormValues } from "@/lib/formTypes";
import { flattenGroups } from "@/lib/group-utils";

interface EntryGroupFieldProps {
  control: Control<EntryFormValues>;
  dbId: string;
  isPending: boolean;
}

export function EntryGroupField({
  control,
  dbId,
  isPending,
}: EntryGroupFieldProps) {
  const { t } = useTranslation();
  const { data: groups } = useGroups(dbId);
  const flatGroups = groups ? flattenGroups(groups) : [];

  if (flatGroups.length <= 1) return null;

  return (
    <Field>
      <FieldLabel htmlFor="groupId">{t("entries.form.group")}</FieldLabel>
      <Controller
        name="groupId"
        control={control}
        render={({ field }) => (
          <Select
            value={field.value ?? ""}
            onValueChange={field.onChange}
            disabled={isPending}
          >
            <SelectTrigger className="w-full" id="groupId">
              <SelectValue placeholder={t("entries.form.selectGroup")} />
            </SelectTrigger>
            <SelectContent>
              {flatGroups.map((group) => (
                <SelectItem key={group.id} value={group.id}>
                  <span style={{ paddingLeft: `${group.depth * 12}px` }}>
                    {group.name}
                  </span>
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        )}
      />
    </Field>
  );
}
