import { Banner } from "@astryxdesign/core/Banner"
import { Button } from "@astryxdesign/core/Button"
import { Dialog, DialogHeader } from "@astryxdesign/core/Dialog"
import { IconButton } from "@astryxdesign/core/IconButton"
import { Layout, LayoutContent, LayoutFooter } from "@astryxdesign/core/Layout"
import { Selector } from "@astryxdesign/core/Selector"
import { TextArea } from "@astryxdesign/core/TextArea"
import { TextInput } from "@astryxdesign/core/TextInput"
import { AtSign, Paperclip, Send } from "lucide-react"
import { useEffect, useMemo, useRef, useState, type CSSProperties, type FormEvent } from "react"

import { api } from "../../app/api"
import type { MailAccount, MailPreferences } from "../../app/types"
import { useI18n } from "../../i18n/I18nProvider"

export interface ComposeDraft {
  accountId?: string
  to?: string
  cc?: string
  bcc?: string
  subject?: string
  body?: string
}

export function ComposeDialog({ isOpen = true, accounts, activeAccountId, preferences, draft, onClose, onSent }: {
  isOpen?: boolean
  accounts: MailAccount[]
  activeAccountId: string | null
  preferences: MailPreferences
  draft?: ComposeDraft | null
  onClose: () => void
  onSent: () => void
}) {
  const { t } = useI18n()
  const defaultAccount = useMemo(
    () => accounts.find((account) => account.id === draft?.accountId)
      || accounts.find((account) => account.id === activeAccountId)
      || accounts.find((account) => account.isDefault)
      || accounts[0],
    [accounts, activeAccountId, draft?.accountId],
  )
  const [accountId, setAccountId] = useState(defaultAccount?.id || "")
  const [to, setTo] = useState(draft?.to || "")
  const [cc, setCc] = useState(draft?.cc || "")
  const [bcc, setBcc] = useState(draft?.bcc || "")
  const [subject, setSubject] = useState(draft?.subject || "")
  const [body, setBody] = useState(draft?.body || "")
  const [showCopies, setShowCopies] = useState(Boolean(draft?.cc || draft?.bcc))
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const busyRef = useRef(false)
  const dirty = Boolean(to || cc || bcc || subject || body)
  const editorStyle = {
    "--compose-font-size": `${preferences.composeFontSize}px`,
    "--compose-font-color": preferences.composeFontColor,
  } as CSSProperties

  useEffect(() => {
    if (!isOpen) return
    setAccountId(defaultAccount?.id || "")
    setTo(draft?.to || "")
    setCc(draft?.cc || "")
    setBcc(draft?.bcc || "")
    setSubject(draft?.subject || "")
    setBody(draft?.body || "")
    setShowCopies(Boolean(draft?.cc || draft?.bcc))
    setError(null)
  }, [defaultAccount?.id, draft, isOpen])

  function requestClose() {
    if (busyRef.current) return
    if (!dirty || window.confirm(t("discardDraftConfirm"))) onClose()
  }

  async function submit(event: FormEvent) {
    event.preventDefault()
    busyRef.current = true
    setBusy(true)
    setError(null)
    try {
      await api.sendMessage({
        accountId,
        to: addresses(to),
        cc: addresses(cc),
        bcc: addresses(bcc),
        subject,
        textBody: body,
      })
      onSent()
    } catch {
      setError(t("genericError"))
    } finally {
      busyRef.current = false
      setBusy(false)
    }
  }

  return (
    <Dialog
      className="compose-dialog"
      isOpen={isOpen}
      onOpenChange={(open) => { if (!open) requestClose() }}
      purpose="form"
      width={760}
      maxHeight="92dvh"
      padding={0}
      aria-label={t("compose")}
    >
      <form className="compose-form" onSubmit={submit}>
        <Layout
          className="compose-dialog-layout"
          height="fill"
          padding={4}
          header={
            <DialogHeader
              title={t("compose")}
              startContent={<span className="compose-dialog-icon"><AtSign aria-hidden="true" /></span>}
              onOpenChange={busy ? undefined : (open) => { if (!open) requestClose() }}
              hasDivider
            />
          }
          content={
            <LayoutContent className="compose-dialog-content" padding={0} isScrollable>
              <div className="compose-fields">
                <Selector
                  label={t("from")}
                  value={accountId}
                  onChange={setAccountId}
                  options={accounts.map((account) => ({ value: account.id, label: `${account.displayName} · ${account.email}` }))}
                  width="100%"
                />

                <div className="compose-recipient-row">
                  <label htmlFor="compose-to">{t("to")}</label>
                  <input
                    id="compose-to"
                    className="compose-native-input"
                    value={to}
                    onChange={(event) => setTo(event.target.value)}
                    placeholder={t("recipientPlaceholder")}
                    autoComplete="email"
                    data-autofocus
                    required
                  />
                  <Button label={t("ccBcc")} variant="ghost" size="sm" onClick={() => setShowCopies((value) => !value)} aria-expanded={showCopies} />
                </div>
                {showCopies && (
                  <div className="compose-copy-fields">
                    <label><span>{t("cc")}</span><input className="compose-native-input" value={cc} onChange={(event) => setCc(event.target.value)} placeholder={t("recipientPlaceholder")} autoComplete="email" /></label>
                    <label><span>{t("bcc")}</span><input className="compose-native-input" value={bcc} onChange={(event) => setBcc(event.target.value)} placeholder={t("recipientPlaceholder")} autoComplete="email" /></label>
                  </div>
                )}

                <TextInput label={t("subject")} value={subject} onChange={setSubject} placeholder={t("subjectPlaceholder")} width="100%" />
                <TextArea
                  className={`compose-body-field font-${preferences.composeFontFamily}`}
                  style={editorStyle}
                  label={`${t("message")} · ${t("required")}`}
                  value={body}
                  onChange={setBody}
                  placeholder={t("messagePlaceholder")}
                  rows={14}
                  width="100%"
                />
                {error && <Banner status="error" title={error} container="section" />}
              </div>
            </LayoutContent>
          }
          footer={
            <LayoutFooter className="compose-dialog-footer" padding={3} hasDivider>
              <IconButton label={t("attachmentsComingSoon")} icon={<Paperclip aria-hidden="true" />} variant="ghost" isDisabled tooltip={t("attachmentsComingSoon")} />
              <span className="compose-dialog-actions">
                <Button label={t("cancel")} variant="secondary" isDisabled={busy} onClick={requestClose} />
                <Button
                  label={busy ? t("sending") : t("send")}
                  icon={<Send aria-hidden="true" />}
                  variant="primary"
                  type="submit"
                  isLoading={busy}
                  isDisabled={busy || !accountId || !to || !body}
                />
              </span>
            </LayoutFooter>
          }
        />
      </form>
    </Dialog>
  )
}

function addresses(value: string) {
  return value.split(/[;,]/).map((address) => address.trim()).filter(Boolean)
}
