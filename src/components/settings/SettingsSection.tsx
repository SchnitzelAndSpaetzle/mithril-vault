// SPDX-License-Identifier: MIT

import type { ReactNode } from "react";

interface SettingsSectionProps {
  id: string;
  title: string;
  description?: string;
  children: ReactNode;
  actions?: ReactNode;
}

export function SettingsSection({
  id,
  title,
  description,
  children,
  actions,
}: Readonly<SettingsSectionProps>) {
  return (
    <section id={id} className="rounded-lg border bg-card p-5 space-y-4">
      <div className="flex flex-col gap-3 md:flex-row md:items-start md:justify-between">
        <div className="space-y-1">
          <h2 className="text-base font-semibold">{title}</h2>
          {description ? (
            <p className="text-sm text-muted-foreground">{description}</p>
          ) : null}
        </div>
        {actions ? <div className="flex shrink-0 gap-2">{actions}</div> : null}
      </div>
      <div className="space-y-4">{children}</div>
    </section>
  );
}
