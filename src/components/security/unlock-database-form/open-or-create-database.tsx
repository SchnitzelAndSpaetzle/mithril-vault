import { useTranslation } from "react-i18next";
import { cn } from "@/lib";
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
import DropdownMenuOpenDatabase from "@/components/security/unlock-database-form/DropdownMenuOpenDatabase.tsx";
import { Button } from "@/components/ui/button.tsx";
import { Plus } from "lucide-react";
import DropdownMenuMoreOptions from "@/components/security/unlock-database-form/DropdownMenuMoreOptions.tsx";
import RecentOpenedDatabaseItems from "@/components/security/unlock-database-form/RecentOpenedDatabaseItems.tsx";
import { type ComponentProps } from "react";
import type { RecentDatabase } from "@/lib/types";
import { ScrollArea } from "@/components/ui/scroll-area.tsx";
import { Link } from "@tanstack/react-router";

interface OpenOrCreateDatabaseProps extends ComponentProps<"div"> {
  recentDatabases: RecentDatabase[];
}

export default function OpenOrCreateDatabase({
  className,
  recentDatabases,
  ...props
}: OpenOrCreateDatabaseProps) {
  const { t } = useTranslation();

  return (
    <div className={cn("flex flex-col gap-6", className)} {...props}>
      <Card>
        <CardHeader className="text-center">
          <CardTitle className="text-xl">{t("welcome.title")}</CardTitle>
          <CardDescription>{t("welcome.description")}</CardDescription>
        </CardHeader>
        <CardContent>
          <FieldGroup>
            <Field className="grid gap-2 sm:grid-cols-3">
              <DropdownMenuOpenDatabase />
              <Button variant="outline" asChild>
                <Link to="/new">
                  <Plus />
                  {t("welcome.new")}
                </Link>
              </Button>
              <DropdownMenuMoreOptions />
            </Field>

            <FieldSeparator className="*:data-[slot=field-separator-content]:bg-card">
              {t("welcome.recentDatabases")}
            </FieldSeparator>
            <ScrollArea className="h-44">
              <RecentOpenedDatabaseItems recentDatabases={recentDatabases} />
            </ScrollArea>
          </FieldGroup>
        </CardContent>
      </Card>
      <FieldDescription className="px-6 text-center">
        {t("welcome.termsPrefix")} <a href="#">{t("welcome.termsOfService")}</a>{" "}
        {t("welcome.and")} <a href="#">{t("welcome.privacyPolicy")}</a>.
      </FieldDescription>
    </div>
  );
}
