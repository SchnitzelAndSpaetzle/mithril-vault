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
