import { useController } from "react-hook-form";
import { Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { CustomFieldValueInput } from "./CustomFieldValueInput";
import type { CustomFieldRowProps } from "./types";

export function CustomFieldRow({
  index,
  control,
  onRemove,
  disabled,
}: CustomFieldRowProps) {
  const { field: keyField } = useController({
    control,
    name: `customFields.${index}.key`,
  });

  return (
    <div className="flex items-start gap-2">
      <Input
        {...keyField}
        placeholder="Field name"
        disabled={disabled}
        className="h-9 w-1/3"
      />

      <CustomFieldValueInput
        index={index}
        control={control}
        disabled={disabled}
      />

      <Button
        type="button"
        variant="ghost"
        size="icon-sm"
        aria-label="Remove custom field"
        onClick={onRemove}
        disabled={disabled}
      >
        <Trash2 className="size-3.5" />
      </Button>
    </div>
  );
}
