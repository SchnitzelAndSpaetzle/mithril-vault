import { useState } from "react";
import { useForm, type Resolver } from "react-hook-form";
import { standardSchemaResolver } from "@hookform/resolvers/standard-schema";
import { useNavigate } from "@tanstack/react-router";
import { ArrowLeft, ArrowRight, Loader2, ShieldAlert } from "lucide-react";
import { ask } from "@tauri-apps/plugin-dialog";
import { toast } from "sonner";

import {
  createDatabaseSchema,
  type CreateDatabaseFormValues,
  type KeyfileMode,
} from "@/lib/formTypes";
import { database, keyfile, settings } from "@/lib/tauri";
import {
  type DatabaseTabsState,
  useDatabaseTabs,
} from "@/stores/database-tabs";
import { Button } from "@/components/ui/button";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";

import { LocationStep } from "./steps/LocationStep";
import { DatabaseInfoStep } from "./steps/DatabaseInfoStep";
import { MasterPasswordStep } from "./steps/MasterPasswordStep";
import { KeyFileStep } from "./steps/KeyFileStep";
import { ReviewStep } from "./steps/ReviewStep";

const WIZARD_STEPS = [
  { id: "location", title: "Location", fields: ["filePath"] as const },
  { id: "info", title: "Database Info", fields: ["name"] as const },
  {
    id: "password",
    title: "Master Password",
    fields: ["password", "confirmPassword"] as const,
  },
  { id: "keyfile", title: "Key File", fields: [] as const },
  { id: "review", title: "Review", fields: [] as const },
];

function mapErrorToMessage(error: unknown): string {
  const errorStr = String(error);

  if (errorStr.includes("already exists") || errorStr.includes("File exists")) {
    return "A file already exists at this location. Please choose a different path.";
  }
  if (
    errorStr.includes("Permission denied") ||
    errorStr.includes("permission denied")
  ) {
    return "Permission denied. Please check that you have write access to this location.";
  }
  if (errorStr.includes("No credentials provided")) {
    return "Please set a master password or select a key file.";
  }
  if (errorStr.includes("Parent directory does not exist")) {
    return "The selected directory does not exist. Please choose a valid location.";
  }
  if (errorStr.includes("IO error") || errorStr.includes("No such file")) {
    return "Could not write to the specified location. Please check the path and try again.";
  }

  return "Failed to create database. Please try again.";
}

export function CreateDatabaseWizard() {
  const navigate = useNavigate();
  const addTab = useDatabaseTabs((state: DatabaseTabsState) => state.addTab);
  const updateTabInfo = useDatabaseTabs(
    (state: DatabaseTabsState) => state.updateTabInfo
  );
  const setActiveTab = useDatabaseTabs(
    (state: DatabaseTabsState) => state.setActiveTab
  );

  const [currentStep, setCurrentStep] = useState(0);
  const [isCreating, setIsCreating] = useState(false);
  const [createError, setCreateError] = useState<string | null>(null);

  const form = useForm<CreateDatabaseFormValues>({
    resolver: standardSchemaResolver(
      createDatabaseSchema
    ) as Resolver<CreateDatabaseFormValues>,
    defaultValues: {
      filePath: "",
      name: "",
      description: "",
      password: "",
      confirmPassword: "",
      keyfileMode: "none",
      keyfilePath: "",
      createDefaultGroups: true,
    },
    mode: "onTouched",
  });

  // currentStepConfig is guaranteed to exist because currentStep is bounded by WIZARD_STEPS.length
  const currentStepConfig = WIZARD_STEPS[currentStep]!;
  const isFirstStep = currentStep === 0;
  const isLastStep = currentStep === WIZARD_STEPS.length - 1;

  async function handleNext() {
    // Validate current step's fields
    const fieldsToValidate = currentStepConfig.fields;

    if (fieldsToValidate.length > 0) {
      const isValid = await form.trigger([
        ...fieldsToValidate,
      ] as (keyof CreateDatabaseFormValues)[]);
      if (!isValid) return;
    }

    // Special validation for password step - check password match
    if (currentStepConfig.id === "password") {
      const password = form.getValues("password");
      const confirmPassword = form.getValues("confirmPassword");
      if (password && password !== confirmPassword) {
        form.setError("confirmPassword", {
          message: "Passwords do not match.",
        });
        return;
      }
    }

    setCurrentStep((prev) => Math.min(prev + 1, WIZARD_STEPS.length - 1));
  }

  function handleBack() {
    setCurrentStep((prev) => Math.max(prev - 1, 0));
  }

  async function handleCancel() {
    const hasData =
      form.getValues("filePath") ||
      form.getValues("name") ||
      form.getValues("password");

    if (hasData) {
      const confirmed = await ask(
        "Are you sure you want to cancel? Any entered information will be lost.",
        {
          title: "Cancel Database Creation",
          kind: "warning",
        }
      );

      if (!confirmed) return;
    }

    navigate({ to: "/" });
  }

  async function onSubmit(data: CreateDatabaseFormValues) {
    setIsCreating(true);
    setCreateError(null);

    try {
      // If generating keyfile, do it first
      if (data.keyfileMode === "generate" && data.keyfilePath) {
        await keyfile.generate(data.keyfilePath);
      }

      const info = await database.create(
        data.filePath,
        data.name,
        data.password || undefined,
        data.keyfileMode !== "none" && data.keyfilePath
          ? data.keyfilePath
          : undefined,
        {
          description: data.description || undefined,
          createDefaultGroups: data.createDefaultGroups,
        }
      );

      // Add to recent databases
      try {
        await settings.addRecentDatabase(
          data.filePath,
          data.keyfileMode !== "none" ? data.keyfilePath : undefined
        );
      } catch (error) {
        console.warn("Failed to update recent database list", error);
      }

      // Add tab and navigate
      const tabId = addTab(data.filePath);
      updateTabInfo(tabId, info);
      setActiveTab(tabId);

      toast.success("Database created successfully!");

      navigate({ to: "/dashboard/index/$dbId", params: { dbId: info.path } });
    } catch (error) {
      setCreateError(mapErrorToMessage(error));
    } finally {
      setIsCreating(false);
    }
  }

  function setKeyfileValue(
    name: "keyfileMode" | "keyfilePath",
    value: KeyfileMode | string
  ) {
    form.setValue(name, value as never, { shouldValidate: true });
  }

  return (
    <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-6">
      {/* Progress indicator */}
      <div className="text-center text-sm text-muted-foreground">
        Step {currentStep + 1} of {WIZARD_STEPS.length} -{" "}
        {currentStepConfig.title}
      </div>

      {/* Step progress bar */}
      <div className="flex gap-1">
        {WIZARD_STEPS.map((step, index) => (
          <div
            key={step.id}
            className={`h-1 flex-1 rounded-full transition-colors ${
              index <= currentStep ? "bg-primary" : "bg-muted"
            }`}
          />
        ))}
      </div>

      {/* Step content */}
      <div className="min-h-[300px]">
        {currentStepConfig.id === "location" && (
          <LocationStep control={form.control} disabled={isCreating} />
        )}
        {currentStepConfig.id === "info" && (
          <DatabaseInfoStep control={form.control} disabled={isCreating} />
        )}
        {currentStepConfig.id === "password" && (
          <MasterPasswordStep control={form.control} disabled={isCreating} />
        )}
        {currentStepConfig.id === "keyfile" && (
          <KeyFileStep
            control={form.control}
            setValue={setKeyfileValue}
            disabled={isCreating}
          />
        )}
        {currentStepConfig.id === "review" && (
          <ReviewStep control={form.control} disabled={isCreating} />
        )}
      </div>

      {/* Error display */}
      {createError && (
        <Alert variant="destructive">
          <ShieldAlert />
          <AlertTitle>Error creating database</AlertTitle>
          <AlertDescription>{createError}</AlertDescription>
        </Alert>
      )}

      {/* Navigation buttons */}
      <div className="flex items-center justify-between pt-4 border-t">
        <Button
          type="button"
          variant="ghost"
          onClick={handleCancel}
          disabled={isCreating}
        >
          Cancel
        </Button>

        <div className="flex items-center gap-2">
          {!isFirstStep && (
            <Button
              type="button"
              variant="outline"
              onClick={handleBack}
              disabled={isCreating}
            >
              <ArrowLeft className="size-4 mr-1" />
              Back
            </Button>
          )}

          {isLastStep ? (
            <Button type="submit" disabled={isCreating}>
              {isCreating ? (
                <>
                  <Loader2 className="size-4 mr-1 animate-spin" />
                  Creating...
                </>
              ) : (
                "Create Database"
              )}
            </Button>
          ) : (
            <Button type="button" onClick={handleNext} disabled={isCreating}>
              Next
              <ArrowRight className="size-4 ml-1" />
            </Button>
          )}
        </div>
      </div>
    </form>
  );
}
