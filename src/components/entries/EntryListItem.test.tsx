// SPDX-License-Identifier: MIT

import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";

import EntryListItem from "./EntryListItem";
import { TooltipProvider } from "@/components/ui/tooltip";

// Override the global setup mock for this file so opts.count flows
// through into the key — the production translation reads
// `Reused ({{count}} entries)`, and we need a substitute the test
// can introspect. We append `:<count>` so a key like
// `passwordHealth.reused.tooltip` rendered with count 3 turns into
// `passwordHealth.reused.tooltip:3`.
vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, opts?: Record<string, unknown>) => {
      if (opts && typeof opts["count"] === "number") {
        return `${key}:${opts["count"]}`;
      }
      return key;
    },
    i18n: { language: "en", changeLanguage: vi.fn() },
  }),
  Trans: ({ children }: { children: React.ReactNode }) => children,
  initReactI18next: {
    type: "3rdParty",
    init: () => {
      /* noop */
    },
  },
}));

const baseProps = {
  id: "entry-1",
  groupId: "root",
  title: "GitHub",
  username: "octocat",
  url: null,
  notes: null,
  iconId: 0,
  customIconUuid: null,
  tags: [],
  customFields: {},
  customFieldMeta: [],
  createdAt: "",
  modifiedAt: "",
  accessedAt: "",
  expires: false,
  customIcons: {},
};

function renderItem(extra: Record<string, unknown>) {
  return render(
    <TooltipProvider>
      <EntryListItem {...baseProps} {...extra} />
    </TooltipProvider>
  );
}

describe("EntryListItem findings indicator", () => {
  // The indicator's aria-label is what a screen reader announces;
  // when an Entry is reused we want that label to include the
  // member count so the user knows the scale of the remediation
  // ("Reused (3 entries)") rather than just "Reused".
  it("includes Reused (N entries) in the aria-label when reusedGroupSize is set", () => {
    renderItem({
      findings: [{ entryId: "entry-1", kind: "password.reused" }],
      reusedGroupSize: 3,
    });
    const icon = screen.getByLabelText(/passwordHealth\.reused\.tooltip/);
    expect(icon.getAttribute("aria-label")).toContain("3");
  });

  // When an Entry has no reused finding, the prop is ignored — the
  // tooltip is just the per-kind label list. Pinning this guards
  // against a regression that always appends the reused-tooltip
  // string regardless of the actual findings.
  it("does not include the reused tooltip when there is no reused finding", () => {
    renderItem({
      findings: [{ entryId: "entry-1", kind: "password.expired" }],
      reusedGroupSize: 5,
    });
    const icon = screen.getByLabelText(
      "passwordHealth.findings.password.expired"
    );
    expect(icon.getAttribute("aria-label")).not.toContain(
      "passwordHealth.reused.tooltip"
    );
  });
});
