// SPDX-License-Identifier: MIT

import { Toaster } from "@/components/ui/sonner.tsx";
import { ThemeProvider } from "@/components/theme-provider.tsx";
import { useLanguageSync } from "@/hooks/use-language-sync";
import React from "react";

interface AppProps {
  children: React.ReactNode;
}
function App({ children }: Readonly<AppProps>) {
  useLanguageSync();

  return (
    <ThemeProvider defaultTheme="system" storageKey="mithril-vault-theme">
      <main>{children}</main>
      <Toaster />
    </ThemeProvider>
  );
}

export default App;
