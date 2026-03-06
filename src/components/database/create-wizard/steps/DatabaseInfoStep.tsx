import { type Control, Controller } from "react-hook-form";
import { useTranslation } from "react-i18next";
import type { CreateDatabaseFormValues } from "@/lib/formTypes";
import {
  Field,
  FieldDescription,
  FieldError,
  FieldGroup,
  FieldLabel,
} from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";

interface DatabaseInfoStepProps {
  control: Control<CreateDatabaseFormValues>;
  disabled?: boolean;
}

export function DatabaseInfoStep({ control, disabled }: DatabaseInfoStepProps) {
  const { t } = useTranslation();

  return (
    <FieldGroup>
      <Field>
        <FieldLabel htmlFor="name">
          {t("createDatabase.info.nameLabel")}
        </FieldLabel>
        <FieldDescription>
          {t("createDatabase.info.nameDescription")}
        </FieldDescription>

        <Controller
          name="name"
          control={control}
          render={({ field, fieldState }) => (
            <>
              <Input
                {...field}
                id={field.name}
                aria-invalid={fieldState.invalid}
                placeholder={t("createDatabase.info.namePlaceholder")}
                disabled={disabled}
              />
              {fieldState.error && (
                <FieldError>{fieldState.error.message}</FieldError>
              )}
            </>
          )}
        />
      </Field>

      <Field>
        <FieldLabel htmlFor="description">
          {t("createDatabase.info.descriptionLabel")}{" "}
          <span className="text-muted-foreground font-normal">
            {t("createDatabase.info.descriptionOptional")}
          </span>
        </FieldLabel>
        <FieldDescription>
          {t("createDatabase.info.descriptionDescription")}
        </FieldDescription>

        <Controller
          name="description"
          control={control}
          render={({ field }) => (
            <Textarea
              {...field}
              id={field.name}
              placeholder={t("createDatabase.info.descriptionPlaceholder")}
              disabled={disabled}
              rows={3}
            />
          )}
        />
      </Field>
    </FieldGroup>
  );
}
