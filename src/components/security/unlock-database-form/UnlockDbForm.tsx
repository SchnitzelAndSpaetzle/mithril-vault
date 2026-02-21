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
import { Controller, useForm, useWatch } from "react-hook-form";
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
import { toast } from "sonner";
import {
  type DatabaseTabsState,
  useDatabaseTabs,
} from "@/stores/database-tabs";
import { getFilenameFromPath } from "@/lib/utils.ts";

interface UnlockDbFormProps {
  initialPath?: string | undefined;
  initialKeyfile?: string | undefined;
  rememberKeyfile?: boolean | undefined;
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

export function UnlockDbForm({
  initialPath,
  initialKeyfile,
  rememberKeyfile: rememberKeyfileDefault,
}: UnlockDbFormProps) {
  const navigate = useNavigate();
  const addTab = useDatabaseTabs((state: DatabaseTabsState) => state.addTab);
  const updateTabInfo = useDatabaseTabs(
    (state: DatabaseTabsState) => state.updateTabInfo
  );
  const setActiveTab = useDatabaseTabs(
    (state: DatabaseTabsState) => state.setActiveTab
  );
  const activeTabId = useDatabaseTabs(
    (state: DatabaseTabsState) => state.activeTabId
  );
  const [showPassword, setShowPassword] = useState(false);
  const [isUnlocking, setIsUnlocking] = useState(false);
  const [unlockError, setUnlockError] = useState<string | null>(null);
  const [rememberKeyfile, setRememberKeyfile] = useState(
    Boolean(rememberKeyfileDefault)
  );

  const openDbForm = useForm<OpenDatabaseFormValues>({
    resolver: zodResolver(openDatabaseSchema),
    defaultValues: {
      filePath: initialPath ?? "",
      password: "",
      keyfilePath: initialKeyfile ?? "",
    },
  });

  const [filePath, keyfilePath] = useWatch({
    control: openDbForm.control,
    name: ["filePath", "keyfilePath"],
  });

  useEffect(() => {
    if (initialPath) {
      openDbForm.setValue("filePath", initialPath);
    }
  }, [initialPath, openDbForm]);

  useEffect(() => {
    if (initialKeyfile !== undefined) {
      openDbForm.setValue("keyfilePath", initialKeyfile ?? "");
    }
    if (rememberKeyfileDefault !== undefined) {
      setRememberKeyfile(Boolean(rememberKeyfileDefault));
    }
  }, [initialKeyfile, rememberKeyfileDefault, openDbForm]);

  async function handleSelectDatabase() {
    try {
      const file = await open({
        title: "Open Database",
        filters: [{ name: "KeePass Database", extensions: ["kdbx"] }],
      });
      if (file) {
        await navigate({ to: "/unlock", search: { path: file as string } });
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
      const tabId = activeTabId ?? addTab(data.filePath);

      let info: Awaited<ReturnType<typeof database.open>> | null = null;

      // Determine which unlock method to use
      if (data.keyfilePath && data.password) {
        info = await database.openWithKeyfile(
          data.filePath,
          data.password,
          data.keyfilePath
        );
      } else if (data.keyfilePath) {
        info = await database.openWithKeyfileOnly(
          data.filePath,
          data.keyfilePath
        );
      } else if (data.password) {
        info = await database.open(data.filePath, data.password);
      } else {
        setUnlockError("Please enter a password or select a keyfile.");
        setIsUnlocking(false);
        return;
      }

      if (info) {
        updateTabInfo(tabId, info);
        setActiveTab(tabId);
      }

      // Save to recent databases (with keyfile if "remember" is checked)
      try {
        await settings.addRecentDatabase(
          data.filePath,
          rememberKeyfile ? data.keyfilePath : undefined
        );
      } catch (error) {
        console.warn("Failed to update recent database list", error);
        toast.warning("Failed to update recent database list");
      }

      if (info) {
        await navigate({
          to: "/dashboard/index/$dbId",
          params: { dbId: info.path },
        });
      } else {
        await navigate({ to: "/" });
      }
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
                  autoFocus
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
