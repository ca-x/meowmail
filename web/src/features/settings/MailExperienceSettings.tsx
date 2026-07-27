import { useEffect, useMemo, useState } from "react"
import {
  BookOpen, ChevronDown, MailPlus, MessageSquareReply, MonitorUp, PencilLine,
  Plus, Save, Trash2,
} from "lucide-react"

import { api } from "../../app/api"
import type { MailAccount, MailPreferences, Signature } from "../../app/types"
import { useI18n } from "../../i18n/I18nProvider"
import type { MessageKey } from "../../i18n/messages"

type IdentityDraft = { displayName: string; signatureId: string }

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
  const [signatures, setSignatures] = useState<Signature[]>([])
  const [signatureId, setSignatureId] = useState<string | "new" | null>(null)
  const [signatureName, setSignatureName] = useState("")
  const [signatureBody, setSignatureBody] = useState("")
  const [identities, setIdentities] = useState<Record<string, IdentityDraft>>({})
  const [busy, setBusy] = useState<"preferences" | "signature" | "identity" | null>(null)

  useEffect(() => setPreferences(initialPreferences), [initialPreferences])
  useEffect(() => {
    setIdentities(Object.fromEntries(accounts.map((account) => [account.id, {
      displayName: account.displayName,
      signatureId: account.signatureId || "",
    }])))
  }, [accounts])
  useEffect(() => {
    api.signatures().then(setSignatures).catch(() => onNotice("genericError", true))
    // Settings are loaded once per dialog instance; the callback does not carry state.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  const selectedSignature = useMemo(
    () => signatures.find((signature) => signature.id === signatureId) || null,
    [signatureId, signatures],
  )

  function chooseSignature(id: string | "new") {
    setSignatureId(id)
    const selected = signatures.find((signature) => signature.id === id)
    setSignatureName(selected?.name || "")
    setSignatureBody(selected?.bodyText || "")
  }

  async function savePreferences() {
    setBusy("preferences")
    try {
      const saved = await api.updateMailPreferences(preferences)
      setPreferences(saved)
      onPreferencesChanged(saved)
      onNotice("mailPreferencesSaved")
    } catch {
      onNotice("mailPreferencesInvalid", true)
    } finally {
      setBusy(null)
    }
  }

  async function chooseDefaultAccount(accountId: string) {
    const account = accounts.find((item) => item.id === accountId)
    if (!account) return
    setBusy("identity")
    try {
      const saved = await api.updateAccountIdentity(account.id, {
        displayName: identities[account.id]?.displayName || account.displayName,
        signatureId: identities[account.id]?.signatureId || null,
        isDefault: true,
      })
      onAccountsChanged(accounts.map((item) => ({ ...item, isDefault: item.id === saved.id, ...(item.id === saved.id ? saved : {}) })))
      onNotice("senderIdentitySaved")
    } catch {
      onNotice("genericError", true)
    } finally {
      setBusy(null)
    }
  }

  async function saveIdentity(account: MailAccount) {
    const draft = identities[account.id]
    if (!draft) return
    setBusy("identity")
    try {
      const saved = await api.updateAccountIdentity(account.id, {
        displayName: draft.displayName,
        signatureId: draft.signatureId || null,
        isDefault: account.isDefault,
      })
      onAccountsChanged(accounts.map((item) => item.id === saved.id ? saved : item))
      onNotice("senderIdentitySaved")
    } catch {
      onNotice("senderIdentityInvalid", true)
    } finally {
      setBusy(null)
    }
  }

  async function saveSignature() {
    setBusy("signature")
    try {
      const saved = selectedSignature
        ? await api.updateSignature(selectedSignature.id, { name: signatureName, bodyText: signatureBody })
        : await api.createSignature({ name: signatureName, bodyText: signatureBody })
      const next = selectedSignature
        ? signatures.map((signature) => signature.id === saved.id ? saved : signature)
        : [...signatures, saved]
      setSignatures(next)
      setSignatureId(saved.id)
      setSignatureName(saved.name)
      setSignatureBody(saved.bodyText)
      onNotice("signatureSaved")
    } catch {
      onNotice("signatureInvalid", true)
    } finally {
      setBusy(null)
    }
  }

  async function removeSignature() {
    if (!selectedSignature || !confirm(t("deleteSignatureConfirm"))) return
    setBusy("signature")
    try {
      await api.deleteSignature(selectedSignature.id)
      setSignatures((current) => current.filter((signature) => signature.id !== selectedSignature.id))
      setSignatureId(null)
      setSignatureName("")
      setSignatureBody("")
      onAccountsChanged(accounts.map((account) => account.signatureId === selectedSignature.id ? { ...account, signatureId: null } : account))
      onNotice("signatureDeleted")
    } catch {
      onNotice("genericError", true)
    } finally {
      setBusy(null)
    }
  }

  return (
    <div className="mail-experience-form">
      <section className="settings-section">
        <div className="settings-section-heading"><BookOpen size={18} /><div><h3>{t("readingSettings")}</h3><p>{t("readingSettingsDescription")}</p></div></div>
        <div className="settings-card preference-card">
          <div className="preference-block">
            <div className="preference-block-heading"><strong>{t("readingMode")}</strong><small>{t("readingModeDescription")}</small></div>
            <div className="visual-choice-grid two">
              <button type="button" className={preferences.readingMode === "preview" ? "active" : ""} aria-pressed={preferences.readingMode === "preview"} onClick={() => setPreferences({ ...preferences, readingMode: "preview" })}>
                <span className="layout-preview three-pane"><i /><i /><i /></span><strong>{t("previewMode")}</strong>
              </button>
              <button type="button" className={preferences.readingMode === "list" ? "active" : ""} aria-pressed={preferences.readingMode === "list"} onClick={() => setPreferences({ ...preferences, readingMode: "list" })}>
                <span className="layout-preview list-only"><i /><i /><i /></span><strong>{t("listMode")}</strong>
              </button>
            </div>
          </div>
          <div className="preference-block">
            <div className="preference-block-heading"><strong>{t("listDensity")}</strong></div>
            <div className="segmented-control compact">
              <button type="button" className={preferences.listDensity === "default" ? "active" : ""} aria-pressed={preferences.listDensity === "default"} onClick={() => setPreferences({ ...preferences, listDensity: "default" })}>{t("densityDefault")}</button>
              <button type="button" className={preferences.listDensity === "compact" ? "active" : ""} aria-pressed={preferences.listDensity === "compact"} onClick={() => setPreferences({ ...preferences, listDensity: "compact" })}>{t("densityCompact")}</button>
            </div>
          </div>
          <Toggle label={t("conversationMode")} description={t("conversationModeDescription")} checked={preferences.conversationMode} onChange={(value) => setPreferences({ ...preferences, conversationMode: value })} />
          <Toggle label={t("aggregatePromotions")} description={t("aggregatePromotionsDescription")} checked={preferences.aggregatePromotions} onChange={(value) => setPreferences({ ...preferences, aggregatePromotions: value })} />
          <div className="compact-options-grid">
            <Check label={t("showMessageSummary")} checked={preferences.showSummary} onChange={(value) => setPreferences({ ...preferences, showSummary: value })} />
            <Check label={t("showMessageSize")} checked={preferences.showMessageSize} onChange={(value) => setPreferences({ ...preferences, showMessageSize: value })} />
            <Check label={t("showAttachmentPreviewOption")} checked={preferences.showAttachmentPreview} onChange={(value) => setPreferences({ ...preferences, showAttachmentPreview: value })} />
          </div>
          <div className="setting-row preference-row">
            <div className="setting-label"><span>{t("afterMessageAction")}</span></div>
            <div className="segmented-control compact">
              <button type="button" className={preferences.afterAction === "nextMessage" ? "active" : ""} onClick={() => setPreferences({ ...preferences, afterAction: "nextMessage" })}>{t("readNextMessage")}</button>
              <button type="button" className={preferences.afterAction === "messageList" ? "active" : ""} onClick={() => setPreferences({ ...preferences, afterAction: "messageList" })}>{t("returnToMessageList")}</button>
            </div>
          </div>
          <Toggle label={t("plainTextReading")} description={t("plainTextReadingDescription")} checked={preferences.plainTextReading} onChange={(value) => setPreferences({ ...preferences, plainTextReading: value })} />
        </div>
      </section>

      <section className="settings-section">
        <div className="settings-section-heading"><MailPlus size={18} /><div><h3>{t("sendingSettings")}</h3><p>{t("sendingSettingsDescription")}</p></div></div>
        <div className="settings-card preference-card">
          <label className="form-field wide"><span>{t("defaultSenderAddress")}</span><span className="select-shell"><select value={accounts.find((account) => account.isDefault)?.id || ""} disabled={busy === "identity"} onChange={(event) => void chooseDefaultAccount(event.target.value)}>{accounts.map((account) => <option key={account.id} value={account.id}>{account.displayName} &lt;{account.email}&gt;</option>)}</select><ChevronDown size={14} /></span></label>
          <Toggle label={t("emptySubjectFromBody")} description={t("emptySubjectFromBodyDescription")} checked={preferences.emptySubjectFromBody} onChange={(value) => setPreferences({ ...preferences, emptySubjectFromBody: value })} />
          <div className="font-preference-grid">
            <label className="form-field"><span>{t("defaultComposeFont")}</span><select value={preferences.composeFontFamily} onChange={(event) => setPreferences({ ...preferences, composeFontFamily: event.target.value as MailPreferences["composeFontFamily"] })}><option value="default">{t("fontDefault")}</option><option value="serif">{t("fontSerif")}</option><option value="monospace">{t("fontMonospace")}</option></select></label>
            <label className="form-field"><span>{t("fontSize")}</span><select value={preferences.composeFontSize} onChange={(event) => setPreferences({ ...preferences, composeFontSize: Number(event.target.value) })}>{[11, 12, 13, 14, 15, 16, 18, 20, 22, 24].map((size) => <option key={size} value={size}>{size}px</option>)}</select></label>
            <label className="color-field"><span>{t("fontColor")}</span><input type="color" value={preferences.composeFontColor} onChange={(event) => setPreferences({ ...preferences, composeFontColor: event.target.value.toUpperCase() })} /></label>
          </div>
          <div className={`compose-font-preview font-${preferences.composeFontFamily}`} style={{ fontSize: preferences.composeFontSize, color: preferences.composeFontColor }}><small>{t("preview")}</small><p>{t("composeFontPreview")}</p></div>

          <div className="signature-management">
            <div className="signature-sidebar">
              <div><strong>{t("emailSignatures")}</strong><button type="button" onClick={() => chooseSignature("new")}><Plus size={14} />{t("newSignature")}</button></div>
              {!signatures.length && <small>{t("noSignatures")}</small>}
              {signatures.map((signature) => <button type="button" key={signature.id} className={signatureId === signature.id ? "active" : ""} onClick={() => chooseSignature(signature.id)}>{signature.name}</button>)}
            </div>
            <div className="signature-editor">
              {signatureId ? (
                <>
                  <label className="form-field"><span>{t("signatureName")}</span><input value={signatureName} onChange={(event) => setSignatureName(event.target.value)} placeholder={t("signatureNamePlaceholder")} /></label>
                  <label className="form-field"><span>{t("signatureContent")}</span><textarea value={signatureBody} onChange={(event) => setSignatureBody(event.target.value)} placeholder={t("signatureContentPlaceholder")} /></label>
                  <div className="editor-actions"><button className="quiet-button danger-text" type="button" disabled={!selectedSignature || busy === "signature"} onClick={() => void removeSignature()}><Trash2 size={14} />{t("delete")}</button><button className="secondary-button" type="button" disabled={!signatureName.trim() || busy === "signature"} onClick={() => void saveSignature()}><Save size={14} />{t("save")}</button></div>
                </>
              ) : <div className="signature-empty"><PencilLine size={20} /><p>{t("selectOrCreateSignature")}</p></div>}
            </div>
          </div>

          <div className="identity-table">
            <div className="identity-table-heading"><strong>{t("accountNicknameAndSignature")}</strong><small>{t("accountNicknameAndSignatureDescription")}</small></div>
            {accounts.map((account) => {
              const draft = identities[account.id] || { displayName: account.displayName, signatureId: account.signatureId || "" }
              return <div className="identity-row" key={account.id}><span><strong>{account.email}</strong><small>{account.isDefault ? t("default") : account.smtp.host}</small></span><input aria-label={`${t("nickname")} ${account.email}`} value={draft.displayName} onChange={(event) => setIdentities({ ...identities, [account.id]: { ...draft, displayName: event.target.value } })} /><select aria-label={`${t("emailSignature")} ${account.email}`} value={draft.signatureId} onChange={(event) => setIdentities({ ...identities, [account.id]: { ...draft, signatureId: event.target.value } })}><option value="">{t("noSignature")}</option>{signatures.map((signature) => <option key={signature.id} value={signature.id}>{signature.name}</option>)}</select><button className="icon-button small" type="button" disabled={busy === "identity"} onClick={() => void saveIdentity(account)} aria-label={`${t("save")} ${account.email}`}><Save size={14} /></button></div>
            })}
          </div>
        </div>
      </section>

      <section className="settings-section">
        <div className="settings-section-heading"><MessageSquareReply size={18} /><div><h3>{t("replyAndForward")}</h3><p>{t("replyAndForwardDescription")}</p></div></div>
        <div className="settings-card preference-card">
          <Toggle label={t("attachOriginalOnReply")} description={t("attachOriginalOnReplyDescription")} checked={preferences.attachOriginalOnReply} onChange={(value) => setPreferences({ ...preferences, attachOriginalOnReply: value })} />
          <div className="setting-row preference-row"><div className="setting-label"><span>{t("replySubjectPrefix")}</span></div><div className="segmented-control compact"><button type="button" className={preferences.subjectPrefixLanguage === "chinese" ? "active" : ""} onClick={() => setPreferences({ ...preferences, subjectPrefixLanguage: "chinese" })}>{t("useChinesePrefix")}</button><button type="button" className={preferences.subjectPrefixLanguage === "english" ? "active" : ""} onClick={() => setPreferences({ ...preferences, subjectPrefixLanguage: "english" })}>{t("useEnglishPrefix")}</button></div></div>
          <Toggle label={t("automaticForwarding")} description={t("automaticForwardingDescription")} checked={preferences.autoForwardEnabled} onChange={(value) => setPreferences({ ...preferences, autoForwardEnabled: value })} />
          {preferences.autoForwardEnabled && <label className="form-field wide inset-field"><span>{t("forwardToAddress")}</span><input type="email" value={preferences.autoForwardAddress || ""} onChange={(event) => setPreferences({ ...preferences, autoForwardAddress: event.target.value })} placeholder={t("forwardToAddressPlaceholder")} /></label>}
          <Toggle label={t("automaticReply")} description={t("automaticReplyDescription")} checked={preferences.autoReplyEnabled} onChange={(value) => setPreferences({ ...preferences, autoReplyEnabled: value })} />
          {preferences.autoReplyEnabled && <label className="form-field wide inset-field"><span>{t("automaticReplyContent")}</span><textarea value={preferences.autoReplyText} onChange={(event) => setPreferences({ ...preferences, autoReplyText: event.target.value })} placeholder={t("automaticReplyContentPlaceholder")} /></label>}
          <div className="automatic-mail-note"><MonitorUp size={15} /><span>{t("automaticMailSafetyNote")}</span></div>
        </div>
      </section>

      <div className="preference-save-bar"><span>{t("mailPreferencesSaveHint")}</span><button className="primary-button" type="button" disabled={busy === "preferences"} onClick={() => void savePreferences()}><Save size={15} />{t("saveMailPreferences")}</button></div>
    </div>
  )
}

function Toggle({ label, description, checked, onChange }: { label: string; description?: string; checked: boolean; onChange: (value: boolean) => void }) {
  return <label className="toggle-row compact-toggle"><span><strong>{label}</strong>{description && <small>{description}</small>}</span><input type="checkbox" checked={checked} onChange={(event) => onChange(event.target.checked)} /><span className="toggle" aria-hidden="true" /></label>
}

function Check({ label, checked, onChange }: { label: string; checked: boolean; onChange: (value: boolean) => void }) {
  return <label className="check-row inline-check"><input type="checkbox" checked={checked} onChange={(event) => onChange(event.target.checked)} /><span className="custom-check">✓</span>{label}</label>
}
