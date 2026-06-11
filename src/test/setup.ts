// SPDX-License-Identifier: MIT
import "@testing-library/jest-dom";
import { vi } from "vitest";

// jsdom lacks ResizeObserver, which Radix primitives (e.g. Checkbox) rely on.
if (!globalThis.ResizeObserver) {
  globalThis.ResizeObserver = class {
    observe() {
      /* noop for tests */
    }
    unobserve() {
      /* noop for tests */
    }
    disconnect() {
      /* noop for tests */
    }
  };
}

// jsdom lacks matchMedia, which `useIsMobile` and the theme provider rely on.
// Default to the desktop breakpoint (no match); tests that care about the
// mobile layout mock `@/hooks/use-mobile` directly.
if (!window.matchMedia) {
  window.matchMedia = (query: string) =>
    ({
      matches: false,
      media: query,
      onchange: null,
      addEventListener: () => {
        /* noop for tests */
      },
      removeEventListener: () => {
        /* noop for tests */
      },
      addListener: () => {
        /* noop for tests */
      },
      removeListener: () => {
        /* noop for tests */
      },
      dispatchEvent: () => false,
    }) as unknown as MediaQueryList;
}

// The native drag-drop listener (`useAttachmentDrop`) subscribes through the
// webview API, which has no Tauri runtime under jsdom. Default to an inert
// subscription; tests that exercise drops mock this module to capture the
// handler.
vi.mock("@tauri-apps/api/webview", () => ({
  getCurrentWebview: () => ({
    onDragDropEvent: () =>
      Promise.resolve(() => {
        /* noop unlisten for tests */
      }),
  }),
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, opts?: Record<string, unknown>) => {
      if (opts) {
        return Object.entries(opts).reduce(
          (acc, [k, v]) => acc.replace(`{{${k}}}`, String(v)),
          key
        );
      }
      return key;
    },
    i18n: {
      language: "en",
      changeLanguage: vi.fn().mockResolvedValue(undefined),
    },
  }),
  Trans: ({ children }: { children: React.ReactNode }) => children,
  initReactI18next: {
    type: "3rdParty",
    init: () => {
      /* noop for tests */
    },
  },
}));
