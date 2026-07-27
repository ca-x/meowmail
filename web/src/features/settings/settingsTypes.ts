import type { MessageKey } from "../../i18n/messages"

export type SettingsNotice = {
  key: MessageKey
  values?: Record<string, string | number>
  error?: boolean
}

export type SettingsTab = "general" | "mail" | "automation" | "security" | "data"

