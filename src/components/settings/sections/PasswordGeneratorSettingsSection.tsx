// SPDX-License-Identifier: MIT

import { useTranslation } from "react-i18next";
import { SettingsSection } from "@/components/settings/SettingsSection";
import { PasswordGenerator } from "@/components/generator/PasswordGenerator";

export function PasswordGeneratorSettingsSection() {
  const { t } = useTranslation();

  return (
    <SettingsSection
      id="generator"
      title={t("settings.generator.title")}
      description={t("settings.generator.description")}
    >
      <PasswordGenerator />
    </SettingsSection>
  );
}
