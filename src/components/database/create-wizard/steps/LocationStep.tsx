import { FolderOpen } from "lucide-react";
import { Controller, type Control } from "react-hook-form";
import { save } from "@tauri-apps/plugin-dialog";
import type { CreateDatabaseFormValues } from "@/lib/formTypes";
import {
  Field,
  FieldDescription,
  FieldError,
  FieldLabel,
} from "@/components/ui/field";
import {
  InputGroup,
  InputGroupAddon,
  InputGroupButton,
  InputGroupInput,
  InputGroupText,
} from "@/components/ui/input-group";

interface LocationStepProps {
  control: Control<CreateDatabaseFormValues>;
  disabled?: boolean;
}

function getFilenameFromPath(path: string | undefined): string {
  if (!path) return "";
  const parts = path.split(/[/\\]/);
  return parts[parts.length - 1] || "";
}

export function LocationStep({ control, disabled }: LocationStepProps) {
  async function handleSelectLocation(
    onChange: (value: string) => void,
    currentValue: string
  ) {
    try {
      const suggestedName = currentValue
        ? getFilenameFromPath(currentValue)
        : "NewDatabase.kdbx";

      const file = await save({
        title: "Choose Database Location",
        filters: [{ name: "KeePass Database", extensions: ["kdbx"] }],
        defaultPath: suggestedName,
      });

      if (file) {
        // Ensure .kdbx extension
        let path = file as string;
        if (!path.toLowerCase().endsWith(".kdbx")) {
          path = `${path}.kdbx`;
        }
        onChange(path);
      }
    } catch {
      // User cancelled or error - ignore
    }
  }

  return (
    <Field>
      <FieldLabel htmlFor="filePath">Database File Location</FieldLabel>
      <FieldDescription>
        Choose where to save your new password database. The file will be
        created with a .kdbx extension.
      </FieldDescription>

      <Controller
        name="filePath"
        control={control}
        render={({ field, fieldState }) => (
          <>
            <InputGroup>
              <InputGroupAddon
                align="block-start"
                className="border-b cursor-pointer hover:bg-muted/50 transition-colors"
                onClick={() =>
                  !disabled && handleSelectLocation(field.onChange, field.value)
                }
              >
                <InputGroupText className="font-mono font-medium">
                  <FolderOpen className="size-4" />
                  {getFilenameFromPath(field.value) ||
                    "Click to select location..."}
                </InputGroupText>
              </InputGroupAddon>

              <InputGroupInput
                {...field}
                id={field.name}
                aria-invalid={fieldState.invalid}
                placeholder="Or type the full path here..."
                disabled={disabled}
                className="py-3"
              />

              <InputGroupAddon align="block-end" className="border-t">
                <InputGroupButton
                  size="sm"
                  variant="ghost"
                  type="button"
                  onClick={() =>
                    handleSelectLocation(field.onChange, field.value)
                  }
                  disabled={disabled}
                >
                  Browse...
                </InputGroupButton>
              </InputGroupAddon>
            </InputGroup>

            {fieldState.error && (
              <FieldError>{fieldState.error.message}</FieldError>
            )}
          </>
        )}
      />
    </Field>
  );
}
