import { useState } from "react";
import { type Resolver, useForm } from "react-hook-form";
import { standardSchemaResolver } from "@hookform/resolvers/standard-schema";
import { useNavigate } from "@tanstack/react-router";
import { useTranslation } from "react-i18next";
import { ArrowLeft, ArrowRight, Loader2, ShieldAlert } from "lucide-react";
import { ask } from "@tauri-apps/plugin-dialog";
import { toast } from "sonner";
import type { TFunction } from "i18next";

import {
  type CreateDatabaseFormValues,
  createDatabaseSchema,
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

function mapErrorToMessage(error: unknown, t: TFunction): string {
  const errorStr = String(error);

  if (errorStr.includes("already exists") || errorStr.includes("File exists")) {
    return t("createDatabase.errors.fileExists");
  }
  if (
    errorStr.includes("Permission denied") ||
    errorStr.includes("permission denied")
  ) {
    return t("createDatabase.errors.permissionDenied");
  }
  if (errorStr.includes("No credentials provided")) {
    return t("createDatabase.errors.noCredentials");
  }
  if (errorStr.includes("Parent directory does not exist")) {
    return t("createDatabase.errors.directoryNotFound");
  }
  if (errorStr.includes("IO error") || errorStr.includes("No such file")) {
    return t("createDatabase.errors.ioError");
  }

  return t("createDatabase.errors.generic");
}

export function CreateDatabaseWizard() {
  const { t } = useTranslation();
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

  const WIZARD_STEPS = [
    {
      id: "location",
      titleKey: "createDatabase.steps.location" as const,
      fields: ["filePath"] as const,
    },
    {
      id: "info",
      titleKey: "createDatabase.steps.info" as const,
      fields: ["name"] as const,
    },
    {
      id: "password",
      titleKey: "createDatabase.steps.password" as const,
      fields: ["password", "confirmPassword"] as const,
    },
    {
      id: "keyfile",
      titleKey: "createDatabase.steps.keyfile" as const,
      fields: [] as const,
    },
    {
      id: "review",
      titleKey: "createDatabase.steps.review" as const,
      fields: [] as const,
    },
  ];

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
    const fieldsToValidate = currentStepConfig.fields;

    if (fieldsToValidate.length > 0) {
      const isValid = await form.trigger([
        ...fieldsToValidate,
      ] as (keyof CreateDatabaseFormValues)[]);
      if (!isValid) return;
    }

    if (currentStepConfig.id === "password") {
      const password = form.getValues("password");
      const confirmPassword = form.getValues("confirmPassword");
      if (password && password !== confirmPassword) {
        form.setError("confirmPassword", {
          message: t("createDatabase.password.mismatch"),
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
      const confirmed = await ask(t("createDatabase.cancelConfirm"), {
        title: t("createDatabase.cancelTitle"),
        kind: "warning",
      });

      if (!confirmed) return;
    }

    await navigate({ to: "/" });
  }

  async function onSubmit(data: CreateDatabaseFormValues) {
    setIsCreating(true);
    setCreateError(null);

    try {
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

      try {
        await settings.addRecentDatabase(
          data.filePath,
          data.keyfileMode !== "none" ? data.keyfilePath : undefined
        );
      } catch (error) {
        console.warn("Failed to update recent database list", error);
      }

      const tabId = addTab(data.filePath);
      updateTabInfo(tabId, info);
      setActiveTab(tabId);

      toast.success(t("createDatabase.toast.created"));

      await navigate({
        to: "/dashboard/index/$dbId",
        params: { dbId: info.path },
      });
    } catch (error) {
      setCreateError(mapErrorToMessage(error, t));
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
        {t("createDatabase.stepProgress", {
          current: currentStep + 1,
          total: WIZARD_STEPS.length,
          title: t(currentStepConfig.titleKey),
        })}
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
      <div className="min-h-75">
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
          <AlertTitle>{t("createDatabase.errorTitle")}</AlertTitle>
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
          {t("common.cancel")}
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
              {t("common.back")}
            </Button>
          )}

          {isLastStep ? (
            <Button type="submit" disabled={isCreating}>
              {isCreating ? (
                <>
                  <Loader2 className="size-4 mr-1 animate-spin" />
                  {t("createDatabase.creating")}
                </>
              ) : (
                t("createDatabase.createDatabase")
              )}
            </Button>
          ) : (
            <Button type="button" onClick={handleNext} disabled={isCreating}>
              {t("common.next")}
              <ArrowRight className="size-4 ml-1" />
            </Button>
          )}
        </div>
      </div>
    </form>
  );
}
