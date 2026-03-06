import { useFieldArray } from "react-hook-form";
import { useTranslation } from "react-i18next";
import { Plus } from "lucide-react";
import { Button } from "@/components/ui/button";
import { CustomFieldRow } from "./CustomFieldRow";
import type { CustomFieldsEditorProps } from "./types";

export function CustomFieldsEditor({
  control,
  disabled = false,
}: CustomFieldsEditorProps) {
  const { t } = useTranslation();
  const { fields, append, remove } = useFieldArray({
    control,
    name: "customFields",
  });

  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between">
        <span className="text-sm font-medium">
          {t("entries.form.customFields")}
        </span>
        <Button
          type="button"
          variant="outline"
          size="sm"
          onClick={() => append({ key: "", value: "", isProtected: false })}
          disabled={disabled}
        >
          <Plus className="mr-1 size-3.5" />
          {t("entries.form.addField")}
        </Button>
      </div>

      {fields.map((field, index) => (
        <CustomFieldRow
          key={field.id}
          index={index}
          control={control}
          onRemove={() => remove(index)}
          disabled={disabled}
        />
      ))}

      {fields.length === 0 && (
        <p className="text-xs text-muted-foreground">
          {t("entries.form.noCustomFields")}
        </p>
      )}
    </div>
  );
}
