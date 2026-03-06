// SPDX-License-Identifier: MIT

import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { SearchForm } from "./search-form";

describe("SearchForm", () => {
  it("renders with placeholder", () => {
    render(<SearchForm query="" onQueryChange={vi.fn()} onClear={vi.fn()} />);
    expect(
      screen.getByPlaceholderText("entries.search.placeholder")
    ).toBeInTheDocument();
  });

  it("calls onQueryChange when typing", () => {
    const onQueryChange = vi.fn();
    render(
      <SearchForm query="" onQueryChange={onQueryChange} onClear={vi.fn()} />
    );

    const input = screen.getByPlaceholderText("entries.search.placeholder");
    fireEvent.change(input, { target: { value: "test" } });
    expect(onQueryChange).toHaveBeenCalledWith("test");
  });

  it("shows clear button when query is non-empty", () => {
    render(
      <SearchForm query="something" onQueryChange={vi.fn()} onClear={vi.fn()} />
    );
    expect(
      screen.getByLabelText("entries.search.clearSearch")
    ).toBeInTheDocument();
  });

  it("does not show clear button when query is empty", () => {
    render(<SearchForm query="" onQueryChange={vi.fn()} onClear={vi.fn()} />);
    expect(
      screen.queryByLabelText("entries.search.clearSearch")
    ).not.toBeInTheDocument();
  });

  it("calls onClear when clear button is clicked", () => {
    const onClear = vi.fn();
    render(
      <SearchForm query="test" onQueryChange={vi.fn()} onClear={onClear} />
    );

    fireEvent.click(screen.getByLabelText("entries.search.clearSearch"));
    expect(onClear).toHaveBeenCalledOnce();
  });

  it("calls onEscape when Escape key is pressed", () => {
    const onEscape = vi.fn();
    render(
      <SearchForm
        query="test"
        onQueryChange={vi.fn()}
        onClear={vi.fn()}
        onEscape={onEscape}
      />
    );

    const input = screen.getByPlaceholderText("entries.search.placeholder");
    fireEvent.keyDown(input, { key: "Escape" });
    expect(onEscape).toHaveBeenCalledOnce();
  });

  it("sets autoFocus on input when prop is true", () => {
    render(
      <SearchForm
        query=""
        onQueryChange={vi.fn()}
        onClear={vi.fn()}
        autoFocus
      />
    );

    const input = screen.getByPlaceholderText("entries.search.placeholder");
    expect(input).toHaveFocus();
  });

  it("prevents form submission", () => {
    const { container } = render(
      <SearchForm query="" onQueryChange={vi.fn()} onClear={vi.fn()} />
    );

    const form = container.querySelector("form")!;
    const submitEvent = new Event("submit", {
      bubbles: true,
      cancelable: true,
    });
    const prevented = !form.dispatchEvent(submitEvent);
    expect(prevented).toBe(true);
  });

  it("applies custom input id", () => {
    render(
      <SearchForm
        query=""
        onQueryChange={vi.fn()}
        onClear={vi.fn()}
        inputId="custom-search-input"
      />
    );

    expect(screen.getByLabelText("entries.search.label")).toHaveAttribute(
      "id",
      "custom-search-input"
    );
  });
});
