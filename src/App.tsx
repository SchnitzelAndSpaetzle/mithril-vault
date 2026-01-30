// SPDX-License-Identifier: MIT

import { Toaster } from "@/components/ui/sonner.tsx";
import { ThemeProvider } from "@/components/theme-provider.tsx";
import React from "react";

interface AppProps {
  children: React.ReactNode;
}

function App({ children }: Readonly<AppProps>) {
  return (
    <ThemeProvider defaultTheme="dark" storageKey="vite-ui-theme">
      <main>{children}</main>
      <Toaster />
    </ThemeProvider>
  );
}

export default App;
