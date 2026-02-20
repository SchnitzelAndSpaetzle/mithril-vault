import { type Control, Controller } from "react-hook-form";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Field, FieldLabel } from "@/components/ui/field";
import { useGroups } from "@/hooks/use-groups";
import type { EntryFormValues } from "@/lib/formTypes";
import type { Group } from "@/lib/types";

interface EntryGroupFieldProps {
  control: Control<EntryFormValues>;
  dbId: string;
  isPending: boolean;
}

interface FlatGroup {
  id: string;
  name: string;
  depth: number;
}

function flattenGroups(groups: Group[], depth = 0): FlatGroup[] {
  const result: FlatGroup[] = [];
  for (const group of groups) {
    result.push({ id: group.id, name: group.name, depth });
    if (group.children.length > 0) {
      result.push(...flattenGroups(group.children, depth + 1));
    }
  }
  return result;
}

export function EntryGroupField({
  control,
  dbId,
  isPending,
}: EntryGroupFieldProps) {
  const { data: groups } = useGroups(dbId);
  const flatGroups = groups ? flattenGroups(groups) : [];

  if (flatGroups.length <= 1) return null;

  return (
    <Field>
      <FieldLabel htmlFor="groupId">Group</FieldLabel>
      <Controller
        name="groupId"
        control={control}
        render={({ field }) => (
          <Select
            value={field.value ?? ""}
            onValueChange={field.onChange}
            disabled={isPending}
          >
            <SelectTrigger className="w-full" id="groupId">
              <SelectValue placeholder="Select a group" />
            </SelectTrigger>
            <SelectContent>
              {flatGroups.map((group) => (
                <SelectItem key={group.id} value={group.id}>
                  <span style={{ paddingLeft: `${group.depth * 12}px` }}>
                    {group.name}
                  </span>
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        )}
      />
    </Field>
  );
}
