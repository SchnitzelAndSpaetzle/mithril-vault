import { useState } from "react";
import { useController } from "react-hook-form";
import { Eye, EyeOff, Lock, LockOpen } from "lucide-react";
import {
  InputGroup,
  InputGroupAddon,
  InputGroupButton,
  InputGroupInput,
} from "@/components/ui/input-group";
import type { CustomFieldValueInputProps } from "./types";

export function CustomFieldValueInput({
  index,
  control,
  disabled,
}: CustomFieldValueInputProps) {
  const [showValue, setShowValue] = useState(false);

  const { field: valueField } = useController({
    control,
    name: `customFields.${index}.value`,
  });
  const { field: protectedField } = useController({
    control,
    name: `customFields.${index}.isProtected`,
  });

  return (
    <InputGroup className="flex-1">
      <InputGroupInput
        {...valueField}
        type={protectedField.value && !showValue ? "password" : "text"}
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
          onClick={() => protectedField.onChange(!protectedField.value)}
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
  );
}
