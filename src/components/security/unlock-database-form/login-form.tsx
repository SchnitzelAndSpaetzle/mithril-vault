import { cn } from "@/lib/utils.ts";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card.tsx";
import React from "react";
import { InputGroupPassword } from "@/components/security/unlock-database-form/InputGroupPassword.tsx";

export function LoginForm({
  className,
  ...props
}: React.ComponentProps<"div">) {
  return (
    <div className={cn("flex flex-col gap-6", className)} {...props}>
      <Card>
        <CardHeader className="text-center">
          <CardTitle className="text-xl">Unlock DB</CardTitle>
          <CardDescription>
            <code className="bg-muted relative rounded px-[0.3rem] py-[0.2rem] font-mono text-xs font-semibold">
              /Documents/Personal/dbs/
            </code>
          </CardDescription>
        </CardHeader>
        <CardContent>
          <InputGroupPassword />
        </CardContent>
      </Card>
    </div>
  );
}
