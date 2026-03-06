import { Eye, EyeClosed, ShieldAlert } from "lucide-react";
import { useState } from "react";
import { type Control, Controller, useWatch } from "react-hook-form";
import { useTranslation } from "react-i18next";
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
  const { t } = useTranslation();
  const [showPassword, setShowPassword] = useState(false);
  const [showConfirmPassword, setShowConfirmPassword] = useState(false);

  const password = useWatch({ control, name: "password" });

  return (
    <FieldGroup>
      <Field>
        <FieldLabel htmlFor="password">
          {t("createDatabase.password.label")}
        </FieldLabel>
        <FieldDescription>
          {t("createDatabase.password.description")}
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
                  placeholder={t("createDatabase.password.placeholder")}
                  autoComplete="new-password"
                  disabled={disabled}
                />
                <InputGroupAddon align="inline-end">
                  <InputGroupButton
                    variant="ghost"
                    aria-label={
                      showPassword
                        ? t("entries.form.hidePassword")
                        : t("entries.form.showPassword")
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
        <FieldLabel htmlFor="confirmPassword">
          {t("createDatabase.password.confirmLabel")}
        </FieldLabel>
        <FieldDescription>
          {t("createDatabase.password.confirmDescription")}
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
                  placeholder={t("createDatabase.password.confirmPlaceholder")}
                  autoComplete="new-password"
                  disabled={disabled}
                />
                <InputGroupAddon align="inline-end">
                  <InputGroupButton
                    variant="ghost"
                    aria-label={
                      showConfirmPassword
                        ? t("entries.form.hidePassword")
                        : t("entries.form.showPassword")
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
            {t("createDatabase.password.securityNoticeTitle")}
          </AlertTitle>
          <AlertDescription className="text-amber-700 dark:text-amber-300">
            {t("createDatabase.password.securityNoticeDescription")}
          </AlertDescription>
        </Alert>
      )}
    </FieldGroup>
  );
}
