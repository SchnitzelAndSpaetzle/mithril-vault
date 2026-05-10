import { useTranslation } from "react-i18next";
import { cn, getFilenameFromPath } from "@/lib/utils.ts";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card.tsx";
import React from "react";
import { UnlockDbForm } from "@/components/security/unlock-database-form/UnlockDbForm.tsx";

interface UnlockViewProps extends React.ComponentProps<"div"> {
  initialPath?: string | undefined;
  initialKeyfile?: string | undefined;
  rememberKeyfile?: boolean | undefined;
  isLocked?: boolean | undefined;
}

function getDirectoryFromPath(path: string | undefined): string {
  if (!path) return "";
  const parts = path.split(/[/\\]/);
  parts.pop();
  return parts.join("/") || "/";
}

export function UnlockView({
  className,
  initialPath,
  initialKeyfile,
  rememberKeyfile,
  isLocked,
  ...props
}: UnlockViewProps) {
  const { t } = useTranslation();
  const filename = getFilenameFromPath(initialPath);
  const directory = getDirectoryFromPath(initialPath);

  return (
    <div className={cn("flex flex-col gap-6", className)} {...props}>
      <Card>
        <CardHeader className="text-center">
          <CardTitle className="text-xl">
            {isLocked
              ? t("unlock.lockedTitle", { filename: filename ?? "" })
              : filename
                ? t("unlock.titleWithName", { filename })
                : t("unlock.title")}
          </CardTitle>
          {isLocked && (
            <CardDescription>{t("unlock.lockedDescription")}</CardDescription>
          )}
          {!isLocked && directory && (
            <CardDescription>
              <code className="bg-muted relative rounded px-[0.3rem] py-[0.2rem] font-mono text-xs font-semibold">
                {directory}
              </code>
            </CardDescription>
          )}
        </CardHeader>
        <CardContent>
          <UnlockDbForm
            initialPath={initialPath}
            initialKeyfile={initialKeyfile}
            rememberKeyfile={rememberKeyfile}
            isLocked={isLocked}
          />
        </CardContent>
      </Card>
    </div>
  );
}
