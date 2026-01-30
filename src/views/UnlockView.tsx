import { cn } from "@/lib/utils.ts";
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
}

function getFilenameFromPath(path: string | undefined): string {
  if (!path) return "";
  const parts = path.split(/[/\\]/);
  return parts[parts.length - 1] || "";
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
  ...props
}: UnlockViewProps) {
  const filename = getFilenameFromPath(initialPath);
  const directory = getDirectoryFromPath(initialPath);

  return (
    <div className={cn("flex flex-col gap-6", className)} {...props}>
      <Card>
        <CardHeader className="text-center">
          <CardTitle className="text-xl">
            {filename ? `Unlock ${filename}` : "Unlock Database"}
          </CardTitle>
          {directory && (
            <CardDescription>
              <code className="bg-muted relative rounded px-[0.3rem] py-[0.2rem] font-mono text-xs font-semibold">
                {directory}
              </code>
            </CardDescription>
          )}
        </CardHeader>
        <CardContent>
          <UnlockDbForm initialPath={initialPath} />
        </CardContent>
      </Card>
    </div>
  );
}
