import { useTranslation } from "react-i18next";
import {
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from "@/components/ui/empty";
import { KeyIcon } from "lucide-react";

export function EntryItemDetailsEmpty() {
  const { t } = useTranslation();

  return (
    <Empty>
      <EmptyHeader>
        <EmptyMedia variant="icon">
          <KeyIcon />
        </EmptyMedia>
        <EmptyTitle>{t("entries.noEntrySelected")}</EmptyTitle>
        <EmptyDescription>
          {t("entries.noEntrySelectedDescription")}
        </EmptyDescription>
      </EmptyHeader>
    </Empty>
  );
}
