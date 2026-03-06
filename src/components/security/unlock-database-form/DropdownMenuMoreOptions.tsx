import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Ellipsis } from "lucide-react";
import { Link } from "@tanstack/react-router";

export default function DropdownMenuMoreOptions() {
  const { t } = useTranslation();

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="outline">
          <Ellipsis />
          {t("welcome.more")}
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent className="w-56" align="end">
        <DropdownMenuGroup>
          <DropdownMenuItem>{t("welcome.moreMenu.demo")}</DropdownMenuItem>
          <DropdownMenuItem>
            <Link to="/password-generator" className="flex items-center gap-2">
              {t("welcome.moreMenu.generatePassword")}
            </Link>
          </DropdownMenuItem>
          <DropdownMenuItem>{t("welcome.moreMenu.settings")}</DropdownMenuItem>
        </DropdownMenuGroup>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
