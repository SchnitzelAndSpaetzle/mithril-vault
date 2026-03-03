// SPDX-License-Identifier: MIT

import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import { SettingsSection } from "@/components/settings/SettingsSection";

describe("SettingsSection", () => {
  it("renders section title, description, and children", () => {
    render(
      <SettingsSection
        id="security"
        title="Security"
        description="Security preferences"
      >
        <div>Child content</div>
      </SettingsSection>
    );

    expect(screen.getByText("Security")).toBeInTheDocument();
    expect(screen.getByText("Security preferences")).toBeInTheDocument();
    expect(screen.getByText("Child content")).toBeInTheDocument();
  });

  it("renders optional actions", () => {
    render(
      <SettingsSection
        id="general"
        title="General"
        actions={<button>Save</button>}
      >
        <div>Body</div>
      </SettingsSection>
    );

    expect(screen.getByRole("button", { name: "Save" })).toBeInTheDocument();
  });
});
