// SPDX-License-Identifier: MIT

import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";

import { ReusedGroupRow } from "./ReusedGroupRow";
import type { Entry } from "@/lib/types";

// Minimal Entry stub — only the title / id pair drives the row, the
// rest of the Entry shape is irrelevant to the inline-expand UI.
function entry(id: string, title: string): Entry {
  return {
    id,
    groupId: "root",
    title,
    username: "",
    tags: [],
    customFields: {},
    customFieldMeta: [],
    createdAt: "",
    modifiedAt: "",
    accessedAt: "",
    expires: false,
  };
}

const memberIds = ["a", "b", "c"];
const entries = [
  entry("a", "GitHub"),
  entry("b", "GitLab"),
  entry("c", "Bitbucket"),
];

describe("ReusedGroupRow", () => {
  // The collapsed row is the default. It announces the member count
  // (the High-section "Reused (N entries)" copy) so a screen reader
  // user knows what they're expanding before they expand it. The
  // member titles are not yet in the DOM.
  it("renders collapsed by default with the member count", () => {
    render(
      <ReusedGroupRow
        entryIds={memberIds}
        entries={entries}
        onOpenEntry={vi.fn()}
      />
    );
    expect(
      screen.getByText("passwordHealth.findings.password.reused")
    ).toBeInTheDocument();
    expect(
      screen.getByText("passwordHealth.reused.memberCount")
    ).toBeInTheDocument();
    // Members are hidden until the row is expanded.
    expect(screen.queryByText("GitHub")).not.toBeInTheDocument();
    expect(screen.queryByText("GitLab")).not.toBeInTheDocument();
  });

  // Clicking the toggle expands the row inline (no navigation): each
  // member Entry's title appears with an "Open Entry" action. The
  // expand toggle stays in place — collapsing must be possible
  // without leaving the report view.
  it("expands inline to show member titles and Open Entry buttons", () => {
    render(
      <ReusedGroupRow
        entryIds={memberIds}
        entries={entries}
        onOpenEntry={vi.fn()}
      />
    );

    fireEvent.click(
      screen.getByRole("button", { name: "passwordHealth.reused.expand" })
    );

    expect(screen.getByText("GitHub")).toBeInTheDocument();
    expect(screen.getByText("GitLab")).toBeInTheDocument();
    expect(screen.getByText("Bitbucket")).toBeInTheDocument();
    // One Open Entry button per member.
    expect(
      screen.getAllByRole("button", {
        name: "passwordHealth.actions.openEntry",
      })
    ).toHaveLength(memberIds.length);
  });

  // Clicking Open Entry on a member fires `onOpenEntry(entryId)` so
  // the parent can navigate. The row stays expanded — the click is
  // not a side effect of collapse.
  it("calls onOpenEntry with the member id when Open Entry is clicked", () => {
    const onOpenEntry = vi.fn();
    render(
      <ReusedGroupRow
        entryIds={memberIds}
        entries={entries}
        onOpenEntry={onOpenEntry}
      />
    );
    fireEvent.click(
      screen.getByRole("button", { name: "passwordHealth.reused.expand" })
    );
    const openButtons = screen.getAllByRole("button", {
      name: "passwordHealth.actions.openEntry",
    });
    const secondMemberButton = openButtons[1];
    expect(secondMemberButton).toBeDefined();
    fireEvent.click(secondMemberButton as HTMLElement);
    expect(onOpenEntry).toHaveBeenCalledWith("b");
  });

  // Toggling collapses the row again — pinning bi-directional
  // behaviour so a future refactor that only handles the expand
  // half (e.g. by switching to a one-shot disclosure) fails loudly.
  it("collapses back when the toggle is clicked twice", () => {
    render(
      <ReusedGroupRow
        entryIds={memberIds}
        entries={entries}
        onOpenEntry={vi.fn()}
      />
    );
    const toggle = screen.getByRole("button", {
      name: "passwordHealth.reused.expand",
    });
    fireEvent.click(toggle);
    expect(screen.getByText("GitHub")).toBeInTheDocument();
    fireEvent.click(
      screen.getByRole("button", { name: "passwordHealth.reused.collapse" })
    );
    expect(screen.queryByText("GitHub")).not.toBeInTheDocument();
  });

  // A member id with no matching Entry (e.g. the Entry list is stale
  // while a re-fetch is in flight) must still render so the row
  // count stays accurate. The id is the fallback label.
  it("falls back to the entry id when no matching Entry is in the lookup", () => {
    render(
      <ReusedGroupRow
        entryIds={["a", "missing"]}
        entries={[entry("a", "GitHub")]}
        onOpenEntry={vi.fn()}
      />
    );
    fireEvent.click(
      screen.getByRole("button", { name: "passwordHealth.reused.expand" })
    );
    expect(screen.getByText("missing")).toBeInTheDocument();
  });
});
