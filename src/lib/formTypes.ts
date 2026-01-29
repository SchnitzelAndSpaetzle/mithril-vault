import { z } from "zod/v4";

export const openDatabaseSchema = z.object({
  filePath: z.string("File path is required."),
  password: z.string().optional(),
  keyfilePath: z.string().optional(),
});

export type OpenDatabaseFormValues = z.infer<typeof openDatabaseSchema>;
