import { createFileRoute } from "@tanstack/react-router";
import {
  SidebarInset,
  SidebarProvider,
  SidebarTrigger,
} from "@/components/ui/sidebar.tsx";
import { AppSidebar } from "@/components/app-sidebar.tsx";
import { NavActions } from "@/components/nav-actions.tsx";
import { SearchForm } from "@/components/search-form.tsx";
import { DataTable } from "@/components/data-table.tsx";

import mockData from "@/mock/tabe-mock-data.json";

export const Route = createFileRoute("/")({
  component: Index,
});

function Index() {
  return (
    <SidebarProvider>
      <AppSidebar />
      <SidebarInset>
        <header className="flex h-14 shrink-0 items-center gap-2">
          <div className="flex flex-1 items-center gap-2 px-3">
            <SidebarTrigger />
            {/*<Breadcrumb>*/}
            {/*  <BreadcrumbList>*/}
            {/*    <BreadcrumbItem>*/}
            {/*      <BreadcrumbPage className="line-clamp-1">*/}
            {/*        Project Management & Task Tracking*/}
            {/*      </BreadcrumbPage>*/}
            {/*    </BreadcrumbItem>*/}
            {/*  </BreadcrumbList>*/}
            {/*</Breadcrumb>*/}
          </div>
          <SearchForm className="w-full px-3" />
          <div className="ml-auto px-3">
            <NavActions />
          </div>
        </header>
        <div className="flex flex-1 flex-col gap-4 px-4 py-10">
          <DataTable data={mockData} />
          {/*<div className="bg-muted/50 mx-auto h-24 w-full max-w-3xl rounded-xl" />*/}
          {/*<div className="bg-muted/50 mx-auto h-full w-full max-w-3xl rounded-xl" />*/}
        </div>
      </SidebarInset>
    </SidebarProvider>
  );
}
