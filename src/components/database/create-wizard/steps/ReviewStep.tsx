import { Check, FileText, FolderOpen, KeyRound, Settings } from "lucide-react";
import { type Control, Controller, useWatch } from "react-hook-form";
import { useTranslation } from "react-i18next";
import type { CreateDatabaseFormValues } from "@/lib/formTypes";
import { Checkbox } from "@/components/ui/checkbox";
import { Label } from "@/components/ui/label";
import { cn, getFilenameFromPath } from "@/lib/utils";

interface ReviewStepProps {
  control: Control<CreateDatabaseFormValues>;
  disabled?: boolean;
}

interface ReviewItemProps {
  icon: React.ReactNode;
  label: string;
  value: string | React.ReactNode;
  variant?: "default" | "success";
}

function ReviewItem({
  icon,
  label,
  value,
  variant = "default",
}: ReviewItemProps) {
  return (
    <div className="flex items-start gap-3 py-3">
      <div
        className={cn(
          "rounded-full p-2",
          variant === "success"
            ? "bg-green-100 text-green-600 dark:bg-green-950/50 dark:text-green-400"
            : "bg-muted text-muted-foreground"
        )}
      >
        {icon}
      </div>
      <div className="flex-1 min-w-0">
        <div className="text-sm text-muted-foreground">{label}</div>
        <div className="font-medium truncate">{value}</div>
      </div>
    </div>
  );
}

export function ReviewStep({ control, disabled }: ReviewStepProps) {
  const { t } = useTranslation();
  const filePath = useWatch({ control, name: "filePath" });
  const name = useWatch({ control, name: "name" });
  const description = useWatch({ control, name: "description" });
  const password = useWatch({ control, name: "password" });
  const keyfileMode = useWatch({ control, name: "keyfileMode" });
  const keyfilePath = useWatch({ control, name: "keyfilePath" });

  const hasPassword = Boolean(password);
  const hasKeyfile = keyfileMode !== "none" && Boolean(keyfilePath);

  return (
    <div className="space-y-6">
      <div className="rounded-lg border divide-y">
        <ReviewItem
          icon={<FolderOpen className="size-4" />}
          label={t("createDatabase.review.location")}
          value={
            <span className="font-mono text-sm">
              {getFilenameFromPath(filePath)}
            </span>
          }
        />

        <ReviewItem
          icon={<FileText className="size-4" />}
          label={t("createDatabase.review.databaseName")}
          value={name || t("createDatabase.review.noName")}
        />

        {description && (
          <ReviewItem
            icon={<FileText className="size-4" />}
            label={t("createDatabase.review.description")}
            value={description}
          />
        )}

        <ReviewItem
          icon={<Check className="size-4" />}
          label={t("createDatabase.review.masterPassword")}
          value={
            hasPassword
              ? t("createDatabase.review.passwordSet")
              : t("createDatabase.review.noPassword")
          }
          variant={hasPassword ? "success" : "default"}
        />

        <ReviewItem
          icon={<KeyRound className="size-4" />}
          label={t("createDatabase.review.keyFile")}
          value={
            hasKeyfile ? (
              <span className="flex items-center gap-2">
                {keyfileMode === "generate"
                  ? t("createDatabase.review.willGenerate")
                  : ""}
                <span className="font-mono text-sm">
                  {getFilenameFromPath(keyfilePath)}
                </span>
              </span>
            ) : (
              t("createDatabase.review.noKeyFile")
            )
          }
          variant={hasKeyfile ? "success" : "default"}
        />
      </div>

      <div className="rounded-lg border p-4">
        <div className="flex items-start gap-3">
          <div className="rounded-full bg-muted p-2 text-muted-foreground">
            <Settings className="size-4" />
          </div>
          <div className="flex-1 space-y-4">
            <div>
              <div className="font-medium">
                {t("createDatabase.review.additionalOptions")}
              </div>
              <div className="text-sm text-muted-foreground">
                {t("createDatabase.review.configureDescription")}
              </div>
            </div>

            <Controller
              name="createDefaultGroups"
              control={control}
              render={({ field }) => (
                <div className="flex items-center space-x-2">
                  <Checkbox
                    id="createDefaultGroups"
                    checked={field.value}
                    onCheckedChange={(checked) =>
                      field.onChange(checked === true)
                    }
                    disabled={disabled}
                  />
                  <Label
                    htmlFor="createDefaultGroups"
                    className="text-sm font-normal cursor-pointer"
                  >
                    {t("createDatabase.review.createDefaultGroups")}
                  </Label>
                </div>
              )}
            />
          </div>
        </div>
      </div>

      <div className="rounded-lg bg-muted/50 p-4 text-sm text-muted-foreground">
        <p>
          <strong>{t("createDatabase.review.securitySummary")}</strong>{" "}
          {t("createDatabase.review.encryptedWith")}{" "}
          {hasPassword && hasKeyfile
            ? t("createDatabase.review.twoFactor")
            : hasPassword
              ? t("createDatabase.review.passwordOnly")
              : hasKeyfile
                ? t("createDatabase.review.keyFileOnly")
                : t("createDatabase.review.noCredentials")}
          .
        </p>
      </div>
    </div>
  );
}
