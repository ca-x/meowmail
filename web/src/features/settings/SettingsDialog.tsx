import { useEffect, useRef, useState, type FormEvent } from "react"
import { BellRing, Globe2, Moon, Palette, Play, Save, Sun, X } from "lucide-react"

import { api } from "../../app/api"
import { useDialogBehavior } from "../../app/useDialogBehavior"
import type { NotificationSettings } from "../../app/types"
import { useI18n } from "../../i18n/I18nProvider"
import type { MessageKey } from "../../i18n/messages"
import { useTheme, type ThemeMode } from "../../theme/ThemeProvider"

const defaults: NotificationSettings = {
  enabled: false,
  messageTemplate: "[{account}] {sender}: {subject}",
  commandTemplate: "",
  httpUrl: "",
}

export function SettingsDialog({ onClose, onOpenAccounts }: { onClose: () => void; onOpenAccounts: () => void }) {
  const { locale, setLocale, t } = useI18n()
  const { mode, setMode } = useTheme()
  const [settings, setSettings] = useState<NotificationSettings>(defaults)
  const [busy, setBusy] = useState<"save" | "test" | null>(null)
  const [messageKey, setMessageKey] = useState<MessageKey | null>(null)
  const dialogRef = useRef<HTMLElement>(null)

  useDialogBehavior(dialogRef, onClose)

  useEffect(() => {
    api.notificationSettings().then(setSettings).catch(() => setMessageKey("genericError"))
  }, [])

  async function save(event: FormEvent) {
    event.preventDefault()
    setBusy("save")
    setMessageKey(null)
    try {
      const saved = await api.updateNotificationSettings(settings)
      setSettings(saved)
      setMessageKey("savedSuccess")
    } catch {
      setMessageKey("genericError")
    } finally {
      setBusy(null)
    }
  }

  async function testNotification() {
    setBusy("test")
    setMessageKey(null)
    try {
      await api.testNotificationSettings(settings)
      setMessageKey("notificationTestOk")
    } catch {
      setMessageKey("genericError")
    } finally {
      setBusy(null)
    }
  }

  return (
    <div className="modal-backdrop" role="presentation" onMouseDown={(event) => event.target === event.currentTarget && onClose()}>
      <section ref={dialogRef} className="modal-card settings-dialog" role="dialog" aria-modal="true" aria-labelledby="settings-title" tabIndex={-1}>
        <header className="modal-header">
          <div className="modal-title-group">
            <span className="modal-icon"><Palette size={20} /></span>
            <div><p>{t("brandName")}</p><h2 id="settings-title">{t("settings")}</h2></div>
          </div>
          <button className="icon-button" type="button" onClick={onClose} aria-label={t("close")}><X size={18} /></button>
        </header>
        <form className="settings-content" onSubmit={save}>
          <section className="settings-section">
            <div className="settings-section-heading"><Palette size={18} /><div><h3>{t("appearance")}</h3><p>{t("language")} · {t("theme")}</p></div></div>
            <div className="settings-card">
              <div className="setting-row">
                <div className="setting-label"><Globe2 size={17} /><span>{t("language")}</span></div>
                <div className="segmented-control compact">
                  <button type="button" className={locale === "zh-CN" ? "active" : ""} aria-pressed={locale === "zh-CN"} data-dialog-initial-focus={locale === "zh-CN" || undefined} onClick={() => setLocale("zh-CN")}>中文</button>
                  <button type="button" className={locale === "en" ? "active" : ""} aria-pressed={locale === "en"} data-dialog-initial-focus={locale === "en" || undefined} onClick={() => setLocale("en")}>English</button>
                </div>
              </div>
              <div className="setting-row">
                <div className="setting-label"><Moon size={17} /><span>{t("theme")}</span></div>
                <div className="segmented-control compact theme-control">
                  {(["system", "light", "dark"] as ThemeMode[]).map((value) => (
                    <button key={value} type="button" className={mode === value ? "active" : ""} aria-pressed={mode === value} onClick={() => setMode(value)}>
                      {value === "light" && <Sun size={14} />}
                      {value === "dark" && <Moon size={14} />}
                      {t(value === "system" ? "themeSystem" : value === "light" ? "themeLight" : "themeDark")}
                    </button>
                  ))}
                </div>
              </div>
              <button className="setting-link" type="button" onClick={onOpenAccounts}>{t("accounts")}<span>→</span></button>
            </div>
          </section>

          <section className="settings-section">
            <div className="settings-section-heading"><BellRing size={18} /><div><h3>{t("notifications")}</h3><p>{t("notificationsDescription")}</p></div></div>
            <div className="settings-card notification-settings">
              <label className="toggle-row">
                <span><strong>{t("enableNotifications")}</strong><small>{t("notificationsDescription")}</small></span>
                <input type="checkbox" checked={settings.enabled} onChange={(event) => setSettings({ ...settings, enabled: event.target.checked })} />
                <span className="toggle" aria-hidden="true" />
              </label>
              <label className="form-field wide">
                <span>{t("messageTemplate")}</span>
                <input value={settings.messageTemplate} onChange={(event) => setSettings({ ...settings, messageTemplate: event.target.value })} />
              </label>
              <label className="form-field wide">
                <span>{t("commandTemplate")}</span>
                <input
                  className="mono-input"
                  value={settings.commandTemplate || ""}
                  onChange={(event) => setSettings({ ...settings, commandTemplate: event.target.value })}
                  placeholder={t("commandPlaceholder")}
                />
                <small>{t("commandHint")}</small>
              </label>
              <label className="form-field wide">
                <span>{t("webhookUrl")}</span>
                <input type="url" value={settings.httpUrl || ""} onChange={(event) => setSettings({ ...settings, httpUrl: event.target.value })} placeholder={t("webhookPlaceholder")} />
              </label>
              <div className="placeholder-box">
                <span>{t("placeholderReference")}</span>
                <div>{["account", "email", "sender", "sender_email", "subject", "preview", "message"].map((name) => <code key={name}>{`{${name}}`}</code>)}</div>
              </div>
            </div>
          </section>

          {messageKey && <div className="inline-notice" aria-live="polite">{t(messageKey)}</div>}
          <footer className="modal-footer settings-footer">
            <button className="secondary-button" type="button" disabled={Boolean(busy)} onClick={testNotification}>
              {busy === "test" ? <span className="spinner spinner-small" /> : <Play size={15} />}
              {busy === "test" ? t("testing") : t("testNotification")}
            </button>
            <button className="primary-button" type="submit" disabled={Boolean(busy)}>
              {busy === "save" ? <span className="spinner spinner-small" /> : <Save size={15} />}
              {busy === "save" ? t("saving") : t("save")}
            </button>
          </footer>
        </form>
      </section>
    </div>
  )
}
