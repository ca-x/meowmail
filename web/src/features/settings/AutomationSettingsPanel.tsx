import type { MailAccount } from "../../app/types"
import { MailAutomationSettings } from "./MailAutomationSettings"
import { NotificationSettingsSection } from "./NotificationSettingsSection"
import type { SettingsNotice } from "./settingsTypes"

export function AutomationSettingsPanel({ accounts, onNotice }: {
  accounts: MailAccount[]
  onNotice: (notice: SettingsNotice) => void
}) {
  return (
    <div className="settings-panel-stack">
      <MailAutomationSettings accounts={accounts} onNotice={onNotice} />
      <div className="settings-subsection-divider" />
      <NotificationSettingsSection onNotice={onNotice} />
    </div>
  )
}
