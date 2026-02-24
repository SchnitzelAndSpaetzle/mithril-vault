// SPDX-License-Identifier: MIT

import { describe, expect, it } from "vitest";
import type { Group } from "@/lib/types";
import {
  flattenGroups,
  getDescendantIds,
  sumGroupEntryCounts,
} from "./group-utils";

function makeGroup(overrides: Partial<Group> = {}): Group {
  return {
    id: "group-id",
    parentId: null,
    name: "Group",
    icon: null,
    customIconUuid: null,
    children: [],
    ...overrides,
  };
}

describe("group-utils", () => {
  it("flattens groups with depth", () => {
    const groups: Group[] = [
      makeGroup({
        id: "root",
        name: "Root",
        children: [
          makeGroup({
            id: "child-1",
            parentId: "root",
            name: "Child 1",
            children: [
              makeGroup({
                id: "grandchild-1",
                parentId: "child-1",
                name: "Grandchild 1",
              }),
            ],
          }),
          makeGroup({
            id: "child-2",
            parentId: "root",
            name: "Child 2",
          }),
        ],
      }),
    ];

    expect(flattenGroups(groups)).toEqual([
      { id: "root", name: "Root", depth: 0 },
      { id: "child-1", name: "Child 1", depth: 1 },
      { id: "grandchild-1", name: "Grandchild 1", depth: 2 },
      { id: "child-2", name: "Child 2", depth: 1 },
    ]);
  });

  it("sums entry counts recursively", () => {
    const tree = makeGroup({
      id: "root",
      children: [
        makeGroup({
          id: "child-a",
          parentId: "root",
          children: [makeGroup({ id: "grandchild-a", parentId: "child-a" })],
        }),
        makeGroup({ id: "child-b", parentId: "root" }),
      ],
    });

    const entryCounts = {
      root: 1,
      "child-a": 2,
      "grandchild-a": 3,
      "child-b": 4,
    };

    expect(sumGroupEntryCounts(tree, entryCounts)).toBe(10);
  });

  it("collects descendant IDs without including the group itself", () => {
    const tree = makeGroup({
      id: "root",
      children: [
        makeGroup({
          id: "child-a",
          parentId: "root",
          children: [makeGroup({ id: "grandchild-a", parentId: "child-a" })],
        }),
        makeGroup({ id: "child-b", parentId: "root" }),
      ],
    });

    expect(getDescendantIds(tree)).toEqual([
      "child-a",
      "grandchild-a",
      "child-b",
    ]);
  });
});
