import { describe, expect, it } from "vitest";
import { entryFormSchema, type EntryFormValues } from "@/lib/formTypes";

function baseValues(overrides: Partial<EntryFormValues> = {}): EntryFormValues {
  return {
    title: "Example",
    username: "",
    password: "",
    url: "",
    notes: "",
    iconId: 0,
    customIconUuid: null,
    tags: [],
    customFields: [],
    groupId: undefined,
    expires: false,
    expiryTime: null,
    ...overrides,
  };
}

describe("entryFormSchema expiry refinement", () => {
  it("fails when expires is on but no expiry time is chosen", () => {
    const result = entryFormSchema.safeParse(
      baseValues({ expires: true, expiryTime: null })
    );
    expect(result.success).toBe(false);
    if (!result.success) {
      expect(result.error.issues.some((i) => i.path[0] === "expiryTime")).toBe(
        true
      );
    }
  });

  it("passes when expires is on with a future expiry time", () => {
    const result = entryFormSchema.safeParse(
      baseValues({ expires: true, expiryTime: new Date(2099, 0, 1) })
    );
    expect(result.success).toBe(true);
  });

  it("passes when expires is on with a past expiry time", () => {
    const result = entryFormSchema.safeParse(
      baseValues({ expires: true, expiryTime: new Date(2000, 0, 1) })
    );
    expect(result.success).toBe(true);
  });

  it("passes when expires is off regardless of expiry time", () => {
    expect(
      entryFormSchema.safeParse(
        baseValues({ expires: false, expiryTime: null })
      ).success
    ).toBe(true);
    expect(
      entryFormSchema.safeParse(
        baseValues({ expires: false, expiryTime: new Date(2000, 0, 1) })
      ).success
    ).toBe(true);
  });
});
