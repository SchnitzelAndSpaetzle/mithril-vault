import { Controller, type Control } from "react-hook-form";
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
  return (
    <FieldGroup>
      <Field>
        <FieldLabel htmlFor="name">Database Name</FieldLabel>
        <FieldDescription>
          A name to identify your database. This will be displayed in the app.
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
                placeholder="e.g., Personal, Work, Family..."
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
          Description{" "}
          <span className="text-muted-foreground font-normal">(optional)</span>
        </FieldLabel>
        <FieldDescription>
          Add a description to help you remember the purpose of this database.
        </FieldDescription>

        <Controller
          name="description"
          control={control}
          render={({ field }) => (
            <Textarea
              {...field}
              id={field.name}
              placeholder="e.g., Contains all my personal accounts..."
              disabled={disabled}
              rows={3}
            />
          )}
        />
      </Field>
    </FieldGroup>
  );
}
