import {
  CornerDownLeft,
  Eye,
  EyeClosed,
  FolderOpen,
  KeyRound,
  Loader2,
  ShieldAlert,
  X,
} from "lucide-react";
import {
  InputGroup,
  InputGroupAddon,
  InputGroupButton,
  InputGroupInput,
  InputGroupText,
} from "@/components/ui/input-group.tsx";
import { useEffect, useState } from "react";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert.tsx";
import { Controller, useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import {
  type OpenDatabaseFormValues,
  openDatabaseSchema,
} from "@/lib/formTypes.ts";
import { useNavigate } from "@tanstack/react-router";
import { open } from "@tauri-apps/plugin-dialog";
import { database, settings } from "@/lib/tauri.ts";
import { Checkbox } from "@/components/ui/checkbox.tsx";
import { Label } from "@/components/ui/label.tsx";

interface UnlockDbFormProps {
  initialPath?: string | undefined;
}

function getFilenameFromPath(path: string | undefined): string {
  if (!path) return "";
  const parts = path.split(/[/\\]/);
  return parts[parts.length - 1] || "";
}

function mapErrorToMessage(error: unknown): string {
  const errorStr = String(error);

  if (errorStr.includes("Invalid password")) {
    return "The password you entered is incorrect. Please try again.";
  }
  if (errorStr.includes("Keyfile not found")) {
    return "The keyfile could not be found at the specified location.";
  }
  if (errorStr.includes("Invalid keyfile format")) {
    return "The selected keyfile has an invalid format.";
  }
  if (errorStr.includes("No credentials provided")) {
    return "Please enter a password or select a keyfile.";
  }
  if (errorStr.includes("Database is locked")) {
    return "This database is currently open in another application. Close it first or force unlock.";
  }
  if (errorStr.includes("Not a valid KDBX file")) {
    return "The selected file is not a valid KeePass database.";
  }
  if (errorStr.includes("Unsupported KDBX version")) {
    return "This database uses an unsupported KeePass format version.";
  }
  if (
    errorStr.includes("IO error") ||
    errorStr.includes("No such file or directory")
  ) {
    return "The database file could not be found or read.";
  }

  return "Failed to unlock database. Please check your credentials and try again.";
}

export function UnlockDbForm({ initialPath }: UnlockDbFormProps) {
  const navigate = useNavigate({ from: "/unlock" });
  const [showPassword, setShowPassword] = useState(false);
  const [isUnlocking, setIsUnlocking] = useState(false);
  const [unlockError, setUnlockError] = useState<string | null>(null);
  const [rememberKeyfile, setRememberKeyfile] = useState(false);

  const openDbForm = useForm<OpenDatabaseFormValues>({
    resolver: zodResolver(openDatabaseSchema),
    defaultValues: {
      filePath: initialPath ?? "",
      password: "",
      keyfilePath: "",
    },
  });

  const filePath = openDbForm.watch("filePath");
  const keyfilePath = openDbForm.watch("keyfilePath");

  // Load saved keyfile when a path changes
  useEffect(() => {
    async function loadSavedKeyfile() {
      if (initialPath) {
        try {
          const savedKeyfile =
            await settings.getKeyfileForDatabase(initialPath);
          if (savedKeyfile) {
            openDbForm.setValue("keyfilePath", savedKeyfile);
            setRememberKeyfile(true);
          }
        } catch {
          // Ignore errors - just don't pre-populate
        }
      }
    }
    loadSavedKeyfile();
  }, [initialPath, openDbForm]);

  async function handleSelectDatabase() {
    try {
      const file = await open({
        title: "Open Database",
        filters: [{ name: "KeePass Database", extensions: ["kdbx"] }],
      });
      if (file) {
        openDbForm.setValue("filePath", file as string);
        setUnlockError(null);

        // Check for saved keyfile for this database
        try {
          const savedKeyfile = await settings.getKeyfileForDatabase(
            file as string
          );
          if (savedKeyfile) {
            openDbForm.setValue("keyfilePath", savedKeyfile);
            setRememberKeyfile(true);
          }
        } catch {
          // Ignore errors
        }
      }
    } catch {
      // User cancelled or error - ignore
    }
  }

  async function handleSelectKeyfile() {
    try {
      const file = await open({
        title: "Select Key File",
        filters: [
          { name: "Key Files", extensions: ["key", "keyx"] },
          { name: "All Files", extensions: ["*"] },
        ],
      });
      if (file) {
        openDbForm.setValue("keyfilePath", file as string);
      }
    } catch {
      // User cancelled or error - ignore
    }
  }

  function handleRemoveKeyfile() {
    openDbForm.setValue("keyfilePath", "");
    setRememberKeyfile(false);
  }

  async function onSubmit(data: OpenDatabaseFormValues) {
    setIsUnlocking(true);
    setUnlockError(null);

    try {
      // Determine which unlock method to use
      if (data.keyfilePath && data.password) {
        await database.openWithKeyfile(
          data.filePath,
          data.password,
          data.keyfilePath
        );
      } else if (data.keyfilePath) {
        await database.openWithKeyfileOnly(data.filePath, data.keyfilePath);
      } else if (data.password) {
        await database.open(data.filePath, data.password);
      } else {
        setUnlockError("Please enter a password or select a keyfile.");
        setIsUnlocking(false);
        return;
      }

      // Save to recent databases (with keyfile if "remember" is checked)
      await settings.addRecentDatabase(
        data.filePath,
        rememberKeyfile ? data.keyfilePath : undefined
      );

      await navigate({ to: "/dashboard" });
    } catch (error) {
      setUnlockError(mapErrorToMessage(error));
    } finally {
      setIsUnlocking(false);
    }
  }

  const filename = getFilenameFromPath(filePath);
  const keyfilename = getFilenameFromPath(keyfilePath);

  return (
    <form id="open-db-form" onSubmit={openDbForm.handleSubmit(onSubmit)}>
      <div className="grid w-full max-w-md gap-6">
        <InputGroup>
          <div className="flex w-full py-4">
            <Controller
              name="password"
              control={openDbForm.control}
              render={({ field, fieldState }) => (
                <InputGroupInput
                  {...field}
                  id={field.name}
                  aria-invalid={fieldState.invalid}
                  type={showPassword ? "text" : "password"}
                  placeholder="enter password here..."
                  autoComplete="off"
                  className="relative flex"
                  disabled={isUnlocking}
                />
              )}
            />

            <InputGroupAddon align="inline-end" className="ml-auto inline-flex">
              <InputGroupButton
                className="ml-auto"
                variant="ghost"
                aria-label={showPassword ? "Hide password" : "Show password"}
                size="icon-xs"
                type="button"
                onClick={() => setShowPassword((prev) => !prev)}
                disabled={isUnlocking}
              >
                {showPassword ? <Eye /> : <EyeClosed />}
              </InputGroupButton>
            </InputGroupAddon>
          </div>

          <InputGroupAddon align="block-end" className="border-t">
            <InputGroupButton
              size="sm"
              variant="ghost"
              type="button"
              onClick={handleSelectKeyfile}
              disabled={isUnlocking}
            >
              add key file <KeyRound />
            </InputGroupButton>
            <InputGroupButton
              type="submit"
              form="open-db-form"
              size="sm"
              className="ml-auto"
              variant="default"
              disabled={isUnlocking || !filePath}
            >
              {isUnlocking ? (
                <>
                  Unlocking <Loader2 className="animate-spin" />
                </>
              ) : (
                <>
                  Unlock <CornerDownLeft />
                </>
              )}
            </InputGroupButton>
          </InputGroupAddon>

          <InputGroupAddon
            align="block-start"
            className="border-b cursor-pointer hover:bg-muted/50 transition-colors"
            onClick={handleSelectDatabase}
          >
            <InputGroupText className="font-mono font-medium">
              <FolderOpen />
              {filename || "Select a database file..."}
            </InputGroupText>
          </InputGroupAddon>

          {keyfilePath && (
            <InputGroupAddon align="block-start" className="border-b">
              <InputGroupText className="font-mono font-medium flex-1">
                <KeyRound />
                {keyfilename}
              </InputGroupText>
              <InputGroupButton
                size="icon-xs"
                variant="ghost"
                type="button"
                onClick={handleRemoveKeyfile}
                disabled={isUnlocking}
                aria-label="Remove keyfile"
              >
                <X className="size-4" />
              </InputGroupButton>
            </InputGroupAddon>
          )}
        </InputGroup>

        {keyfilePath && (
          <div className="flex items-center space-x-2">
            <Checkbox
              id="remember-keyfile"
              checked={rememberKeyfile}
              onCheckedChange={(checked) =>
                setRememberKeyfile(checked === true)
              }
              disabled={isUnlocking}
            />
            <Label
              htmlFor="remember-keyfile"
              className="text-sm text-muted-foreground"
            >
              Remember keyfile for this database
            </Label>
          </div>
        )}

        {unlockError && (
          <Alert variant="destructive">
            <ShieldAlert />
            <AlertTitle>Error unlocking database</AlertTitle>
            <AlertDescription>{unlockError}</AlertDescription>
          </Alert>
        )}
      </div>
    </form>
  );
}
