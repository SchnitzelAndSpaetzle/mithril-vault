import {
  createFileRoute,
  useCanGoBack,
  useRouter,
} from "@tanstack/react-router";
import NavEntries from "@/components/entries/nav-entries.tsx";
import EntryItemDetails from "@/components/entries/EntryItemDetails.tsx";
import { Button } from "@/components/ui/button.tsx";
import { ArrowBigLeft } from "lucide-react";

export const Route = createFileRoute("/dashboard/entry/$id")({
  component: EntryMobileComponent,
});

function EntryMobileComponent() {
  const router = useRouter();
  const canGoBack = useCanGoBack();

  return (
    <div className="overflow-auto">
      <NavEntries>
        {canGoBack && (
          <Button
            variant="outline"
            size="sm"
            onClick={() => router.history.back()}
          >
            <ArrowBigLeft />
            Back
          </Button>
        )}
      </NavEntries>
      <div className="flex flex-col gap-4 p-4">
        <EntryItemDetails />
      </div>
      {/*<ActionsMobileEntries>*/}
      {/*  <Button variant="outline" size="icon-sm" className="">*/}
      {/*    <Plus />*/}
      {/*  </Button>*/}
      {/*  <SearchForm className="w-full" />*/}
      {/*  <Button variant="outline" size="icon-sm" className="">*/}
      {/*    <ArrowDownAZ />*/}
      {/*  </Button>*/}
      {/*</ActionsMobileEntries>*/}
    </div>
  );
}
