// SPDX-License-Identifier: MIT

import { Toaster } from "@/components/ui/sonner.tsx";
import { ThemeProvider } from "@/components/theme-provider.tsx";
import React from "react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";

interface AppProps {
  children: React.ReactNode;
}
const queryClient = new QueryClient();

function App({ children }: Readonly<AppProps>) {
  return (
    <QueryClientProvider client={queryClient}>
      <ThemeProvider defaultTheme="dark" storageKey="vite-ui-theme">
        <main>{children}</main>
        <Toaster />
      </ThemeProvider>
    </QueryClientProvider>
  );
}

export default App;
