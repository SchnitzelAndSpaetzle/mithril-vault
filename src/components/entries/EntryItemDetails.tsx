import { useState } from "react";
import { Separator } from "@/components/ui/separator.tsx";
import { Check, Copy, Keyboard } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import {
  Avatar,
  AvatarBadge,
  AvatarFallback,
  AvatarImage,
} from "@/components/ui/avatar.tsx";
import { EntryItemPasswordStatusCard } from "@/components/entries/entryItemDetails/EntryItemPasswordStatusCard.tsx";

export default function EntryItemDetails() {
  return (
    <>
      {/* title section */}
      <div className="flex justify-between items-center pb-2">
        <div className="flex items-center gap-4">
          <Avatar>
            <AvatarImage src="https://github.com/shadcn.png" alt="@shadcn" />
            <AvatarFallback>CN</AvatarFallback>
            <AvatarBadge className="bg-green-600 dark:bg-green-800" />
          </Avatar>
          <h4 className="scroll-m-20 text-xl font-semibold tracking-tight">
            Jetbrains acc
          </h4>
        </div>
      </div>

      {/* list items */}
      <div className="border rounded-md">
        <EntryItem label="User Name" value="blablabla@text.com" />
        <Separator />
        <EntryItem label="Password" value="••••••••" />
        <Separator />
        <EntryItem
          label="Website"
          value="https://news.ycombinator.com/item?id=46712678"
        />
        <Separator />
        {/* tags section*/}
        <div className="flex justify-between items-center px-4 py-2">
          <small className="text-sm font-medium">Tags</small>
          <div>
            {/* TODO: make badge clickable, so user when click will fill global search with tag */}
            <a href="#">
              <Badge variant="outline">Work</Badge>
            </a>
            <Badge variant="outline">cPanel</Badge>
            <Badge variant="outline">git</Badge>
          </div>
        </div>
        <Separator />
        {/* notes section*/}
        <div className="flex justify-between items-center px-4 py-2">
          <p className="text-sm font-medium text-muted-foreground">
            Lorem ipsum dolor sit amet, consectetur adipiscing elit. Maecenas
            ornare lorem ipsum, in pretium justo finibus eget. In leo metus,
            semper sed maximus finibus, facilisis nec ipsum. Curabitur tempor
            ornare sapien in egestas. Nullam viverra augue nec porttitor
            consequat.
          </p>
        </div>
      </div>

      {/* second part of details*/}
      <div className="border rounded-md">
        <EntryItemBasic label="Created" value="Feb 17, 2022, 3:56:34 PM" />
        <Separator />
        <EntryItemBasic label="Updated" value="Feb 17, 2022, 3:58:43 PM" />
        <Separator />
        <EntryItemBasic label="File" value="KeePass-MainDB" />
        <Separator />
        {/* TODO: change group later to be clickable so user can search oaa the groups in main search */}
        <EntryItemBasic label="Group" value="KeePass-MainDB" />
        <Separator />
        {/* TODO: make history clickable and then show user history of passwords for this entry. */}
        <EntryItemBasic label="History" value="2 records" />
      </div>
      {/*  TODO: make password statuses interactive */}
      <EntryItemPasswordStatusCard />
      <EntryItemPasswordStatusCard status="reused" />
      <EntryItemPasswordStatusCard status="breach" />
      <EntryItemPasswordStatusCard status="compromised" />
    </>
  );
}

function EntryItem({ label, value }: { label: string; value: string }) {
  const [isCopied, setIsCopied] = useState(false);

  const handleCopy = async () => {
    await navigator.clipboard.writeText(value);
    setIsCopied(true);
    setTimeout(() => {
      setIsCopied(false);
    }, 2000);
  };

  return (
    <div className="flex justify-between items-center px-4 py-2">
      <small className="text-sm font-medium">{label}</small>
      <div className="flex gap-2 items-center">
        <button
          onClick={handleCopy}
          className="group text-sm font-medium text-muted-foreground hover:bg-accent px-2 py-1 rounded-sm cursor-pointer transition-all duration-200 flex items-center gap-2"
        >
          <span className="transition-all duration-200 truncate max-w-50">
            {isCopied ? "Copied" : value}
          </span>
          {isCopied ? (
            <Check className="h-3 w-3 text-green-500 transition-all duration-200" />
          ) : (
            <Copy className="h-3 w-3 opacity-0 group-hover:opacity-100 transition-all duration-200" />
          )}
        </button>
        <Button variant="outline" size="icon-xs" aria-label="auto-type">
          <Keyboard />
        </Button>
      </div>
    </div>
  );
}

function EntryItemBasic({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex justify-between items-center px-4 py-2">
      <small className="text-sm font-medium">{label}</small>
      <small className="text-sm font-medium text-muted-foreground">
        {value}
      </small>
    </div>
  );
}
