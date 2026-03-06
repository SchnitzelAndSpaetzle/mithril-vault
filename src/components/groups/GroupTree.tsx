// SPDX-License-Identifier: MIT

import { useTranslation } from "react-i18next";
import { SidebarMenu } from "@/components/ui/sidebar";
import { useCustomIcons } from "@/hooks/use-custom-icons";
import { useGroups } from "@/hooks/use-groups";
import { GroupTreeItem } from "./GroupTreeItem";
import { GroupTreeSkeleton } from "./GroupTreeSkeleton";

interface GroupTreeProps {
  dbId: string | null;
}

export function GroupTree({ dbId }: GroupTreeProps) {
  const { t } = useTranslation();
  const { data: groups, isLoading: groupsLoading } = useGroups(dbId);
  const { data: customIcons, isLoading: iconsLoading } = useCustomIcons(dbId);

  if (!dbId) {
    return null;
  }

  if (groupsLoading || iconsLoading) {
    return <GroupTreeSkeleton />;
  }

  if (!groups || groups.length === 0) {
    return (
      <div className="px-2 py-1 text-sm text-muted-foreground">
        {t("groups.noGroups")}
      </div>
    );
  }

  return (
    <SidebarMenu>
      {groups.map((group) => (
        <GroupTreeItem
          key={group.id}
          group={group}
          dbId={dbId}
          customIcons={customIcons ?? {}}
        />
      ))}
    </SidebarMenu>
  );
}
