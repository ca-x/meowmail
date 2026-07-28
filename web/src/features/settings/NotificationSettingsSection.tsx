import { Button } from "@astryxdesign/core/Button"
import { Switch } from "@astryxdesign/core/Switch"
import { TextInput } from "@astryxdesign/core/TextInput"
import { BellRing, Play, Save } from "lucide-react"
import { useEffect, useState, type FormEvent } from "react"

import { api } from "../../app/api"
import type { NotificationSettings } from "../../app/types"
import { useI18n } from "../../i18n/I18nProvider"
import { SettingsPanelHeading } from "./SettingsPanelHeading"
import type { SettingsNotice } from "./settingsTypes"

const defaults: NotificationSettings = {
  enabled: false,
  messageTemplate: "[{account}] {sender}: {subject}",
  commandTemplate: "",
  httpUrl: "",
}

export function NotificationSettingsSection({ onNotice }: { onNotice: (notice: SettingsNotice) => void }) {
  const { t } = useI18n()
  const [settings, setSettings] = useState<NotificationSettings>(defaults)
  const [busy, setBusy] = useState<"save" | "test" | null>(null)

  useEffect(() => {
    api.notificationSettings().then(setSettings).catch(() => onNotice({ key: "genericError", error: true }))
  }, [onNotice])

  async function save(event: FormEvent) {
    event.preventDefault()
    setBusy("save")
    try {
      setSettings(await api.updateNotificationSettings(settings))
      onNotice({ key: "savedSuccess" })
    } catch {
      onNotice({ key: "genericError", error: true })
    } finally {
      setBusy(null)
    }
  }

  async function testNotification() {
    setBusy("test")
    try {
      await api.testNotificationSettings(settings)
      onNotice({ key: "notificationTestOk" })
    } catch {
      onNotice({ key: "genericError", error: true })
    } finally {
      setBusy(null)
    }
  }

  return (
    <>
      <SettingsPanelHeading icon={<BellRing />} title={t("notifications")} description={t("notificationsDescription")} />
      <form className="settings-notification-block" onSubmit={save}>
        <Switch
          label={t("enableNotifications")}
          labelTooltip={t("notificationsDescription")}
          value={settings.enabled}
          onChange={(enabled) => setSettings({ ...settings, enabled })}
          labelPosition="start"
          labelSpacing="spread"
        />
        <div className="settings-form-grid">
          <TextInput label={t("messageTemplate")} value={settings.messageTemplate} onChange={(messageTemplate) => setSettings({ ...settings, messageTemplate })} placeholder={t("messageTemplatePlaceholder")} width="100%" />
          <TextInput className="settings-mono-field" label={t("commandTemplate")} labelTooltip={t("commandHint")} value={settings.commandTemplate || ""} onChange={(commandTemplate) => setSettings({ ...settings, commandTemplate })} placeholder={t("commandPlaceholder")} width="100%" />
          <TextInput label={t("webhookUrl")} value={settings.httpUrl || ""} onChange={(httpUrl) => setSettings({ ...settings, httpUrl })} placeholder={t("webhookPlaceholder")} width="100%" />
        </div>
        <div className="settings-placeholder-reference">
          <span>{t("placeholderReference")}</span>
          <div>{["account", "email", "sender", "sender_email", "subject", "preview", "message"].map((name) => <code key={name}>{`{${name}}`}</code>)}</div>
        </div>
        <div className="settings-button-row end">
          <Button label={t("testNotification")} icon={<Play aria-hidden="true" />} variant="secondary" isLoading={busy === "test"} isDisabled={Boolean(busy)} onClick={() => void testNotification()} />
          <Button label={t("save")} icon={<Save aria-hidden="true" />} variant="primary" type="submit" isLoading={busy === "save"} isDisabled={Boolean(busy)} />
        </div>
      </form>
    </>
  )
}
