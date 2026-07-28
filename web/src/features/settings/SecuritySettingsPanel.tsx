import { Badge } from "@astryxdesign/core/Badge"
import { Banner } from "@astryxdesign/core/Banner"
import { Button } from "@astryxdesign/core/Button"
import { Switch } from "@astryxdesign/core/Switch"
import { TextInput } from "@astryxdesign/core/TextInput"
import { Bot, Copy, KeyRound, LockKeyhole, RotateCw, ShieldCheck } from "lucide-react"
import { useEffect, useState, type FormEvent } from "react"

import { api } from "../../app/api"
import type { McpSettings, PublicUser, SessionResponse } from "../../app/types"
import { useI18n } from "../../i18n/I18nProvider"
import { useImperativeConfirmDialog } from "../../shared/ui/ImperativeConfirmDialog"
import { SettingsPanelHeading } from "./SettingsPanelHeading"
import type { SettingsNotice } from "./settingsTypes"

const defaultMcpSettings: McpSettings = {
  hasToken: false,
  allowDelete: false,
  endpoint: "/mcp",
}

export function SecuritySettingsPanel({ isOpen, session, onSessionChanged, onLocked, onLoggedOut, onClose, onNotice }: {
  isOpen: boolean
  session: SessionResponse
  onSessionChanged: (session: SessionResponse) => void
  onLocked: (session: SessionResponse) => void
  onLoggedOut: () => void
  onClose: () => void
  onNotice: (notice: SettingsNotice) => void
}) {
  const { locale, t } = useI18n()
  const confirmDialog = useImperativeConfirmDialog()
  const [user, setUser] = useState(session.user)
  const [pin, setPin] = useState("")
  const [currentPassword, setCurrentPassword] = useState("")
  const [newPassword, setNewPassword] = useState("")
  const [confirmPassword, setConfirmPassword] = useState("")
  const [mcpSettings, setMcpSettings] = useState<McpSettings>(defaultMcpSettings)
  const [mcpToken, setMcpToken] = useState<string | null>(null)
  const [busy, setBusy] = useState<"password" | "pin" | "lock" | "ai" | "mcp" | null>(null)
  const [mcpLoading, setMcpLoading] = useState(true)

  useEffect(() => {
    api.mcpSettings()
      .then(setMcpSettings)
      .catch(() => onNotice({ key: "genericError", error: true }))
      .finally(() => setMcpLoading(false))
  }, [onNotice])

  useEffect(() => {
    if (isOpen) return
    setCurrentPassword("")
    setNewPassword("")
    setConfirmPassword("")
  }, [isOpen])

  function publishUser(next: PublicUser) {
    setUser(next)
    onSessionChanged({ ...session, user: next })
  }

  async function savePin(event: FormEvent) {
    event.preventDefault()
    if (!pin) return
    setBusy("pin")
    try {
      publishUser(await api.setPin(pin))
      setPin("")
      onNotice({ key: "pinSaved" })
    } catch {
      onNotice({ key: "pinInvalid", error: true })
    } finally {
      setBusy(null)
    }
  }

  async function savePassword(event: FormEvent) {
    event.preventDefault()
    if (newPassword !== confirmPassword) {
      onNotice({ key: "passwordMismatch", error: true })
      return
    }
    setBusy("password")
    try {
      await api.updatePassword(user.hasPassword ? currentPassword : null, newPassword)
      setCurrentPassword("")
      setNewPassword("")
      setConfirmPassword("")
      onClose()
      onLoggedOut()
    } catch {
      onNotice({ key: "passwordInvalid", error: true })
    } finally {
      setBusy(null)
    }
  }

  async function removePin() {
    setBusy("pin")
    try {
      publishUser(await api.removePin())
      setPin("")
      onNotice({ key: "pinRemoved" })
    } catch {
      onNotice({ key: "genericError", error: true })
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
      onNotice({ key: "genericError", error: true })
      setBusy(null)
    }
  }

  async function toggleAiAccess(enabled: boolean) {
    const previous = user
    setUser({ ...user, aiEnabled: enabled })
    setBusy("ai")
    try {
      publishUser(await api.updateAiAccess(enabled))
      onNotice({ key: enabled ? "aiAccessEnabled" : "aiAccessDisabled" })
    } catch {
      setUser(previous)
      onNotice({ key: "genericError", error: true })
    } finally {
      setBusy(null)
    }
  }

  async function generateMcpToken() {
    if (mcpSettings.hasToken) {
      const confirmed = await confirmDialog.confirm({
        title: t("mcpRegenerate"),
        description: t("mcpRotateConfirm"),
        cancelLabel: t("cancel"),
        actionLabel: t("mcpRegenerate"),
        actionVariant: "destructive",
      })
      if (!confirmed) return
    }
    setBusy("mcp")
    try {
      const generated = await api.generateMcpToken()
      const { token, ...settings } = generated
      setMcpSettings(settings)
      setMcpToken(token)
      onNotice({ key: "mcpTokenGenerated" })
    } catch {
      onNotice({ key: "genericError", error: true })
    } finally {
      setBusy(null)
    }
  }

  async function toggleMcpDelete(allowDelete: boolean) {
    const previous = mcpSettings
    setMcpSettings({ ...mcpSettings, allowDelete })
    setBusy("mcp")
    try {
      setMcpSettings(await api.updateMcpSettings(allowDelete))
      onNotice({ key: allowDelete ? "mcpDeleteEnabled" : "mcpDeleteDisabled" })
    } catch {
      setMcpSettings(previous)
      onNotice({ key: "genericError", error: true })
    } finally {
      setBusy(null)
    }
  }

  async function revokeMcpToken() {
    const confirmed = await confirmDialog.confirm({
      title: t("mcpRevoke"),
      description: t("mcpRevokeConfirm"),
      cancelLabel: t("cancel"),
      actionLabel: t("mcpRevoke"),
      actionVariant: "destructive",
    })
    if (!confirmed) return
    setBusy("mcp")
    try {
      await api.revokeMcpToken()
      setMcpSettings(defaultMcpSettings)
      setMcpToken(null)
      onNotice({ key: "mcpTokenRevoked" })
    } catch {
      onNotice({ key: "genericError", error: true })
    } finally {
      setBusy(null)
    }
  }

  async function copyMcpToken() {
    if (!mcpToken) return
    try {
      await navigator.clipboard.writeText(mcpToken)
      onNotice({ key: "mcpTokenCopied" })
    } catch {
      onNotice({ key: "mcpCopyFailed", error: true })
    }
  }

  return (
    <>
    <div className="settings-panel-stack">
      <SettingsPanelHeading icon={<KeyRound />} title={t("loginPasswordSettings")} description={t("loginPasswordSettingsDescription")} />
      <section className="settings-security-block" aria-label={t("loginPasswordSettings")}>
        <form className="settings-password-form" onSubmit={savePassword}>
          <div className="settings-password-fields">
            {user.hasPassword && (
              <TextInput
                {...{ autoComplete: "current-password" }}
                type="password"
                label={t("currentPassword")}
                value={currentPassword}
                onChange={setCurrentPassword}
                placeholder={t("currentPasswordPlaceholder")}
                width="100%"
              />
            )}
            <TextInput
              {...{ autoComplete: "new-password" }}
              type="password"
              label={t("newPassword")}
              value={newPassword}
              onChange={setNewPassword}
              placeholder={t("newPasswordPlaceholder")}
              width="100%"
            />
            <TextInput
              {...{ autoComplete: "new-password" }}
              type="password"
              label={t("confirmPassword")}
              value={confirmPassword}
              onChange={setConfirmPassword}
              placeholder={t("newPasswordPlaceholder")}
              width="100%"
            />
          </div>
          <Button
            label={user.hasPassword ? t("changeLoginPassword") : t("setLoginPassword")}
            icon={<KeyRound aria-hidden="true" />}
            type="submit"
            variant="secondary"
            isLoading={busy === "password"}
            isDisabled={Boolean(busy) || newPassword.length < 8 || newPassword !== confirmPassword || (user.hasPassword && !currentPassword)}
          />
        </form>
      </section>

      <div className="settings-subsection-divider" />
      <SettingsPanelHeading icon={<ShieldCheck />} title={t("securityAndLock")} description={t("pinLockDescription")} />
      <section className="settings-security-block" aria-label={t("securityAndLock")}>
        <form className="settings-inline-form" onSubmit={savePin}>
          <TextInput
            {...{ autoComplete: "new-password", inputMode: "numeric" }}
            type="password"
            label={user.hasPin ? t("changePin") : t("setPin")}
            value={pin}
            onChange={setPin}
            placeholder={t("personalPinPlaceholder")}
            width="100%"
          />
          <Button label={t("save")} icon={<KeyRound aria-hidden="true" />} type="submit" variant="secondary" isLoading={busy === "pin"} isDisabled={!pin || busy === "pin"} />
        </form>
        {user.hasPin && (
          <div className="settings-button-row">
            <Button label={t("lockNow")} icon={<LockKeyhole aria-hidden="true" />} variant="secondary" isLoading={busy === "lock"} isDisabled={Boolean(busy)} onClick={() => void lockNow()} />
            <Button label={t("removePin")} variant="ghost" isDisabled={Boolean(busy)} onClick={() => void removePin()} />
          </div>
        )}
      </section>

      <div className="settings-subsection-divider" />
      <SettingsPanelHeading icon={<Bot />} title={t("aiAccess")} description={t("aiAccessDescription")} />
      <section className="settings-security-block" aria-label={t("aiAccess")}>
        <Switch
          label={t("enableAiFeatures")}
          labelTooltip={t("enableAiFeaturesDescription")}
          value={user.aiEnabled}
          onChange={(enabled) => void toggleAiAccess(enabled)}
          isLoading={busy === "ai"}
          isDisabled={Boolean(busy)}
          labelSpacing="spread"
          labelPosition="start"
        />
      </section>

      <div className="settings-subsection-divider" />
      <SettingsPanelHeading icon={<Bot />} title={t("mcpAccess")} description={t("mcpAccessDescription")} />
      <section className="settings-mcp-block" aria-label={t("mcpAccess")}>
        <div className="settings-status-row">
          <div><strong>{t("mcpPersonalToken")}</strong><small>{mcpSettings.hasToken ? t("mcpTokenActive") : t("mcpTokenInactive")}</small></div>
          <Badge variant={mcpSettings.hasToken ? "success" : "neutral"} label={mcpSettings.hasToken ? t("enabled") : t("disabled")} />
        </div>
        <div className="settings-code-row"><span>{t("mcpEndpoint")}</span><code>{`${window.location.origin}${mcpSettings.endpoint}`}</code></div>
        {mcpToken && (
          <div className="settings-token-reveal">
            <Banner status="warning" title={t("mcpCopyNow")} description={t("mcpShownOnce")} />
            <div className="settings-token-row">
              <input aria-label={t("mcpPersonalToken")} className="settings-token-input" readOnly value={mcpToken} onFocus={(event) => event.currentTarget.select()} />
              <Button label={t("copy")} icon={<Copy aria-hidden="true" />} variant="secondary" onClick={() => void copyMcpToken()} />
            </div>
          </div>
        )}
        {mcpSettings.hasToken && (
          <Switch
            label={t("mcpAllowDelete")}
            description={t("mcpAllowDeleteDescription")}
            value={mcpSettings.allowDelete}
            onChange={(checked) => void toggleMcpDelete(checked)}
            isLoading={busy === "mcp"}
            isDisabled={busy === "mcp"}
            labelSpacing="spread"
            labelPosition="start"
          />
        )}
        <div className="settings-button-row">
          <Button
            label={mcpSettings.hasToken ? t("mcpRegenerate") : t("mcpGenerate")}
            icon={mcpSettings.hasToken ? <RotateCw aria-hidden="true" /> : <KeyRound aria-hidden="true" />}
            variant="secondary"
            isLoading={mcpLoading || busy === "mcp"}
            isDisabled={mcpLoading || busy === "mcp"}
            onClick={() => void generateMcpToken()}
          />
          {mcpSettings.hasToken && <Button label={t("mcpRevoke")} variant="destructive" isDisabled={busy === "mcp"} onClick={() => void revokeMcpToken()} />}
          {mcpSettings.lastUsedAt && <small className="settings-last-used">{t("mcpLastUsed", { time: new Date(mcpSettings.lastUsedAt * 1000).toLocaleString(locale) })}</small>}
        </div>
      </section>
    </div>
    {confirmDialog.element}
    </>
  )
}
