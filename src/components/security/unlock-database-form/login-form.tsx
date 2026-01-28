import { cn } from "@/lib/utils.ts";
import { Button } from "@/components/ui/button.tsx";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card.tsx";
import {
  Field,
  FieldDescription,
  FieldGroup,
  FieldSeparator,
} from "@/components/ui/field.tsx";
import React from "react";
import { Plus } from "lucide-react";
import { InputGroupPassword } from "@/components/security/unlock-database-form/InputGroupPassword.tsx";
import RecentOpenedDatabaseItems from "@/components/security/unlock-database-form/RecentOpenedDatabaseItems.tsx";
import DropdownMenuOpenDatabase from "@/components/security/unlock-database-form/DropdownMenuOpenDatabase.tsx";
import DropdownMenuMoreOptions from "@/components/security/unlock-database-form/DropdownMenuMoreOptions.tsx";

export function LoginForm({
  className,
  ...props
}: React.ComponentProps<"div">) {
  return (
    <div className={cn("flex flex-col gap-6", className)} {...props}>
      <Card>
        <CardHeader className="text-center">
          <CardTitle className="text-xl">Welcome friend</CardTitle>
          <CardDescription>
            Create or open a database to continue.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <form>
            <FieldGroup>
              <Field className="grid gap-2 sm:grid-cols-3">
                <DropdownMenuOpenDatabase />
                {/*<Button variant="outline" type="button">*/}
                {/*  <FolderOpen />*/}
                {/*  Open*/}
                {/*</Button>*/}
                <Button variant="outline" type="button">
                  <Plus />
                  New
                </Button>
                <DropdownMenuMoreOptions />
                {/*<Button variant="outline" type="button">*/}
                {/*  <Ellipsis />*/}
                {/*  More*/}
                {/*</Button>*/}
              </Field>
              <FieldSeparator className="*:data-[slot=field-separator-content]:bg-card">
                Or continue with
              </FieldSeparator>
              <InputGroupPassword />
              <FieldSeparator className="*:data-[slot=field-separator-content]:bg-card">
                Recent databases
              </FieldSeparator>
              <RecentOpenedDatabaseItems />
            </FieldGroup>
          </form>
        </CardContent>
      </Card>
      <FieldDescription className="px-6 text-center">
        By clicking continue, you agree to our <a href="#">Terms of Service</a>{" "}
        and <a href="#">Privacy Policy</a>.
      </FieldDescription>
    </div>
  );
}
