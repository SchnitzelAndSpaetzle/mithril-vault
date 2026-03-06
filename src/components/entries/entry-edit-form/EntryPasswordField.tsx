import { useState } from "react";
import { useTranslation } from "react-i18next";
import { type Control, Controller } from "react-hook-form";
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
  const { t } = useTranslation();
  const [showPassword, setShowPassword] = useState(false);

  return (
    <Field>
      <FieldLabel htmlFor="password">{t("entries.form.password")}</FieldLabel>
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
                placeholder={t("entries.form.passwordPlaceholder")}
                autoComplete="new-password"
                disabled={isPending}
              />
              <InputGroupAddon align="inline-end">
                <InputGroupButton
                  variant="ghost"
                  size="icon-xs"
                  type="button"
                  aria-label={
                    showPassword
                      ? t("entries.form.hidePassword")
                      : t("entries.form.showPassword")
                  }
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
                    aria-label={t("entries.form.generatePassword")}
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
