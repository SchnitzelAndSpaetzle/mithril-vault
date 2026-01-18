import {
  Item,
  ItemActions,
  ItemContent,
  ItemDescription,
  ItemMedia,
  ItemTitle,
} from "@/components/ui/item.tsx";
import { ChevronRightIcon, FolderOpen } from "lucide-react";
import DropBoxIcon from "@/components/ui/customIcons/DropBoxIcon.tsx";
import OneDriveIcon from "@/components/ui/customIcons/OneDriveIcon.tsx";
import GoogleDriveIcon from "@/components/ui/customIcons/GoogleDriveIcon.tsx";

const mockRecentItems = [
  {
    icon: "local",
    title: "MainDatabase-example.kdbx",
    description: "/Documents/Personal/MainDatabase-example.kdbx",
  },
  {
    icon: "dropbox",
    title: "cloud-db-test.kdbx",
    description: "/Dropbox/cloud-db-test.kdbx",
  },
  {
    icon: "googledrive",
    title: "cloud-db-test.kdbx",
    description: "/Google Drive/cloud-db-test.kdbx",
  },
  {
    icon: "onedrive",
    title: "cloud-db-test.kdbx",
    description: "/OneDrive/cloud-db-test.kdbx",
  },
];

export default function RecentOpenedDatabaseItems() {
  return (
    <div className="flex w-full max-w-md flex-col gap-2">
      {mockRecentItems.map((item) => (
        <Item key={item.description} variant="outline" size="sm" asChild>
          <a href="#">
            <ItemMedia>
              <IconSwitch name={item.icon} />
            </ItemMedia>
            <ItemContent>
              <ItemTitle>{item.title}</ItemTitle>
              <ItemDescription className="line-clamp-1">
                {item.description}
              </ItemDescription>
            </ItemContent>
            <ItemActions>
              <ChevronRightIcon className="size-4" />
            </ItemActions>
          </a>
        </Item>
      ))}
    </div>
  );
}

interface IconProps {
  name: string;
}

function IconSwitch({ name }: IconProps) {
  switch (name) {
    case "dropbox":
      return <DropBoxIcon className="size-5" />;
    case "googledrive":
      return <GoogleDriveIcon className="size-5" />;
    case "onedrive":
      return <OneDriveIcon className="size-5" />;
    case "local":
      return <FolderOpen className="size-5" />;
    default:
      return <FolderOpen className="size-5" />;
  }
}
