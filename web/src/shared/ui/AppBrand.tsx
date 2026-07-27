import type { ReactNode } from "react"

import { useI18n } from "../../i18n/I18nProvider"

export function AppBrand({ variant = "compact", eyebrow }: {
  variant?: "compact" | "hero"
  eyebrow?: ReactNode
}) {
  const { t } = useI18n()
  const Name = variant === "hero" ? "h1" : "strong"

  return (
    <div className={`app-brand app-brand-${variant}`}>
      <span className="app-brand-logo" aria-hidden="true">
        <img src="/meowmail-logo.png" alt="" />
      </span>
      <span className="app-brand-copy">
        {eyebrow && <span className="app-brand-eyebrow">{eyebrow}</span>}
        <Name>{t("brandName")}</Name>
      </span>
    </div>
  )
}
