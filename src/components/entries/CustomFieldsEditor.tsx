import { useState } from "react";
import { type Control, Controller, useFieldArray } from "react-hook-form";
import { Eye, EyeOff, Lock, LockOpen, Plus, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  InputGroup,
  InputGroupAddon,
  InputGroupButton,
  InputGroupInput,
} from "@/components/ui/input-group";
import type { EntryFormValues } from "@/lib/formTypes";

interface CustomFieldsEditorProps {
  control: Control<EntryFormValues>;
  disabled?: boolean;
}

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

function CustomFieldRow({
  index,
  control,
  onRemove,
  disabled,
}: {
  index: number;
  control: Control<EntryFormValues>;
  onRemove: () => void;
  disabled: boolean;
}) {
  const [showValue, setShowValue] = useState(false);

  return (
    <div className="flex items-start gap-2">
      {/* Key input */}
      <Controller
        name={`customFields.${index}.key`}
        control={control}
        render={({ field }) => (
          <Input
            {...field}
            placeholder="Field name"
            disabled={disabled}
            className="h-9 w-1/3"
          />
        )}
      />

      {/* Value input with show/hide for protected */}
      <Controller
        name={`customFields.${index}.value`}
        control={control}
        render={({ field: valueField }) => (
          <Controller
            name={`customFields.${index}.isProtected`}
            control={control}
            render={({ field: protectedField }) => (
              <InputGroup className="flex-1">
                <InputGroupInput
                  {...valueField}
                  type={
                    protectedField.value && !showValue ? "password" : "text"
                  }
                  placeholder="Value"
                  disabled={disabled}
                />
                <InputGroupAddon align="inline-end">
                  {protectedField.value && (
                    <InputGroupButton
                      variant="ghost"
                      size="icon-xs"
                      type="button"
                      aria-label={showValue ? "Hide value" : "Show value"}
                      onClick={() => setShowValue((prev) => !prev)}
                      disabled={disabled}
                    >
                      {showValue ? (
                        <EyeOff className="size-3.5" />
                      ) : (
                        <Eye className="size-3.5" />
                      )}
                    </InputGroupButton>
                  )}
                  <InputGroupButton
                    variant="ghost"
                    size="icon-xs"
                    type="button"
                    aria-label={
                      protectedField.value ? "Unprotect field" : "Protect field"
                    }
                    onClick={() =>
                      protectedField.onChange(!protectedField.value)
                    }
                    disabled={disabled}
                  >
                    {protectedField.value ? (
                      <Lock className="size-3.5" />
                    ) : (
                      <LockOpen className="size-3.5 text-muted-foreground" />
                    )}
                  </InputGroupButton>
                </InputGroupAddon>
              </InputGroup>
            )}
          />
        )}
      />

      {/* Remove button */}
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
