// SPDX-License-Identifier: MIT

import { z } from "zod/v4";

export const DatabaseInfoSchema = z.object({
  name: z.string(),
  path: z.string(),
  isModified: z.boolean(),
  rootGroupId: z.string(),
  version: z.string(),
});
export type DatabaseInfo = z.infer<typeof DatabaseInfoSchema>;

export const CustomFieldMetaSchema = z.object({
  key: z.string(),
  isProtected: z.boolean(),
});
export type CustomFieldMeta = z.infer<typeof CustomFieldMetaSchema>;

export const EntrySchema = z.object({
  id: z.string(),
  groupId: z.string(),
  title: z.string(),
  username: z.string(),
  url: z.string().optional(),
  notes: z.string().optional(),
  iconId: z.number().int().optional(),
  tags: z.array(z.string()),
  customFields: z.record(z.string(), z.string()),
  customFieldMeta: z.array(CustomFieldMetaSchema),
  createdAt: z.string(),
  modifiedAt: z.string(),
  accessedAt: z.string(),
});
export type Entry = z.infer<typeof EntrySchema>;

export interface Group {
  id: string;
  parentId?: string | undefined;
  name: string;
  icon?: string | undefined;
  children: Group[];
}

export const GroupSchema: z.ZodType<Group> = z.lazy(() =>
  z.object({
    id: z.string(),
    parentId: z.string().optional(),
    name: z.string(),
    icon: z.string().optional(),
    children: z.array(GroupSchema),
  })
);

export const PasswordGeneratorOptionsSchema = z.object({
  length: z.number().int().min(1).max(128),
  uppercase: z.boolean(),
  lowercase: z.boolean(),
  numbers: z.boolean(),
  symbols: z.boolean(),
  excludeAmbiguous: z.boolean(),
  excludeChars: z.string().optional(),
});
export type PasswordGeneratorOptions = z.infer<
  typeof PasswordGeneratorOptionsSchema
>;

export const CreateEntryDataSchema = z.object({
  title: z.string().min(1),
  username: z.string(),
  password: z.string(),
  url: z.string().optional(),
  notes: z.string().optional(),
  iconId: z.number().int().optional(),
  tags: z.array(z.string()).optional(),
  customFields: z.record(z.string(), z.string()).optional(),
  protectedCustomFields: z.record(z.string(), z.string()).optional(),
});
export type CreateEntryData = z.infer<typeof CreateEntryDataSchema>;

export const UpdateEntryDataSchema = z.object({
  title: z.string().min(1).optional(),
  username: z.string().optional(),
  password: z.string().optional(),
  url: z.string().optional(),
  notes: z.string().optional(),
  iconId: z.number().int().optional(),
  tags: z.array(z.string()).optional(),
  customFields: z.record(z.string(), z.string()).optional(),
  protectedCustomFields: z.record(z.string(), z.string()).optional(),
});
export type UpdateEntryData = z.infer<typeof UpdateEntryDataSchema>;

export const CustomFieldValueSchema = z.object({
  key: z.string(),
  value: z.string(),
});
export type CustomFieldValue = z.infer<typeof CustomFieldValueSchema>;

export const DatabaseCreationOptionsSchema = z.object({
  description: z.string().optional(),
  createDefaultGroups: z.boolean().optional(),
  kdfMemory: z.number().int().positive().optional(),
  kdfIterations: z.number().int().positive().optional(),
  kdfParallelism: z.number().int().positive().optional(),
});
export type DatabaseCreationOptions = z.infer<
  typeof DatabaseCreationOptionsSchema
>;
