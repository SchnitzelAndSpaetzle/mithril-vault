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
import { useTranslation } from "react-i18next";
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
import type { TFunction } from "i18next";

interface UnlockDbFormProps {
  initialPath?: string | undefined;
  initialKeyfile?: string | undefined;
  rememberKeyfile?: boolean | undefined;
  isLocked?: boolean | undefined;
}

function mapErrorToMessage(error: unknown, t: TFunction): string {
  const errorStr = String(error);

  if (errorStr.includes("Invalid password")) {
    return t("unlock.errors.invalidPassword");
  }
  if (errorStr.includes("Keyfile not found")) {
    return t("unlock.errors.keyfileNotFound");
  }
  if (errorStr.includes("Invalid keyfile format")) {
    return t("unlock.errors.invalidKeyfileFormat");
  }
  if (errorStr.includes("No credentials provided")) {
    return t("unlock.errors.noCredentials");
  }
  if (errorStr.includes("Not a valid KDBX file")) {
    return t("unlock.errors.notValidKdbx");
  }
  if (errorStr.includes("Unsupported KDBX version")) {
    return t("unlock.errors.unsupportedVersion");
  }
  if (
    errorStr.includes("IO error") ||
    errorStr.includes("No such file or directory")
  ) {
    return t("unlock.errors.ioError");
  }

  return t("unlock.errors.generic");
}

export function UnlockDbForm({
  initialPath,
  initialKeyfile,
  rememberKeyfile: rememberKeyfileDefault,
  isLocked,
}: UnlockDbFormProps) {
  const { t } = useTranslation();
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

  // Sync the rememberKeyfile checkbox to the prop during render so we don't
  // trigger an extra render via useEffect setState.
  const [prevRememberKeyfileDefault, setPrevRememberKeyfileDefault] = useState(
    rememberKeyfileDefault
  );
  if (prevRememberKeyfileDefault !== rememberKeyfileDefault) {
    setPrevRememberKeyfileDefault(rememberKeyfileDefault);
    if (rememberKeyfileDefault !== undefined) {
      setRememberKeyfile(Boolean(rememberKeyfileDefault));
    }
  }

  useEffect(() => {
    if (initialKeyfile !== undefined) {
      openDbForm.setValue("keyfilePath", initialKeyfile ?? "");
    }
  }, [initialKeyfile, openDbForm]);

  async function handleSelectDatabase() {
    try {
      const file = await open({
        title: t("databaseSwitcher.openDatabase"),
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

      if (isLocked) {
        // Re-unlock a locked database (backend has keyfile path)
        info = await database.unlock(data.filePath, data.password || undefined);
      } else if (data.keyfilePath && data.password) {
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
        setUnlockError(t("unlock.errors.noCredentials"));
        setIsUnlocking(false);
        return;
      }

      if (info) {
        updateTabInfo(tabId, info);
        setActiveTab(tabId);
      }

      // Save to recent databases (skip for locked databases - already in list)
      if (!isLocked) {
        try {
          await settings.addRecentDatabase(
            data.filePath,
            rememberKeyfile ? data.keyfilePath : undefined
          );
        } catch (error) {
          console.warn("Failed to update recent database list", error);
          toast.warning(t("unlock.recentListWarning"));
        }
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
      setUnlockError(mapErrorToMessage(error, t));
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
                  placeholder={t("unlock.passwordPlaceholder")}
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
                aria-label={
                  showPassword
                    ? t("unlock.hidePassword")
                    : t("unlock.showPassword")
                }
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
            {!isLocked && (
              <InputGroupButton
                size="sm"
                variant="ghost"
                type="button"
                onClick={handleSelectKeyfile}
                disabled={isUnlocking}
              >
                {t("unlock.addKeyFile")} <KeyRound />
              </InputGroupButton>
            )}
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
                  {t("unlock.unlocking")} <Loader2 className="animate-spin" />
                </>
              ) : (
                <>
                  {t("unlock.unlock")} <CornerDownLeft />
                </>
              )}
            </InputGroupButton>
          </InputGroupAddon>

          {!isLocked && (
            <InputGroupAddon
              align="block-start"
              className="border-b cursor-pointer hover:bg-muted/50 transition-colors"
              onClick={handleSelectDatabase}
            >
              <InputGroupText className="font-mono font-medium">
                <FolderOpen />
                {filename || t("unlock.selectDatabase")}
              </InputGroupText>
            </InputGroupAddon>
          )}

          {!isLocked && keyfilePath && (
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
                aria-label={t("unlock.removeKeyfile")}
              >
                <X className="size-4" />
              </InputGroupButton>
            </InputGroupAddon>
          )}
        </InputGroup>

        {!isLocked && keyfilePath && (
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
              {t("unlock.rememberKeyfile")}
            </Label>
          </div>
        )}

        {unlockError && (
          <Alert variant="destructive">
            <ShieldAlert />
            <AlertTitle>{t("unlock.errorTitle")}</AlertTitle>
            <AlertDescription>{unlockError}</AlertDescription>
          </Alert>
        )}
      </div>
    </form>
  );
}
