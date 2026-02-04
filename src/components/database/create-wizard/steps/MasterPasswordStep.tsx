import { Eye, EyeClosed, ShieldAlert } from "lucide-react";
import { useState } from "react";
import { Controller, type Control, useWatch } from "react-hook-form";
import type { CreateDatabaseFormValues } from "@/lib/formTypes";
import {
  Field,
  FieldDescription,
  FieldError,
  FieldGroup,
  FieldLabel,
} from "@/components/ui/field";
import {
  InputGroup,
  InputGroupAddon,
  InputGroupButton,
  InputGroupInput,
} from "@/components/ui/input-group";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { PasswordStrengthIndicator } from "../PasswordStrengthIndicator";

interface MasterPasswordStepProps {
  control: Control<CreateDatabaseFormValues>;
  disabled?: boolean;
}

export function MasterPasswordStep({
  control,
  disabled,
}: MasterPasswordStepProps) {
  const [showPassword, setShowPassword] = useState(false);
  const [showConfirmPassword, setShowConfirmPassword] = useState(false);

  const password = useWatch({ control, name: "password" });

  return (
    <FieldGroup>
      <Field>
        <FieldLabel htmlFor="password">Master Password</FieldLabel>
        <FieldDescription>
          Choose a strong password that you can remember. This password encrypts
          your entire database.
        </FieldDescription>

        <Controller
          name="password"
          control={control}
          render={({ field, fieldState }) => (
            <>
              <InputGroup>
                <InputGroupInput
                  {...field}
                  id={field.name}
                  aria-invalid={fieldState.invalid}
                  type={showPassword ? "text" : "password"}
                  placeholder="Enter your master password..."
                  autoComplete="new-password"
                  disabled={disabled}
                />
                <InputGroupAddon align="inline-end">
                  <InputGroupButton
                    variant="ghost"
                    aria-label={
                      showPassword ? "Hide password" : "Show password"
                    }
                    size="icon-xs"
                    type="button"
                    onClick={() => setShowPassword((prev) => !prev)}
                    disabled={disabled}
                  >
                    {showPassword ? <Eye /> : <EyeClosed />}
                  </InputGroupButton>
                </InputGroupAddon>
              </InputGroup>

              <PasswordStrengthIndicator password={field.value || ""} />

              {fieldState.error && (
                <FieldError>{fieldState.error.message}</FieldError>
              )}
            </>
          )}
        />
      </Field>

      <Field>
        <FieldLabel htmlFor="confirmPassword">Confirm Password</FieldLabel>
        <FieldDescription>
          Re-enter your password to confirm it.
        </FieldDescription>

        <Controller
          name="confirmPassword"
          control={control}
          render={({ field, fieldState }) => (
            <>
              <InputGroup>
                <InputGroupInput
                  {...field}
                  id={field.name}
                  aria-invalid={fieldState.invalid}
                  type={showConfirmPassword ? "text" : "password"}
                  placeholder="Confirm your master password..."
                  autoComplete="new-password"
                  disabled={disabled}
                />
                <InputGroupAddon align="inline-end">
                  <InputGroupButton
                    variant="ghost"
                    aria-label={
                      showConfirmPassword ? "Hide password" : "Show password"
                    }
                    size="icon-xs"
                    type="button"
                    onClick={() => setShowConfirmPassword((prev) => !prev)}
                    disabled={disabled}
                  >
                    {showConfirmPassword ? <Eye /> : <EyeClosed />}
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

      {password && (
        <Alert
          variant="default"
          className="border-amber-500/50 bg-amber-50/50 dark:bg-amber-950/20"
        >
          <ShieldAlert className="size-4 text-amber-600" />
          <AlertTitle className="text-amber-800 dark:text-amber-400">
            Important Security Notice
          </AlertTitle>
          <AlertDescription className="text-amber-700 dark:text-amber-300">
            Your master password cannot be recovered if forgotten. Store it
            safely or use a memorable passphrase.
          </AlertDescription>
        </Alert>
      )}
    </FieldGroup>
  );
}
