import { MailOpen } from "lucide-react"
import { useState } from "react"

import type { MailAccount, MailPreferences } from "../../app/types"
import { useI18n } from "../../i18n/I18nProvider"
import { MailExperienceSettings } from "./MailExperienceSettings"
import { SettingsPanelHeading } from "./SettingsPanelHeading"
import type { SettingsNotice } from "./settingsTypes"

export function MailSettingsPanel({ accounts, mailPreferences, onMailPreferencesChanged, onAccountsChanged, onNotice }: {
  accounts: MailAccount[]
  mailPreferences: MailPreferences
  onMailPreferencesChanged: (preferences: MailPreferences) => void
  onAccountsChanged: (accounts: MailAccount[]) => void
  onNotice: (notice: SettingsNotice) => void
}) {
  const { t } = useI18n()
  const [preferences, setPreferences] = useState(mailPreferences)

  return (
    <div className="settings-panel-stack">
      <SettingsPanelHeading icon={<MailOpen />} title={t("settingsMail")} description={t("settingsMailDescription")} />
      <MailExperienceSettings
        initialPreferences={preferences}
        accounts={accounts}
        onPreferencesChanged={(next) => {
          setPreferences(next)
          onMailPreferencesChanged(next)
        }}
        onAccountsChanged={onAccountsChanged}
        onNotice={(key, error) => onNotice({ key, error })}
      />
    </div>
  )
}
