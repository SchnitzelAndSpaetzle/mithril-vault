import { FolderOpen } from "lucide-react";
import { type Control, Controller } from "react-hook-form";
import { useTranslation } from "react-i18next";
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
import { getFilenameFromPath } from "@/lib/utils";

interface LocationStepProps {
  control: Control<CreateDatabaseFormValues>;
  disabled?: boolean;
}

export function LocationStep({ control, disabled }: LocationStepProps) {
  const { t } = useTranslation();

  async function handleSelectLocation(
    onChange: (value: string) => void,
    currentValue: string
  ) {
    try {
      const suggestedName = currentValue
        ? getFilenameFromPath(currentValue)
        : "NewDatabase.kdbx";

      const file = await save({
        title: t("createDatabase.location.dialogTitle"),
        filters: [{ name: "KeePass Database", extensions: ["kdbx"] }],
        defaultPath: suggestedName,
      });

      if (file) {
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
      <FieldLabel htmlFor="filePath">
        {t("createDatabase.location.label")}
      </FieldLabel>
      <FieldDescription>
        {t("createDatabase.location.description")}
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
                    t("createDatabase.location.clickToSelect")}
                </InputGroupText>
              </InputGroupAddon>

              <InputGroupInput
                {...field}
                id={field.name}
                aria-invalid={fieldState.invalid}
                placeholder={t("createDatabase.location.typePath")}
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
                  {t("createDatabase.location.browse")}
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
