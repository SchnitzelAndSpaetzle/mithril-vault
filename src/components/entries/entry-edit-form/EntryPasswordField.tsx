import { useState } from "react";
import { Controller, type Control } from "react-hook-form";
import { Dices, Eye, EyeClosed } from "lucide-react";
import { PasswordStrengthIndicator } from "@/components/database/create-wizard/PasswordStrengthIndicator";
import { PasswordGeneratorPopover } from "@/components/entries/PasswordGeneratorPopover";
import { Field, FieldError, FieldLabel } from "@/components/ui/field";
import {
  InputGroup,
  InputGroupAddon,
  InputGroupButton,
  InputGroupInput,
} from "@/components/ui/input-group";
import type { EntryFormValues } from "@/lib/formTypes";

interface EntryPasswordFieldProps {
  control: Control<EntryFormValues>;
  isPending: boolean;
  watchedPassword: string;
  onUseGeneratedPassword: (password: string) => void;
}

export function EntryPasswordField({
  control,
  isPending,
  watchedPassword,
  onUseGeneratedPassword,
}: EntryPasswordFieldProps) {
  const [showPassword, setShowPassword] = useState(false);

  return (
    <Field>
      <FieldLabel htmlFor="password">Password</FieldLabel>
      <Controller
        name="password"
        control={control}
        render={({ field, fieldState }) => (
          <>
            <InputGroup>
              <InputGroupInput
                {...field}
                id="password"
                aria-invalid={fieldState.invalid}
                type={showPassword ? "text" : "password"}
                placeholder="Enter password..."
                autoComplete="new-password"
                disabled={isPending}
              />
              <InputGroupAddon align="inline-end">
                <InputGroupButton
                  variant="ghost"
                  size="icon-xs"
                  type="button"
                  aria-label={showPassword ? "Hide password" : "Show password"}
                  onClick={() => setShowPassword((prev) => !prev)}
                  disabled={isPending}
                >
                  {showPassword ? <Eye /> : <EyeClosed />}
                </InputGroupButton>
                <PasswordGeneratorPopover
                  onUsePassword={onUseGeneratedPassword}
                >
                  <InputGroupButton
                    variant="ghost"
                    size="icon-xs"
                    type="button"
                    aria-label="Generate password"
                    disabled={isPending}
                  >
                    <Dices />
                  </InputGroupButton>
                </PasswordGeneratorPopover>
              </InputGroupAddon>
            </InputGroup>
            <PasswordStrengthIndicator password={watchedPassword} />
            {fieldState.error && (
              <FieldError>{fieldState.error.message}</FieldError>
            )}
          </>
        )}
      />
    </Field>
  );
}
