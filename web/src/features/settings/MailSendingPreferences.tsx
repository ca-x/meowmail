import { Button } from "@astryxdesign/core/Button"
import { Card } from "@astryxdesign/core/Card"
import { IconButton } from "@astryxdesign/core/IconButton"
import { Selector } from "@astryxdesign/core/Selector"
import { Switch } from "@astryxdesign/core/Switch"
import { TextArea } from "@astryxdesign/core/TextArea"
import { TextInput } from "@astryxdesign/core/TextInput"
import { MailPlus, PencilLine, Plus, Save, Trash2 } from "lucide-react"
import { useEffect, useMemo, useState } from "react"

import { api } from "../../app/api"
import type { MailAccount, MailPreferences, Signature } from "../../app/types"
import { useI18n } from "../../i18n/I18nProvider"
import type { MessageKey } from "../../i18n/messages"
import { useImperativeConfirmDialog } from "../../shared/ui/ImperativeConfirmDialog"

type IdentityDraft = { displayName: string; signatureId: string }

export function MailSendingPreferences({ preferences, onChange, accounts, onAccountsChanged, onNotice }: {
  preferences: MailPreferences
  onChange: (preferences: MailPreferences) => void
  accounts: MailAccount[]
  onAccountsChanged: (accounts: MailAccount[]) => void
  onNotice: (key: MessageKey, error?: boolean) => void
}) {
  const { t } = useI18n()
  const deleteDialog = useImperativeConfirmDialog()
  const [signatures, setSignatures] = useState<Signature[]>([])
  const [signatureId, setSignatureId] = useState<string | "new" | null>(null)
  const [signatureName, setSignatureName] = useState("")
  const [signatureBody, setSignatureBody] = useState("")
  const [identities, setIdentities] = useState<Record<string, IdentityDraft>>({})
  const [busy, setBusy] = useState<"default" | "signature" | string | null>(null)

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

  function chooseSignature(id: string) {
    const selected = signatures.find((signature) => signature.id === id)
    if (!selected) return
    setSignatureId(selected.id)
    setSignatureName(selected.name)
    setSignatureBody(selected.bodyText)
  }

  function createSignature() {
    setSignatureId("new")
    setSignatureName("")
    setSignatureBody("")
  }

  async function chooseDefaultAccount(accountId: string) {
    const account = accounts.find((item) => item.id === accountId)
    if (!account) return
    setBusy("default")
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
    setBusy(account.id)
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
      setSignatures(selectedSignature
        ? signatures.map((signature) => signature.id === saved.id ? saved : signature)
        : [...signatures, saved])
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

  async function requestDeleteSignature() {
    const signature = selectedSignature
    if (!signature) return
    const confirmed = await deleteDialog.confirm({
      title: t("deleteSignatureConfirm"),
      description: signature.name,
      cancelLabel: t("cancel"),
      actionLabel: t("delete"),
      actionVariant: "destructive",
    })
    if (!confirmed) return
    setBusy("signature")
    try {
      await api.deleteSignature(signature.id)
      setSignatures((current) => current.filter((item) => item.id !== signature.id))
      setSignatureId(null)
      setSignatureName("")
      setSignatureBody("")
      onAccountsChanged(accounts.map((account) => account.signatureId === signature.id ? { ...account, signatureId: null } : account))
      onNotice("signatureDeleted")
    } catch {
      onNotice("genericError", true)
    } finally {
      setBusy(null)
    }
  }

  return (
    <>
      <section className="mail-preference-section" aria-labelledby="sending-settings-title">
        <div className="mail-preference-heading">
          <MailPlus aria-hidden="true" />
          <div><h3 id="sending-settings-title">{t("sendingSettings")}</h3><p>{t("sendingSettingsDescription")}</p></div>
        </div>
        <Card className="mail-preference-card" padding={4}>
          <Selector
            label={t("defaultSenderAddress")}
            value={accounts.find((account) => account.isDefault)?.id}
            onChange={(accountId) => void chooseDefaultAccount(accountId)}
            options={accounts.map((account) => ({ value: account.id, label: `${account.displayName} · ${account.email}` }))}
            isLoading={busy === "default"}
            isDisabled={!accounts.length || Boolean(busy)}
            width="100%"
          />
          <Switch label={t("emptySubjectFromBody")} description={t("emptySubjectFromBodyDescription")} value={preferences.emptySubjectFromBody} onChange={(emptySubjectFromBody) => onChange({ ...preferences, emptySubjectFromBody })} labelPosition="start" labelSpacing="spread" />
          <div className="mail-compose-style-grid">
            <Selector label={t("defaultComposeFont")} value={preferences.composeFontFamily} onChange={(composeFontFamily) => onChange({ ...preferences, composeFontFamily: composeFontFamily as MailPreferences["composeFontFamily"] })} options={[
              { value: "default", label: t("fontDefault") },
              { value: "serif", label: t("fontSerif") },
              { value: "monospace", label: t("fontMonospace") },
            ]} width="100%" />
            <Selector label={t("fontSize")} value={String(preferences.composeFontSize)} onChange={(composeFontSize) => onChange({ ...preferences, composeFontSize: Number(composeFontSize) })} options={[11, 12, 13, 14, 15, 16, 18, 20, 22, 24].map((size) => ({ value: String(size), label: `${size}px` }))} width="100%" />
            <label className="mail-color-field"><span>{t("fontColor")}</span><input type="color" value={preferences.composeFontColor} onChange={(event) => onChange({ ...preferences, composeFontColor: event.target.value.toUpperCase() })} /></label>
          </div>
          <div className={`mail-compose-preview font-${preferences.composeFontFamily}`} style={{ fontSize: preferences.composeFontSize, color: preferences.composeFontColor }}>
            <small>{t("preview")}</small><p>{t("composeFontPreview")}</p>
          </div>

          <div className="mail-signature-block">
            <div className="mail-signature-toolbar">
              <Selector label={t("emailSignatures")} value={typeof signatureId === "string" && signatureId !== "new" ? signatureId : undefined} onChange={chooseSignature} options={signatures.map((signature) => ({ value: signature.id, label: signature.name }))} placeholder={t("selectOrCreateSignature")} width="100%" isDisabled={!signatures.length} />
              <Button label={t("newSignature")} icon={<Plus aria-hidden="true" />} variant="secondary" onClick={createSignature} />
            </div>
            {signatureId ? (
              <div className="mail-signature-editor">
                <TextInput label={`${t("signatureName")} · ${t("required")}`} value={signatureName} onChange={setSignatureName} placeholder={t("signatureNamePlaceholder")} width="100%" />
                <TextArea label={`${t("signatureContent")} · ${t("required")}`} value={signatureBody} onChange={setSignatureBody} placeholder={t("signatureContentPlaceholder")} rows={6} width="100%" />
                <div className="settings-button-row mail-signature-actions">
                  {selectedSignature && <Button label={t("delete")} icon={<Trash2 aria-hidden="true" />} variant="ghost" isDisabled={busy === "signature"} onClick={() => void requestDeleteSignature()} />}
                  <Button label={t("save")} icon={<Save aria-hidden="true" />} variant="secondary" isLoading={busy === "signature"} isDisabled={!signatureName.trim() || busy === "signature"} onClick={() => void saveSignature()} />
                </div>
              </div>
            ) : (
              <div className="mail-signature-empty"><PencilLine aria-hidden="true" /><p>{signatures.length ? t("selectOrCreateSignature") : t("noSignatures")}</p></div>
            )}
          </div>

          <div className="mail-identities-block">
            <div className="mail-identities-heading"><strong>{t("accountNicknameAndSignature")}</strong><small>{t("accountNicknameAndSignatureDescription")}</small></div>
            <div className="mail-identity-list">
              {accounts.map((account) => {
                const draft = identities[account.id] || { displayName: account.displayName, signatureId: account.signatureId || "" }
                return (
                  <div className="mail-identity-row" key={account.id}>
                    <span className="mail-identity-account"><strong>{account.email}</strong><small>{account.isDefault ? t("default") : account.smtp.host}</small></span>
                    <TextInput label={`${t("nickname")} ${account.email}`} isLabelHidden value={draft.displayName} onChange={(displayName) => setIdentities({ ...identities, [account.id]: { ...draft, displayName } })} width="100%" />
                    <Selector label={`${t("emailSignature")} ${account.email}`} isLabelHidden value={draft.signatureId || "none"} onChange={(signatureId) => setIdentities({ ...identities, [account.id]: { ...draft, signatureId: signatureId === "none" ? "" : signatureId } })} options={[{ value: "none", label: t("noSignature") }, ...signatures.map((signature) => ({ value: signature.id, label: signature.name }))]} width="100%" />
                    <IconButton label={`${t("save")} ${account.email}`} icon={<Save aria-hidden="true" />} variant="ghost" size="sm" isLoading={busy === account.id} isDisabled={Boolean(busy)} onClick={() => void saveIdentity(account)} />
                  </div>
                )
              })}
            </div>
          </div>
        </Card>
      </section>
      {deleteDialog.element}
    </>
  )
}
