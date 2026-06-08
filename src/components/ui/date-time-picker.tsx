// SPDX-License-Identifier: MIT

import * as React from "react";
import dayjs from "dayjs";
import { CalendarIcon } from "lucide-react";

import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { Calendar } from "@/components/ui/calendar";
import { Input } from "@/components/ui/input";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";

export interface DateTimePickerProps {
  value?: Date | undefined;
  onChange: (date: Date | undefined) => void;
  placeholder?: string;
  disabled?: boolean;
  id?: string;
}

const DISPLAY_FORMAT = "MMM D, YYYY h:mm A";
const TIME_FORMAT = "HH:mm";

function DateTimePicker({
  value,
  onChange,
  placeholder,
  disabled,
  id,
}: Readonly<DateTimePickerProps>) {
  const timeValue = value ? dayjs(value).format(TIME_FORMAT) : "";

  function handleDaySelect(day: Date | undefined) {
    if (!day) {
      onChange(undefined);
      return;
    }

    // Keep the existing time-of-day; default to "now" on the first pick.
    const base = value ? dayjs(value) : dayjs();
    const next = dayjs(day)
      .hour(base.hour())
      .minute(base.minute())
      .second(0)
      .millisecond(0);

    onChange(next.toDate());
  }

  function handleTimeChange(event: React.ChangeEvent<HTMLInputElement>) {
    const [hours, minutes] = event.target.value.split(":");
    if (hours === undefined || minutes === undefined) {
      return;
    }

    const base = value ? dayjs(value) : dayjs();
    const next = base
      .hour(Number(hours))
      .minute(Number(minutes))
      .second(0)
      .millisecond(0);

    onChange(next.toDate());
  }

  return (
    <Popover>
      <PopoverTrigger asChild>
        <Button
          id={id}
          type="button"
          variant="outline"
          disabled={disabled}
          data-slot="date-time-picker-trigger"
          className={cn(
            "w-full justify-start text-left font-normal",
            !value && "text-muted-foreground"
          )}
        >
          <CalendarIcon className="mr-2 size-4" />
          {value ? dayjs(value).format(DISPLAY_FORMAT) : placeholder}
        </Button>
      </PopoverTrigger>
      <PopoverContent className="w-auto p-0" align="start">
        <Calendar mode="single" selected={value} onSelect={handleDaySelect} />
        <div className="border-t p-3">
          <Input
            type="time"
            aria-label="Time"
            value={timeValue}
            onChange={handleTimeChange}
          />
        </div>
      </PopoverContent>
    </Popover>
  );
}

export { DateTimePicker };
