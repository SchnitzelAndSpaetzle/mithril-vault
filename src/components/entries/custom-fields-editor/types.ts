import type { Control } from "react-hook-form";
import type { EntryFormValues } from "@/lib/formTypes";

export interface CustomFieldsEditorProps {
  control: Control<EntryFormValues>;
  disabled?: boolean;
}

export interface CustomFieldRowProps {
  index: number;
  control: Control<EntryFormValues>;
  onRemove: () => void;
  disabled: boolean;
}

export interface CustomFieldValueInputProps {
  index: number;
  control: Control<EntryFormValues>;
  disabled: boolean;
}
