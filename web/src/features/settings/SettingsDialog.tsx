import { useCallback, useEffect, useRef, useState, type ChangeEvent, type FormEvent } from "react"
import {
  Archive, BellRing, ChevronDown, Download, Globe2, KeyRound, LockKeyhole,
  MailCheck, Moon, Palette, Pencil, Play, Plus, Save, ShieldCheck, Sun,
  Trash2, Upload, UserRound, X,
} from "lucide-react"

import { api } from "../../app/api"
import type {
  CleanupRule, CleanupRuleInput, MailAccount, MailSettings, MigrationArchive,
  MigrationScope, MigrationSections, NotificationSettings, PublicUser, SessionResponse,
} from "../../app/types"
import { useDialogBehavior } from "../../app/useDialogBehavior"
import { useI18n } from "../../i18n/I18nProvider"
import type { MessageKey } from "../../i18n/messages"
import { useTheme, type ThemeMode } from "../../theme/ThemeProvider"

const notificationDefaults: NotificationSettings = {
  enabled: false,
  messageTemplate: "[{account}] {sender}: {subject}",
  commandTemplate: "",
  httpUrl: "",
}

const defaultSections: MigrationSections = {
  profile: true,
  mailAccounts: true,
  notifications: true,
  cleanup: true,
}

interface RuleDraft {
  id?: string
  accountId: string
  name: string
  senderContains: string
  subjectContains: string
  bodyContains: string
  olderThanDays: string
  deleteFromServer: boolean
  enabled: boolean
}

const emptyRule: RuleDraft = {
  accountId: "",
  name: "",
  senderContains: "",
  subjectContains: "",
  bodyContains: "",
  olderThanDays: "",
  deleteFromServer: false,
  enabled: true,
}

type Notice = { key: MessageKey; values?: Record<string, string | number>; error?: boolean }
type BusyAction = "profile" | "avatar" | "pin" | "lock" | "retention" | "rule" | "notification" | "test" | "export" | "import"

export function SettingsDialog({ session, accounts, onSessionChanged, onLocked, onClose, onOpenAccounts }: {
  session: SessionResponse
  accounts: MailAccount[]
  onSessionChanged: (session: SessionResponse) => void
  onLocked: (session: SessionResponse) => void
  onClose: () => void
  onOpenAccounts: () => void
}) {
  const { locale, setLocale, t } = useI18n()
  const { mode, setMode } = useTheme()
  const [user, setUser] = useState(session.user)
  const [nickname, setNickname] = useState(session.user.nickname)
  const [pin, setPin] = useState("")
  const [notifications, setNotifications] = useState<NotificationSettings>(notificationDefaults)
  const [mailSettings, setMailSettings] = useState<MailSettings>({ keepLocalAfterServerDelete: true })
  const [rules, setRules] = useState<CleanupRule[]>([])
  const [ruleDraft, setRuleDraft] = useState<RuleDraft | null>(null)
  const [migrationScope, setMigrationScope] = useState<MigrationScope>("mine")
  const [exportSections, setExportSections] = useState(defaultSections)
  const [importSections, setImportSections] = useState(defaultSections)
  const [passphrase, setPassphrase] = useState("")
  const [archive, setArchive] = useState<MigrationArchive | null>(null)
  const [archiveName, setArchiveName] = useState("")
  const [busy, setBusy] = useState<BusyAction | null>(null)
  const [notice, setNotice] = useState<Notice | null>(null)
  const dialogRef = useRef<HTMLElement>(null)
  const avatarInputRef = useRef<HTMLInputElement>(null)

  useDialogBehavior(dialogRef, onClose)

  const loadSettings = useCallback(async () => {
    const [nextNotifications, nextMailSettings, nextRules] = await Promise.all([
      api.notificationSettings(),
      api.mailSettings(),
      api.cleanupRules(),
    ])
    setNotifications(nextNotifications)
    setMailSettings(nextMailSettings)
    setRules(nextRules)
  }, [])

  useEffect(() => {
    loadSettings().catch(() => setNotice({ key: "genericError", error: true }))
  }, [loadSettings])

  function publishUser(next: PublicUser) {
    setUser(next)
    setNickname(next.nickname)
    onSessionChanged({ ...session, user: next })
  }

  async function saveProfile(event: FormEvent) {
    event.preventDefault()
    setBusy("profile")
    setNotice(null)
    try {
      publishUser(await api.updateProfile(nickname))
      setNotice({ key: "profileSaved" })
    } catch {
      setNotice({ key: "genericError", error: true })
    } finally {
      setBusy(null)
    }
  }

  async function updateAvatar(event: ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0]
    event.target.value = ""
    if (!file) return
    if (file.size > 512 * 1024 || !new Set(["image/png", "image/jpeg", "image/webp"]).has(file.type)) {
      setNotice({ key: "avatarInvalid", error: true })
      return
    }
    setBusy("avatar")
    setNotice(null)
    try {
      publishUser(await api.updateAvatar(file))
      setNotice({ key: "avatarSaved" })
    } catch {
      setNotice({ key: "avatarInvalid", error: true })
    } finally {
      setBusy(null)
    }
  }

  async function removeAvatar() {
    setBusy("avatar")
    try {
      publishUser(await api.removeAvatar())
      setNotice({ key: "avatarRemoved" })
    } catch {
      setNotice({ key: "genericError", error: true })
    } finally {
      setBusy(null)
    }
  }

  async function savePin(event: FormEvent) {
    event.preventDefault()
    if (!pin) return
    setBusy("pin")
    setNotice(null)
    try {
      publishUser(await api.setPin(pin))
      setPin("")
      setNotice({ key: "pinSaved" })
    } catch {
      setNotice({ key: "pinInvalid", error: true })
    } finally {
      setBusy(null)
    }
  }

  async function removePin() {
    setBusy("pin")
    try {
      publishUser(await api.removePin())
      setPin("")
      setNotice({ key: "pinRemoved" })
    } catch {
      setNotice({ key: "genericError", error: true })
    } finally {
      setBusy(null)
    }
  }

  async function lockNow() {
    setBusy("lock")
    try {
      const next = await api.lock()
      onClose()
      onLocked(next)
    } catch {
      setNotice({ key: "genericError", error: true })
      setBusy(null)
    }
  }

  async function toggleRetention(keep: boolean) {
    const previous = mailSettings
    setMailSettings({ keepLocalAfterServerDelete: keep })
    setBusy("retention")
    try {
      setMailSettings(await api.updateMailSettings({ keepLocalAfterServerDelete: keep }))
      setNotice({ key: "retentionSaved" })
    } catch {
      setMailSettings(previous)
      setNotice({ key: "genericError", error: true })
    } finally {
      setBusy(null)
    }
  }

  function editRule(rule?: CleanupRule) {
    setRuleDraft(rule ? {
      id: rule.id,
      accountId: rule.accountId || "",
      name: rule.name,
      senderContains: rule.senderContains || "",
      subjectContains: rule.subjectContains || "",
      bodyContains: rule.bodyContains || "",
      olderThanDays: rule.olderThanDays?.toString() || "",
      deleteFromServer: rule.deleteFromServer,
      enabled: rule.enabled,
    } : { ...emptyRule })
  }

  async function saveRule(event: FormEvent) {
    event.preventDefault()
    if (!ruleDraft) return
    const input: CleanupRuleInput = {
      accountId: ruleDraft.accountId || null,
      name: ruleDraft.name,
      senderContains: ruleDraft.senderContains || null,
      subjectContains: ruleDraft.subjectContains || null,
      bodyContains: ruleDraft.bodyContains || null,
      olderThanDays: ruleDraft.olderThanDays ? Number(ruleDraft.olderThanDays) : null,
      deleteFromServer: ruleDraft.deleteFromServer,
      enabled: ruleDraft.enabled,
    }
    setBusy("rule")
    setNotice(null)
    try {
      if (ruleDraft.id) await api.updateCleanupRule(ruleDraft.id, input)
      else await api.createCleanupRule(input)
      setRules(await api.cleanupRules())
      setRuleDraft(null)
      setNotice({ key: "cleanupRuleSaved" })
    } catch {
      setNotice({ key: "cleanupRuleInvalid", error: true })
    } finally {
      setBusy(null)
    }
  }

  async function deleteRule(id: string) {
    setBusy("rule")
    try {
      await api.deleteCleanupRule(id)
      setRules((current) => current.filter((rule) => rule.id !== id))
      if (ruleDraft?.id === id) setRuleDraft(null)
      setNotice({ key: "cleanupRuleDeleted" })
    } catch {
      setNotice({ key: "genericError", error: true })
    } finally {
      setBusy(null)
    }
  }

  async function saveNotifications(event: FormEvent) {
    event.preventDefault()
    setBusy("notification")
    setNotice(null)
    try {
      setNotifications(await api.updateNotificationSettings(notifications))
      setNotice({ key: "savedSuccess" })
    } catch {
      setNotice({ key: "genericError", error: true })
    } finally {
      setBusy(null)
    }
  }

  async function testNotification() {
    setBusy("test")
    setNotice(null)
    try {
      await api.testNotificationSettings(notifications)
      setNotice({ key: "notificationTestOk" })
    } catch {
      setNotice({ key: "genericError", error: true })
    } finally {
      setBusy(null)
    }
  }

  async function exportConfiguration() {
    if (!passphrase) return
    setBusy("export")
    setNotice(null)
    try {
      const exported = await api.exportConfiguration(passphrase, migrationScope, exportSections)
      const blob = new Blob([JSON.stringify(exported, null, 2)], { type: "application/json" })
      const link = document.createElement("a")
      link.href = URL.createObjectURL(blob)
      link.download = `meowmail-config-${migrationScope}-${new Date().toISOString().slice(0, 10)}.json`
      link.click()
      URL.revokeObjectURL(link.href)
      setNotice({ key: "exportReady" })
    } catch {
      setNotice({ key: "genericError", error: true })
    } finally {
      setBusy(null)
    }
  }

  async function chooseArchive(event: ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0]
    event.target.value = ""
    if (!file) return
    if (file.size > 16 * 1024 * 1024) {
      setNotice({ key: "archiveInvalid", error: true })
      return
    }
    try {
      const parsed = JSON.parse(await file.text()) as MigrationArchive
      if (parsed.format !== "meowmail-migration"
        || parsed.version !== 1
        || !["mine", "allUsers"].includes(parsed.scope)
        || !parsed.sections
        || typeof parsed.encryptedData !== "string") {
        throw new Error("invalid archive")
      }
      setArchive(parsed)
      setArchiveName(file.name)
      setImportSections({ ...parsed.sections })
      setNotice(null)
    } catch {
      setArchive(null)
      setArchiveName("")
      setNotice({ key: "archiveInvalid", error: true })
    }
  }

  async function importConfiguration() {
    if (!passphrase || !archive) return
    if (archive.scope === "allUsers" && user.role !== "admin") {
      setNotice({ key: "adminImportRequired", error: true })
      return
    }
    setBusy("import")
    setNotice(null)
    try {
      const report = await api.importConfiguration(passphrase, importSections, archive)
      const nextSession = await api.session()
      setUser(nextSession.user)
      setNickname(nextSession.user.nickname)
      onSessionChanged(nextSession)
      await loadSettings()
      setNotice({
        key: "importComplete",
        values: {
          users: report.usersImported,
          accounts: report.accountsImported,
          rules: report.rulesImported,
          conflicts: report.conflicts.length,
        },
      })
    } catch {
      setNotice({ key: "archiveImportFailed", error: true })
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

        <div className="settings-content">
          <section className="settings-section profile-section">
            <div className="settings-section-heading"><UserRound size={18} /><div><h3>{t("profile")}</h3><p>{t("profileDescription")}</p></div></div>
            <div className="settings-card profile-card">
              <div className="profile-avatar">
                {user.hasAvatar
                  ? <img src={`/api/v1/users/me/avatar?v=${user.updatedAt}`} alt="" />
                  : user.nickname.slice(0, 1).toUpperCase()}
              </div>
              <div className="profile-summary"><strong>{user.nickname}</strong><span>@{user.username}</span><small>{user.role === "admin" ? t("administrator") : t("standardUser")}</small></div>
              <div className="profile-actions">
                <input ref={avatarInputRef} className="visually-hidden" type="file" accept="image/png,image/jpeg,image/webp" onChange={updateAvatar} />
                <button className="quiet-button" type="button" disabled={busy === "avatar"} onClick={() => avatarInputRef.current?.click()}><Upload size={14} />{t("changeAvatar")}</button>
                {user.hasAvatar && <button className="quiet-button danger-text" type="button" disabled={busy === "avatar"} onClick={removeAvatar}>{t("remove")}</button>}
              </div>
              <form className="profile-name-form" onSubmit={saveProfile}>
                <label className="form-field"><span>{t("nickname")}</span><input value={nickname} onChange={(event) => setNickname(event.target.value)} placeholder={t("nicknamePlaceholder")} /></label>
                <button className="secondary-button" type="submit" disabled={!nickname.trim() || busy === "profile"}>{busy === "profile" ? <span className="spinner spinner-small" /> : <Save size={14} />}{t("save")}</button>
              </form>
            </div>
          </section>

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
            <div className="settings-section-heading"><ShieldCheck size={18} /><div><h3>{t("securityAndLock")}</h3><p>{t("pinLockDescription")}</p></div></div>
            <div className="settings-card security-card">
              <form className="security-pin-row" onSubmit={savePin}>
                <label className="form-field"><span>{user.hasPin ? t("changePin") : t("setPin")}</span><input type="password" inputMode="numeric" autoComplete="new-password" value={pin} onChange={(event) => setPin(event.target.value)} placeholder={t("personalPinPlaceholder")} /></label>
                <button className="secondary-button" type="submit" disabled={!pin || busy === "pin"}><KeyRound size={14} />{t("save")}</button>
              </form>
              {user.hasPin && (
                <div className="security-actions">
                  <button className="secondary-button" type="button" disabled={busy === "lock"} onClick={lockNow}><LockKeyhole size={15} />{t("lockNow")}</button>
                  <button className="quiet-button danger-text" type="button" disabled={busy === "pin"} onClick={removePin}>{t("removePin")}</button>
                </div>
              )}
            </div>
          </section>

          <section className="settings-section">
            <div className="settings-section-heading"><MailCheck size={18} /><div><h3>{t("mailRetention")}</h3><p>{t("mailRetentionDescription")}</p></div></div>
            <div className="settings-card mail-policy-card">
              <label className="toggle-row compact-toggle">
                <span><strong>{t("keepLocalCopies")}</strong><small>{t("keepLocalCopiesDescription")}</small></span>
                <input type="checkbox" checked={mailSettings.keepLocalAfterServerDelete} disabled={busy === "retention"} onChange={(event) => void toggleRetention(event.target.checked)} />
                <span className="toggle" aria-hidden="true" />
              </label>
              <div className="cleanup-heading"><div><strong>{t("automaticCleanup")}</strong><small>{t("automaticCleanupDescription")}</small></div><button className="quiet-button" type="button" onClick={() => editRule()}><Plus size={14} />{t("addRule")}</button></div>
              <div className="cleanup-list">
                {!rules.length && <p className="empty-inline">{t("noCleanupRules")}</p>}
                {rules.map((rule) => (
                  <div className="cleanup-rule-row" key={rule.id}>
                    <span className={`status-dot ${rule.enabled ? "enabled" : ""}`} aria-hidden="true" />
                    <div><strong>{rule.name}</strong><small>{cleanupSummary(rule, accounts, t)}</small></div>
                    {rule.deleteFromServer && <span className="danger-chip">{t("serverDelete")}</span>}
                    <button className="icon-button small" type="button" onClick={() => editRule(rule)} aria-label={t("edit")}><Pencil size={14} /></button>
                    <button className="icon-button small danger-text" type="button" onClick={() => void deleteRule(rule.id)} aria-label={t("delete")}><Trash2 size={14} /></button>
                  </div>
                ))}
              </div>
              {ruleDraft && (
                <form className="cleanup-editor" onSubmit={saveRule}>
                  <div className="cleanup-editor-heading"><strong>{ruleDraft.id ? t("editRule") : t("newRule")}</strong><button className="icon-button small" type="button" onClick={() => setRuleDraft(null)} aria-label={t("close")}><X size={14} /></button></div>
                  <div className="form-grid two-columns">
                    <label className="form-field"><span>{t("ruleName")}</span><input autoFocus value={ruleDraft.name} onChange={(event) => setRuleDraft({ ...ruleDraft, name: event.target.value })} placeholder={t("ruleNamePlaceholder")} /></label>
                    <label className="form-field"><span>{t("mailAccountScope")}</span><select value={ruleDraft.accountId} onChange={(event) => setRuleDraft({ ...ruleDraft, accountId: event.target.value })}><option value="">{t("allAccounts")}</option>{accounts.map((account) => <option key={account.id} value={account.id}>{account.displayName}</option>)}</select></label>
                    <label className="form-field"><span>{t("senderContains")}</span><input value={ruleDraft.senderContains} onChange={(event) => setRuleDraft({ ...ruleDraft, senderContains: event.target.value })} placeholder={t("senderContainsPlaceholder")} /></label>
                    <label className="form-field"><span>{t("subjectContains")}</span><input value={ruleDraft.subjectContains} onChange={(event) => setRuleDraft({ ...ruleDraft, subjectContains: event.target.value })} placeholder={t("subjectContainsPlaceholder")} /></label>
                    <label className="form-field"><span>{t("bodyContains")}</span><input value={ruleDraft.bodyContains} onChange={(event) => setRuleDraft({ ...ruleDraft, bodyContains: event.target.value })} placeholder={t("bodyContainsPlaceholder")} /></label>
                    <label className="form-field"><span>{t("olderThanDays")}</span><input type="number" min="1" max="36500" value={ruleDraft.olderThanDays} onChange={(event) => setRuleDraft({ ...ruleDraft, olderThanDays: event.target.value })} placeholder={t("olderThanDaysPlaceholder")} /></label>
                  </div>
                  <label className="check-row inline-check"><input type="checkbox" checked={ruleDraft.enabled} onChange={(event) => setRuleDraft({ ...ruleDraft, enabled: event.target.checked })} /><span className="custom-check">✓</span>{t("enableRule")}</label>
                  <label className="check-row inline-check danger-check"><input type="checkbox" checked={ruleDraft.deleteFromServer} onChange={(event) => setRuleDraft({ ...ruleDraft, deleteFromServer: event.target.checked })} /><span className="custom-check">✓</span><span>{t("deleteFromServer")}<small>{t("deleteFromServerWarning")}</small></span></label>
                  <div className="editor-actions"><button className="quiet-button" type="button" onClick={() => setRuleDraft(null)}>{t("cancel")}</button><button className="primary-button" type="submit" disabled={busy === "rule"}>{busy === "rule" ? <span className="spinner spinner-small" /> : <Save size={14} />}{t("saveRule")}</button></div>
                </form>
              )}
            </div>
          </section>

          <details className="settings-section disclosure-section">
            <summary><span><BellRing size={18} /></span><div><h3>{t("notifications")}</h3><p>{t("notificationsDescription")}</p></div><ChevronDown size={16} /></summary>
            <form className="settings-card notification-settings" onSubmit={saveNotifications}>
              <label className="toggle-row">
                <span><strong>{t("enableNotifications")}</strong><small>{t("notificationsDescription")}</small></span>
                <input type="checkbox" checked={notifications.enabled} onChange={(event) => setNotifications({ ...notifications, enabled: event.target.checked })} />
                <span className="toggle" aria-hidden="true" />
              </label>
              <label className="form-field wide"><span>{t("messageTemplate")}</span><input value={notifications.messageTemplate} onChange={(event) => setNotifications({ ...notifications, messageTemplate: event.target.value })} placeholder={t("messageTemplatePlaceholder")} /></label>
              <label className="form-field wide"><span>{t("commandTemplate")}</span><input className="mono-input" value={notifications.commandTemplate || ""} onChange={(event) => setNotifications({ ...notifications, commandTemplate: event.target.value })} placeholder={t("commandPlaceholder")} /><small>{t("commandHint")}</small></label>
              <label className="form-field wide"><span>{t("webhookUrl")}</span><input type="url" value={notifications.httpUrl || ""} onChange={(event) => setNotifications({ ...notifications, httpUrl: event.target.value })} placeholder={t("webhookPlaceholder")} /></label>
              <div className="placeholder-box"><span>{t("placeholderReference")}</span><div>{["account", "email", "sender", "sender_email", "subject", "preview", "message"].map((name) => <code key={name}>{`{${name}}`}</code>)}</div></div>
              <div className="card-actions"><button className="secondary-button" type="button" disabled={Boolean(busy)} onClick={testNotification}>{busy === "test" ? <span className="spinner spinner-small" /> : <Play size={15} />}{t("testNotification")}</button><button className="primary-button" type="submit" disabled={Boolean(busy)}>{busy === "notification" ? <span className="spinner spinner-small" /> : <Save size={15} />}{t("save")}</button></div>
            </form>
          </details>

          <section className="settings-section">
            <div className="settings-section-heading"><Archive size={18} /><div><h3>{t("configurationTransfer")}</h3><p>{t("configurationTransferDescription")}</p></div></div>
            <div className="settings-card migration-card">
              <div className="migration-scope-row">
                <div><strong>{t("exportScope")}</strong><small>{migrationScope === "allUsers" ? t("allUsersExportDescription") : t("mineExportDescription")}</small></div>
                {user.role === "admin" ? (
                  <div className="segmented-control compact">
                    <button type="button" className={migrationScope === "mine" ? "active" : ""} aria-pressed={migrationScope === "mine"} onClick={() => setMigrationScope("mine")}>{t("onlyMyConfiguration")}</button>
                    <button type="button" className={migrationScope === "allUsers" ? "active" : ""} aria-pressed={migrationScope === "allUsers"} onClick={() => setMigrationScope("allUsers")}>{t("allUsers")}</button>
                  </div>
                ) : <span className="scope-chip">{t("onlyMyConfiguration")}</span>}
              </div>
              {migrationScope === "allUsers" && <div className="sensitive-note"><ShieldCheck size={15} /><span>{t("allUsersSensitiveNote")}</span></div>}
              <SectionPicker value={exportSections} onChange={setExportSections} t={t} />
              <label className="form-field migration-passphrase"><span>{t("archivePassphrase")}</span><input type="password" autoComplete="new-password" value={passphrase} onChange={(event) => setPassphrase(event.target.value)} placeholder={t("archivePassphrasePlaceholder")} /><small>{t("archivePassphraseHint")}</small></label>
              <div className="migration-actions">
                <button className="secondary-button" type="button" disabled={!passphrase || !hasSection(exportSections) || busy === "export"} onClick={exportConfiguration}>{busy === "export" ? <span className="spinner spinner-small" /> : <Download size={15} />}{t("exportConfiguration")}</button>
                <label className="secondary-button file-button"><Upload size={15} />{t("chooseArchive")}<input type="file" accept="application/json,.json" onChange={chooseArchive} /></label>
              </div>
              {archive && (
                <div className="import-panel">
                  <div className="archive-summary"><div><strong>{archiveName}</strong><small>{archive.scope === "allUsers" ? t("allUsersArchive") : t("personalArchive")}</small></div><button className="icon-button small" type="button" onClick={() => { setArchive(null); setArchiveName("") }} aria-label={t("remove")}><X size={14} /></button></div>
                  <SectionPicker value={importSections} available={archive.sections} onChange={setImportSections} t={t} />
                  <button className="primary-button import-button" type="button" disabled={!passphrase || !hasSection(importSections) || busy === "import"} onClick={importConfiguration}>{busy === "import" ? <span className="spinner spinner-small" /> : <Upload size={15} />}{t("importSelectedConfiguration")}</button>
                </div>
              )}
            </div>
          </section>

          {notice && <div className={`inline-notice settings-notice ${notice.error ? "error" : ""}`} aria-live="polite">{t(notice.key, notice.values)}</div>}
        </div>
        <footer className="modal-footer settings-footer-simple"><span>{t("settingsSavedImmediately")}</span><button className="secondary-button" type="button" onClick={onClose}>{t("done")}</button></footer>
      </section>
    </div>
  )
}

function SectionPicker({ value, available, onChange, t }: {
  value: MigrationSections
  available?: MigrationSections
  onChange: (value: MigrationSections) => void
  t: (key: MessageKey, values?: Record<string, string | number>) => string
}) {
  const sections: Array<[keyof MigrationSections, MessageKey, MessageKey]> = [
    ["profile", "profileAndAvatar", "profileAndAvatarDescription"],
    ["mailAccounts", "mailAccountsAndCredentials", "mailAccountsAndCredentialsDescription"],
    ["notifications", "notificationConfiguration", "notificationConfigurationDescription"],
    ["cleanup", "retentionAndCleanupRules", "retentionAndCleanupRulesDescription"],
  ]
  return (
    <div className="migration-sections">
      {sections.map(([key, label, description]) => {
        const enabled = available?.[key] ?? true
        return (
          <label className={`migration-section-option ${enabled ? "" : "unavailable"}`} key={key}>
            <input type="checkbox" checked={value[key] && enabled} disabled={!enabled} onChange={(event) => onChange({ ...value, [key]: event.target.checked })} />
            <span className="custom-check">✓</span>
            <span><strong>{t(label)}</strong><small>{t(description)}</small></span>
          </label>
        )
      })}
    </div>
  )
}

function hasSection(sections: MigrationSections) {
  return sections.profile || sections.mailAccounts || sections.notifications || sections.cleanup
}

function cleanupSummary(
  rule: CleanupRule,
  accounts: MailAccount[],
  t: (key: MessageKey, values?: Record<string, string | number>) => string,
) {
  const account = rule.accountId
    ? accounts.find((item) => item.id === rule.accountId)?.displayName || t("oneAccount")
    : t("allAccounts")
  const conditions = [
    rule.senderContains && t("senderCondition", { value: rule.senderContains }),
    rule.subjectContains && t("subjectCondition", { value: rule.subjectContains }),
    rule.bodyContains && t("bodyCondition", { value: rule.bodyContains }),
    rule.olderThanDays && t("ageCondition", { days: rule.olderThanDays }),
  ].filter(Boolean)
  return `${account} · ${conditions.join(" · ")}`
}
