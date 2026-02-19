import { useFieldArray } from "react-hook-form";
import { Plus } from "lucide-react";
import { Button } from "@/components/ui/button";
import { CustomFieldRow } from "./CustomFieldRow";
import type { CustomFieldsEditorProps } from "./types";

export function CustomFieldsEditor({
  control,
  disabled = false,
}: CustomFieldsEditorProps) {
  const { fields, append, remove } = useFieldArray({
    control,
    name: "customFields",
  });

  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between">
        <span className="text-sm font-medium">Custom Fields</span>
        <Button
          type="button"
          variant="outline"
          size="sm"
          onClick={() => append({ key: "", value: "", isProtected: false })}
          disabled={disabled}
        >
          <Plus className="mr-1 size-3.5" />
          Add Field
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
        <p className="text-xs text-muted-foreground">No custom fields.</p>
      )}
    </div>
  );
}
