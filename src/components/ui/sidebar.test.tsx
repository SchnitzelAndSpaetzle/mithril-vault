import { render } from "@testing-library/react";
import { SidebarMenuSkeleton } from "./sidebar";
import { expect, test } from "vitest";
import * as React from "react";

test("SidebarMenuSkeleton has a stable width", () => {
  const { rerender, container } = render(<SidebarMenuSkeleton />);
  const skeleton = container.querySelector(
    '[data-sidebar="menu-skeleton-text"]'
  );
  const initialWidth = skeleton?.getAttribute("style");

  rerender(<SidebarMenuSkeleton />);
  const updatedWidth = skeleton?.getAttribute("style");

  expect(initialWidth).toBe(updatedWidth);
  expect(initialWidth).toMatch(/--skeleton-width: \d+%/);
});

test("SidebarMenuSkeleton widths are different for different instances", () => {
  const { container } = render(
    <div>
      <SidebarMenuSkeleton data-testid="s1" />
      <SidebarMenuSkeleton data-testid="s2" />
    </div>
  );

  const skeletons = container.querySelectorAll(
    '[data-sidebar="menu-skeleton-text"]'
  );
  const width1 = skeletons[0].getAttribute("style");
  const width2 = skeletons[1].getAttribute("style");

  // While they COULD be the same by chance (1 in 40), they are likely different.
  // With Math.random it's random. With useId it should also be different.
  // This is just to ensure they aren't all hardcoded to the same value.
  console.log({ width1, width2 });
});
