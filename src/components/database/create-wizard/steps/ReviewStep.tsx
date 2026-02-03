import { Check, FileText, FolderOpen, KeyRound, Settings } from "lucide-react";
import { type Control, Controller, useWatch } from "react-hook-form";
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
          label="Location"
          value={
            <span className="font-mono text-sm">
              {getFilenameFromPath(filePath)}
            </span>
          }
        />

        <ReviewItem
          icon={<FileText className="size-4" />}
          label="Database Name"
          value={name || "No database name provided"}
        />

        {description && (
          <ReviewItem
            icon={<FileText className="size-4" />}
            label="Description"
            value={description}
          />
        )}

        <ReviewItem
          icon={<Check className="size-4" />}
          label="Master Password"
          value={hasPassword ? "Password set" : "No password"}
          variant={hasPassword ? "success" : "default"}
        />

        <ReviewItem
          icon={<KeyRound className="size-4" />}
          label="Key File"
          value={
            hasKeyfile ? (
              <span className="flex items-center gap-2">
                {keyfileMode === "generate" ? "Will generate: " : ""}
                <span className="font-mono text-sm">
                  {getFilenameFromPath(keyfilePath)}
                </span>
              </span>
            ) : (
              "No key file"
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
              <div className="font-medium">Additional Options</div>
              <div className="text-sm text-muted-foreground">
                Configure how your database is created
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
                    Create default groups (General, Email, Banking, Social,
                    Work)
                  </Label>
                </div>
              )}
            />
          </div>
        </div>
      </div>

      <div className="rounded-lg bg-muted/50 p-4 text-sm text-muted-foreground">
        <p>
          <strong>Security Summary:</strong> Your database will be encrypted
          using{" "}
          {hasPassword && hasKeyfile
            ? "both a password and key file (two-factor)"
            : hasPassword
              ? "your master password"
              : hasKeyfile
                ? "your key file only"
                : "no credentials (please go back and set a password or key file)"}
          .
        </p>
      </div>
    </div>
  );
}
