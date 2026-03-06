// SPDX-License-Identifier: MIT

import type { ReactNode } from "react";
import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { type Control, useForm } from "react-hook-form";
import {
  EntryFormActions,
  EntryNotesField,
  EntryPasswordField,
  EntryTagsField,
  EntryTitleField,
  EntryUrlField,
  EntryUsernameField,
} from "@/components/entries/entry-edit-form";
import type { EntryFormValues } from "@/lib/formTypes";

vi.mock(
  "@/components/database/create-wizard/PasswordStrengthIndicator",
  () => ({
    PasswordStrengthIndicator: () => null,
  })
);

vi.mock("@/components/entries/PasswordGeneratorPopover", () => ({
  PasswordGeneratorPopover: ({
    children,
    onUsePassword,
  }: {
    children: ReactNode;
    onUsePassword: (password: string) => void;
  }) => (
    <div>
      <button type="button" onClick={() => onUsePassword("generated-password")}>
        Use generated password
      </button>
      {children}
    </div>
  ),
}));

const defaultValues: EntryFormValues = {
  title: "",
  username: "",
  password: "",
  url: "",
  notes: "",
  iconId: 0,
  tags: [],
  customFields: [],
};

function renderWithForm(
  renderField: (control: Control<EntryFormValues>) => ReactNode
) {
  function TestForm() {
    const { control } = useForm<EntryFormValues>({ defaultValues });
    return <form>{renderField(control)}</form>;
  }

  return render(<TestForm />);
}

describe("EntryEditForm field components", () => {
  it("renders title input and icon button", () => {
    renderWithForm((control) => (
      <EntryTitleField control={control} isPending={false} />
    ));

    expect(
      screen.getByPlaceholderText("entries.form.titlePlaceholder")
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "entries.form.chooseIcon" })
    ).toBeInTheDocument();
  });

  it("renders username suggestions and delegates selection callback", () => {
    const onFocus = vi.fn();
    const onBlur = vi.fn((_, onBlurField: () => void) => onBlurField());
    const onKeyDown = vi.fn();
    const onSelectSuggestion = vi.fn();

    renderWithForm((control) => (
      <EntryUsernameField
        control={control}
        isPending={false}
        usernameSuggestions={["alice@example.com"]}
        activeUsernameSuggestionIndex={0}
        showUsernameSuggestions
        onFocus={onFocus}
        onBlur={onBlur}
        onKeyDown={onKeyDown}
        onSelectSuggestion={onSelectSuggestion}
      />
    ));

    fireEvent.focus(
      screen.getByPlaceholderText("entries.form.usernamePlaceholder")
    );
    fireEvent.click(screen.getByRole("option", { name: "alice@example.com" }));

    expect(onFocus).toHaveBeenCalled();
    expect(onSelectSuggestion).toHaveBeenCalledWith(
      "alice@example.com",
      expect.any(Function)
    );
  });

  it("toggles password visibility and forwards generator callback", () => {
    const onUseGeneratedPassword = vi.fn();

    renderWithForm((control) => (
      <EntryPasswordField
        control={control}
        isPending={false}
        watchedPassword=""
        onUseGeneratedPassword={onUseGeneratedPassword}
      />
    ));

    const passwordInput = screen.getByPlaceholderText(
      "entries.form.passwordPlaceholder"
    );
    expect(passwordInput).toHaveAttribute("type", "password");

    fireEvent.click(
      screen.getByRole("button", { name: "entries.form.showPassword" })
    );
    expect(passwordInput).toHaveAttribute("type", "text");

    fireEvent.click(
      screen.getByRole("button", { name: "Use generated password" })
    );
    expect(onUseGeneratedPassword).toHaveBeenCalledWith("generated-password");
  });

  it("renders url and notes fields", () => {
    renderWithForm((control) => (
      <>
        <EntryUrlField control={control} isPending={false} />
        <EntryNotesField control={control} isPending={false} />
      </>
    ));

    expect(
      screen.getByPlaceholderText("entries.form.urlPlaceholder")
    ).toBeInTheDocument();
    expect(
      screen.getByPlaceholderText("entries.form.notesPlaceholder")
    ).toBeInTheDocument();
  });

  it("renders tags field through TagInput", () => {
    renderWithForm((control) => (
      <EntryTagsField
        control={control}
        isPending={false}
        availableTags={["work", "personal"]}
      />
    ));

    expect(screen.getByPlaceholderText("Add tags...")).toBeInTheDocument();
  });

  it("renders action buttons and retry state", () => {
    const onCancel = vi.fn();
    const onRetrySecretLoad = vi.fn();

    render(
      <EntryFormActions
        isPending={false}
        isSubmitDisabled={false}
        isEditMode
        secretLoadError="failed"
        onCancel={onCancel}
        onRetrySecretLoad={onRetrySecretLoad}
      />
    );

    fireEvent.click(screen.getByRole("button", { name: "common.cancel" }));
    fireEvent.click(screen.getByRole("button", { name: "common.retry" }));

    expect(
      screen.getByRole("button", { name: "entries.form.saveChanges" })
    ).toBeInTheDocument();
    expect(onCancel).toHaveBeenCalled();
    expect(onRetrySecretLoad).toHaveBeenCalled();
  });
});
