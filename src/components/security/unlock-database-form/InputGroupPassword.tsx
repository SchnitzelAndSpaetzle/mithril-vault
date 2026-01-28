import {
  CornerDownLeft,
  Eye,
  EyeClosed,
  FolderOpen,
  KeyRound,
  ShieldAlert,
} from "lucide-react";
import {
  InputGroup,
  InputGroupAddon,
  InputGroupButton,
  InputGroupInput,
  InputGroupText,
} from "@/components/ui/input-group.tsx";
import { useState } from "react";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert.tsx";
import { Controller, useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import {
  type OpenDatabaseFormValues,
  openDatabaseSchema,
} from "@/lib/formTypes.ts";
import { useNavigate } from "@tanstack/react-router";

export function InputGroupPassword() {
  const navigate = useNavigate({ from: "/login" });
  const [showPassword, setShowPassword] = useState(false);

  const openDbForm = useForm<OpenDatabaseFormValues>({
    resolver: zodResolver(openDatabaseSchema),
    defaultValues: {
      filePath: "", // TODO: Should be set from file picker or props
      password: "",
      keyfilePath: "",
    },
  });

  const {
    formState: { errors },
  } = openDbForm;

  async function onSubmit(data: OpenDatabaseFormValues) {
    console.log(data);

    // TODO: implement unlocking database
    await navigate({
      to: "/dashboard",
    });
  }

  return (
    <form id="open-db-form" onSubmit={openDbForm.handleSubmit(onSubmit)}>
      <div className="grid w-full max-w-md gap-6">
        <InputGroup>
          <div className="flex w-full py-4">
            <Controller
              name="password"
              control={openDbForm.control}
              render={({ field, fieldState }) => (
                <InputGroupInput
                  {...field}
                  id={field.name}
                  aria-invalid={fieldState.invalid}
                  type={showPassword ? "text" : "password"}
                  placeholder="enter password here..."
                  autoComplete="off"
                  className="relative flex"
                />
              )}
            />

            <InputGroupAddon align="inline-end" className="ml-auto inline-flex">
              <InputGroupButton
                className="ml-auto"
                variant="ghost"
                aria-label="Info"
                size="icon-xs"
                onClick={() => setShowPassword((prev) => !prev)}
              >
                {showPassword ? <Eye /> : <EyeClosed />}
              </InputGroupButton>
            </InputGroupAddon>
          </div>

          <InputGroupAddon align="block-end" className="border-t">
            <InputGroupButton size="sm" variant="ghost">
              add key file <KeyRound />
            </InputGroupButton>
            <InputGroupButton
              type="submit"
              form="open-db-form"
              size="sm"
              className="ml-auto"
              variant="default"
            >
              Unlock <CornerDownLeft />
            </InputGroupButton>
          </InputGroupAddon>

          <InputGroupAddon align="block-start" className="border-b">
            <InputGroupText className="font-mono font-medium">
              <FolderOpen />
              MainDatabase-example.kdbx
            </InputGroupText>
          </InputGroupAddon>

          {/*TODO: if key file is selected*/}
          <InputGroupAddon align="block-start" className="border-b">
            <InputGroupText className="font-mono font-medium">
              <KeyRound />
              keyFile-example.key
            </InputGroupText>
          </InputGroupAddon>
        </InputGroup>

        {errors.filePath && (
          <Alert variant="destructive">
            <ShieldAlert />
            <AlertTitle>Error reading database</AlertTitle>
            <AlertDescription>Wrong credentials were provided</AlertDescription>
          </Alert>
        )}
      </div>
    </form>
  );
}
