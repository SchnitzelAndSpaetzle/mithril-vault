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

export function InputGroupPassword() {
  const [showPassword, setShowPassword] = useState(false);
  return (
    <div className="grid w-full max-w-md gap-6">
      <InputGroup>
        <div className="flex w-full py-4">
          <InputGroupInput
            id="password"
            type={showPassword ? "text" : "password"}
            placeholder="enter password here..."
            className="relative flex"
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
          <InputGroupButton size="sm" className="ml-auto" variant="default">
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

      <Alert variant="destructive">
        <ShieldAlert />
        <AlertTitle>Error reading database</AlertTitle>
        <AlertDescription>Wrong credentials were provided</AlertDescription>
      </Alert>
    </div>
  );
}
