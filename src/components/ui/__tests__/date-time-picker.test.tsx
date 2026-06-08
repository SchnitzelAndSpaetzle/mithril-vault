// SPDX-License-Identifier: MIT

import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { DateTimePicker } from "@/components/ui/date-time-picker";

describe("DateTimePicker", () => {
  it("renders the placeholder on the trigger when no value is set", () => {
    render(
      <DateTimePicker
        value={undefined}
        onChange={vi.fn()}
        placeholder="Pick a date"
      />
    );

    expect(
      screen.getByRole("button", { name: /pick a date/i })
    ).toBeInTheDocument();
  });

  it("renders the value formatted with dayjs on the trigger", () => {
    // 2026-06-08 15:45 local time
    const value = new Date(2026, 5, 8, 15, 45);

    render(<DateTimePicker value={value} onChange={vi.fn()} />);

    expect(
      screen.getByRole("button", { name: "Jun 8, 2026 3:45 PM" })
    ).toBeInTheDocument();
  });

  it("opens a popover with a calendar and a time input when the trigger is clicked", () => {
    render(
      <DateTimePicker
        value={undefined}
        onChange={vi.fn()}
        placeholder="Pick a date"
      />
    );

    expect(screen.queryByRole("grid")).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /pick a date/i }));

    expect(screen.getByRole("grid")).toBeInTheDocument();
    expect(screen.getByLabelText(/time/i)).toBeInTheDocument();
  });

  it("emits a minute-precision date when the time input changes", () => {
    const onChange = vi.fn();
    const value = new Date(2026, 5, 8, 15, 45);

    render(<DateTimePicker value={value} onChange={onChange} />);

    fireEvent.click(
      screen.getByRole("button", { name: "Jun 8, 2026 3:45 PM" })
    );
    fireEvent.change(screen.getByLabelText(/time/i), {
      target: { value: "09:30" },
    });

    expect(onChange).toHaveBeenCalledTimes(1);
    const emitted = onChange.mock.calls[0]?.[0] as Date;
    expect(emitted.getFullYear()).toBe(2026);
    expect(emitted.getMonth()).toBe(5);
    expect(emitted.getDate()).toBe(8);
    expect(emitted.getHours()).toBe(9);
    expect(emitted.getMinutes()).toBe(30);
    expect(emitted.getSeconds()).toBe(0);
  });

  it("keeps the existing time-of-day when a new day is picked", () => {
    const onChange = vi.fn();
    const value = new Date(2026, 5, 8, 15, 45);

    render(<DateTimePicker value={value} onChange={onChange} />);

    fireEvent.click(
      screen.getByRole("button", { name: "Jun 8, 2026 3:45 PM" })
    );
    fireEvent.click(screen.getByText("20"));

    const emitted = onChange.mock.calls[0]?.[0] as Date;
    expect(emitted.getFullYear()).toBe(2026);
    expect(emitted.getMonth()).toBe(5);
    expect(emitted.getDate()).toBe(20);
    // Time-of-day is preserved from the previous value.
    expect(emitted.getHours()).toBe(15);
    expect(emitted.getMinutes()).toBe(45);
  });

  it("clears the value when the selected day is deselected", () => {
    const onChange = vi.fn();
    const value = new Date(2026, 5, 8, 15, 45);

    render(<DateTimePicker value={value} onChange={onChange} />);

    fireEvent.click(
      screen.getByRole("button", { name: "Jun 8, 2026 3:45 PM" })
    );
    // Clicking the already-selected day deselects it.
    fireEvent.click(screen.getByText("8"));

    expect(onChange).toHaveBeenCalledWith(undefined);
  });

  it("uses the current time-of-day when picking a day with no prior value", () => {
    const onChange = vi.fn();

    render(
      <DateTimePicker
        value={undefined}
        onChange={onChange}
        placeholder="Pick a date"
      />
    );

    fireEvent.click(screen.getByRole("button", { name: /pick a date/i }));
    fireEvent.click(screen.getByText("20"));

    const emitted = onChange.mock.calls[0]?.[0] as Date;
    expect(emitted).toBeInstanceOf(Date);
    expect(emitted.getDate()).toBe(20);
    // Seconds are always zeroed to keep minute precision.
    expect(emitted.getSeconds()).toBe(0);
  });

  it("ignores a cleared time input without emitting a value", () => {
    const onChange = vi.fn();
    const value = new Date(2026, 5, 8, 15, 45);

    render(<DateTimePicker value={value} onChange={onChange} />);

    fireEvent.click(
      screen.getByRole("button", { name: "Jun 8, 2026 3:45 PM" })
    );
    fireEvent.change(screen.getByLabelText(/time/i), {
      target: { value: "" },
    });

    expect(onChange).not.toHaveBeenCalled();
  });
});
