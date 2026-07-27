import { Button } from "@astryxdesign/core/Button"
import { Save } from "lucide-react"
import { useEffect, useState } from "react"

import { api } from "../../app/api"
import type { MailAccount, MailPreferences } from "../../app/types"
import { useI18n } from "../../i18n/I18nProvider"
import type { MessageKey } from "../../i18n/messages"
import { MailReadingPreferences } from "./MailReadingPreferences"
import { MailReplyPreferences } from "./MailReplyPreferences"
import { MailSendingPreferences } from "./MailSendingPreferences"

export function MailExperienceSettings({
  initialPreferences,
  accounts,
  onPreferencesChanged,
  onAccountsChanged,
  onNotice,
}: {
  initialPreferences: MailPreferences
  accounts: MailAccount[]
  onPreferencesChanged: (preferences: MailPreferences) => void
  onAccountsChanged: (accounts: MailAccount[]) => void
  onNotice: (key: MessageKey, error?: boolean) => void
}) {
  const { t } = useI18n()
  const [preferences, setPreferences] = useState(initialPreferences)
  const [saving, setSaving] = useState(false)

  useEffect(() => setPreferences(initialPreferences), [initialPreferences])

  async function savePreferences() {
    setSaving(true)
    try {
      const saved = await api.updateMailPreferences(preferences)
      setPreferences(saved)
      onPreferencesChanged(saved)
      onNotice("mailPreferencesSaved")
    } catch {
      onNotice("mailPreferencesInvalid", true)
    } finally {
      setSaving(false)
    }
  }

  return (
    <div className="mail-experience-form">
      <MailReadingPreferences preferences={preferences} onChange={setPreferences} />
      <div className="settings-subsection-divider" />
      <MailSendingPreferences
        preferences={preferences}
        onChange={setPreferences}
        accounts={accounts}
        onAccountsChanged={onAccountsChanged}
        onNotice={onNotice}
      />
      <div className="settings-subsection-divider" />
      <MailReplyPreferences preferences={preferences} onChange={setPreferences} />
      <div className="mail-preferences-save-bar">
        <span>{t("mailPreferencesSaveHint")}</span>
        <Button
          label={t("saveMailPreferences")}
          icon={<Save aria-hidden="true" />}
          variant="primary"
          isLoading={saving}
          isDisabled={saving}
          onClick={() => void savePreferences()}
        />
      </div>
    </div>
  )
}
