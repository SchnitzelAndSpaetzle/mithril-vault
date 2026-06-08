import { z } from "zod/v4";

export const openDatabaseSchema = z.object({
  filePath: z.string("File path is required."),
  password: z.string().optional(),
  keyfilePath: z.string().optional(),
});

export type OpenDatabaseFormValues = z.infer<typeof openDatabaseSchema>;

export const keyfileModeSchema = z.enum(["none", "select", "generate"]);
export type KeyfileMode = z.infer<typeof keyfileModeSchema>;

// Base schema without refinements (used for form type inference)
const createDatabaseBaseSchema = z.object({
  filePath: z.string().min(1, "File location is required."),
  name: z.string().min(1, "Database name is required."),
  description: z.string().optional(),
  password: z.string().optional(),
  confirmPassword: z.string().optional(),
  keyfileMode: keyfileModeSchema.default("none"),
  keyfilePath: z.string().optional(),
  createDefaultGroups: z.boolean().default(true),
});

// Full schema with refinements (used for validation)
export const createDatabaseSchema = createDatabaseBaseSchema
  .refine(
    (data) => {
      // Password confirmation must match if password is provided
      if (data.password && data.password !== data.confirmPassword) {
        return false;
      }
      return true;
    },
    {
      message: "Passwords do not match.",
      path: ["confirmPassword"],
    }
  )
  .refine(
    (data) => {
      // Must have password OR keyfile
      const hasPassword = data.password && data.password.length > 0;
      const hasKeyfile = data.keyfileMode !== "none" && data.keyfilePath;
      return hasPassword || hasKeyfile;
    },
    {
      message: "You must provide a password or a key file.",
      path: ["password"],
    }
  );

// Use the base schema for type inference to avoid type mismatch with react-hook-form
export type CreateDatabaseFormValues = z.infer<typeof createDatabaseBaseSchema>;

// Entry edit/create form schemas
export const entryCustomFieldSchema = z.object({
  key: z.string().min(1, "Field name is required."),
  value: z.string(),
  isProtected: z.boolean(),
});

export type EntryCustomField = z.infer<typeof entryCustomFieldSchema>;

const entryFormBaseSchema = z.object({
  title: z.string().min(1, "Title is required."),
  username: z.string(),
  password: z.string(),
  url: z.string(),
  notes: z.string(),
  iconId: z.number().int(),
  customIconUuid: z.string().nullable(),
  tags: z.array(z.string()),
  customFields: z.array(entryCustomFieldSchema),
  groupId: z.string().optional(),
  expires: z.boolean(),
  // Held as a Date in the form (matches DateTimePicker); converted to UTC ISO
  // 8601 at the IPC boundary. Past dates are valid.
  expiryTime: z.date().nullable(),
});

export const entryFormSchema = entryFormBaseSchema
  .refine((data) => data.url === "" || /^https?:\/\/.+/.test(data.url), {
    message: "Must be a valid URL.",
    path: ["url"],
  })
  .refine((data) => !data.expires || data.expiryTime !== null, {
    message: "An expiry date is required when expiry is enabled.",
    path: ["expiryTime"],
  });

// Use the base schema for type inference to avoid type mismatch with react-hook-form
export type EntryFormValues = z.infer<typeof entryFormBaseSchema>;
