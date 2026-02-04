import { FolderOpen, KeyRound, Plus, ShieldAlert, X } from "lucide-react";
import { type Control, Controller, useWatch } from "react-hook-form";
import { open, save } from "@tauri-apps/plugin-dialog";
import type { CreateDatabaseFormValues, KeyfileMode } from "@/lib/formTypes";
import {
  Field,
  FieldDescription,
  FieldError,
  FieldGroup,
  FieldLabel,
} from "@/components/ui/field";
import { Button } from "@/components/ui/button";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { cn, getFilenameFromPath } from "@/lib/utils";

interface KeyFileStepProps {
  control: Control<CreateDatabaseFormValues>;
  setValue: (
    name: "keyfileMode" | "keyfilePath",
    value: KeyfileMode | string
  ) => void;
  disabled?: boolean;
}

interface KeyfileModeOptionProps {
  mode: KeyfileMode;
  currentMode: KeyfileMode;
  icon: React.ReactNode;
  title: string;
  description: string;
  onClick: () => void;
  disabled?: boolean;
}

function KeyfileModeOption({
  mode,
  currentMode,
  icon,
  title,
  description,
  onClick,
  disabled,
}: KeyfileModeOptionProps) {
  const isSelected = mode === currentMode;

  return (
    <button
      type="button"
      onClick={onClick}
      disabled={disabled}
      className={cn(
        "flex items-start gap-3 rounded-lg border p-4 text-left transition-colors",
        "hover:bg-muted/50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
        isSelected && "border-primary bg-primary/5 dark:bg-primary/10",
        disabled && "opacity-50 cursor-not-allowed"
      )}
    >
      <div
        className={cn(
          "rounded-full p-2",
          isSelected
            ? "bg-primary/20 text-primary"
            : "bg-muted text-muted-foreground"
        )}
      >
        {icon}
      </div>
      <div className="flex-1">
        <div className={cn("font-medium", isSelected && "text-primary")}>
          {title}
        </div>
        <div className="text-sm text-muted-foreground">{description}</div>
      </div>
      <div
        className={cn(
          "mt-1 size-4 rounded-full border-2",
          isSelected
            ? "border-primary bg-primary"
            : "border-muted-foreground/30"
        )}
      >
        {isSelected && (
          <div className="flex h-full items-center justify-center">
            <div className="size-1.5 rounded-full bg-white" />
          </div>
        )}
      </div>
    </button>
  );
}

export function KeyFileStep({ control, setValue, disabled }: KeyFileStepProps) {
  const keyfileMode = useWatch({ control, name: "keyfileMode" });
  const keyfilePath = useWatch({ control, name: "keyfilePath" });

  async function handleSelectExistingKeyfile() {
    try {
      const file = await open({
        title: "Select Key File",
        filters: [
          { name: "Key Files", extensions: ["key", "keyx"] },
          { name: "All Files", extensions: ["*"] },
        ],
      });
      if (file) {
        setValue("keyfileMode", "select");
        setValue("keyfilePath", file as string);
      }
    } catch {
      // User cancelled or error - ignore
    }
  }

  async function handleSelectNewKeyfileLocation() {
    try {
      const file = await save({
        title: "Save Key File As",
        filters: [{ name: "Key Files", extensions: ["keyx"] }],
        defaultPath: "keyfile.keyx",
      });
      if (file) {
        let path = file as string;
        if (
          !path.toLowerCase().endsWith(".keyx") &&
          !path.toLowerCase().endsWith(".key")
        ) {
          path = `${path}.keyx`;
        }
        setValue("keyfileMode", "generate");
        setValue("keyfilePath", path);
      }
    } catch {
      // User cancelled or error - ignore
    }
  }

  function handleRemoveKeyfile() {
    setValue("keyfileMode", "none");
    setValue("keyfilePath", "");
  }

  function handleModeChange(mode: KeyfileMode) {
    if (mode === "none") {
      setValue("keyfileMode", "none");
      setValue("keyfilePath", "");
    } else if (mode === "select") {
      handleSelectExistingKeyfile();
    } else if (mode === "generate") {
      handleSelectNewKeyfileLocation();
    }
  }

  return (
    <FieldGroup>
      <Field>
        <FieldLabel>Key File (Optional)</FieldLabel>
        <FieldDescription>
          A key file provides additional security. You can use it alone or with
          a password for two-factor authentication.
        </FieldDescription>

        <Controller
          name="keyfileMode"
          control={control}
          render={({ fieldState }) => (
            <>
              <div className="grid gap-3">
                <KeyfileModeOption
                  mode="none"
                  currentMode={keyfileMode}
                  icon={<X className="size-4" />}
                  title="No Key File"
                  description="Use password only for authentication"
                  onClick={() => handleModeChange("none")}
                  disabled={disabled ?? false}
                />

                <KeyfileModeOption
                  mode="select"
                  currentMode={keyfileMode}
                  icon={<FolderOpen className="size-4" />}
                  title="Select Existing Key File"
                  description="Use a key file you already have"
                  onClick={() => handleModeChange("select")}
                  disabled={disabled ?? false}
                />

                <KeyfileModeOption
                  mode="generate"
                  currentMode={keyfileMode}
                  icon={<Plus className="size-4" />}
                  title="Generate New Key File"
                  description="Create a new random key file"
                  onClick={() => handleModeChange("generate")}
                  disabled={disabled ?? false}
                />
              </div>

              {fieldState.error && (
                <FieldError>{fieldState.error.message}</FieldError>
              )}
            </>
          )}
        />
      </Field>

      {keyfileMode !== "none" && keyfilePath && (
        <div className="rounded-lg border bg-muted/30 p-4">
          <div className="flex items-center gap-3">
            <div className="rounded-full bg-primary/20 p-2 text-primary">
              <KeyRound className="size-4" />
            </div>
            <div className="flex-1 min-w-0">
              <div className="font-medium truncate">
                {getFilenameFromPath(keyfilePath)}
              </div>
              <div className="text-xs text-muted-foreground truncate">
                {keyfileMode === "generate"
                  ? "Will be generated when creating database"
                  : "Existing key file"}
              </div>
            </div>
            <Button
              type="button"
              variant="ghost"
              size="icon-xs"
              onClick={handleRemoveKeyfile}
              disabled={disabled}
              aria-label="Remove keyfile"
            >
              <X className="size-4" />
            </Button>
          </div>
        </div>
      )}

      <Controller
        name="keyfilePath"
        control={control}
        render={({ fieldState }) => (
          <>
            {fieldState.error && (
              <FieldError>{fieldState.error.message}</FieldError>
            )}
          </>
        )}
      />

      {keyfileMode !== "none" && (
        <Alert
          variant="default"
          className="border-amber-500/50 bg-amber-50/50 dark:bg-amber-950/20"
        >
          <ShieldAlert className="size-4 text-amber-600" />
          <AlertTitle className="text-amber-800 dark:text-amber-400">
            Key File Security
          </AlertTitle>
          <AlertDescription className="text-amber-700 dark:text-amber-300">
            Store your key file separately from your database. If you lose the
            key file and forget your password, you will not be able to access
            your data.
          </AlertDescription>
        </Alert>
      )}
    </FieldGroup>
  );
}
