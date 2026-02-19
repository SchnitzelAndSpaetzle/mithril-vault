// SPDX-License-Identifier: MIT

import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { TagInput } from "../TagInput";

describe("TagInput", () => {
  it("renders existing tags as badges", () => {
    render(<TagInput value={["work", "dev"]} onChange={vi.fn()} />);

    expect(screen.getByText("work")).toBeInTheDocument();
    expect(screen.getByText("dev")).toBeInTheDocument();
  });

  it("adds a tag on Enter", () => {
    const onChange = vi.fn();
    render(<TagInput value={["work"]} onChange={onChange} />);

    const input = screen.getByPlaceholderText("");
    fireEvent.change(input, { target: { value: "new-tag" } });
    fireEvent.keyDown(input, { key: "Enter" });

    expect(onChange).toHaveBeenCalledWith(["work", "new-tag"]);
  });

  it("does not add duplicate tags", () => {
    const onChange = vi.fn();
    render(<TagInput value={["work"]} onChange={onChange} />);

    const input = screen.getByPlaceholderText("");
    fireEvent.change(input, { target: { value: "work" } });
    fireEvent.keyDown(input, { key: "Enter" });

    expect(onChange).not.toHaveBeenCalled();
  });

  it("removes a tag when clicking the remove button", () => {
    const onChange = vi.fn();
    render(<TagInput value={["work", "dev"]} onChange={onChange} />);

    fireEvent.click(screen.getByRole("button", { name: "Remove tag work" }));
    expect(onChange).toHaveBeenCalledWith(["dev"]);
  });

  it("removes last tag on Backspace when input is empty", () => {
    const onChange = vi.fn();
    render(<TagInput value={["work", "dev"]} onChange={onChange} />);

    const input = screen.getByPlaceholderText("");
    fireEvent.keyDown(input, { key: "Backspace" });

    expect(onChange).toHaveBeenCalledWith(["work"]);
  });

  it("shows placeholder when no tags exist", () => {
    render(<TagInput value={[]} onChange={vi.fn()} />);
    expect(screen.getByPlaceholderText("Add tags...")).toBeInTheDocument();
  });

  it("shows matching suggestions and adds selected suggestion", () => {
    const onChange = vi.fn();
    render(
      <TagInput
        value={["work"]}
        onChange={onChange}
        suggestions={["home", "dev", "office"]}
      />
    );

    const input = screen.getByPlaceholderText("");
    fireEvent.focus(input);
    fireEvent.change(input, { target: { value: "de" } });

    fireEvent.click(screen.getByRole("option", { name: "dev" }));
    expect(onChange).toHaveBeenCalledWith(["work", "dev"]);
  });

  it("still allows creating a new tag when no suggestions match", () => {
    const onChange = vi.fn();
    render(
      <TagInput value={[]} onChange={onChange} suggestions={["work", "dev"]} />
    );

    const input = screen.getByPlaceholderText("Add tags...");
    fireEvent.change(input, { target: { value: "personal" } });
    fireEvent.keyDown(input, { key: "Enter" });

    expect(onChange).toHaveBeenCalledWith(["personal"]);
  });

  it("selects highlighted suggestion with ArrowDown + Enter", () => {
    const onChange = vi.fn();
    render(
      <TagInput
        value={[]}
        onChange={onChange}
        suggestions={["home", "dev", "office"]}
      />
    );

    const input = screen.getByPlaceholderText("Add tags...");
    fireEvent.focus(input);
    fireEvent.change(input, { target: { value: "o" } });
    fireEvent.keyDown(input, { key: "ArrowDown" });
    fireEvent.keyDown(input, { key: "Enter" });

    expect(onChange).toHaveBeenCalledWith(["home"]);
  });

  it("moves highlight with ArrowUp and picks suggestion on Enter", () => {
    const onChange = vi.fn();
    render(
      <TagInput
        value={[]}
        onChange={onChange}
        suggestions={["home", "dev", "office"]}
      />
    );

    const input = screen.getByPlaceholderText("Add tags...");
    fireEvent.focus(input);
    fireEvent.change(input, { target: { value: "o" } });
    fireEvent.keyDown(input, { key: "ArrowUp" });
    fireEvent.keyDown(input, { key: "Enter" });

    expect(onChange).toHaveBeenCalledWith(["office"]);
  });
});
