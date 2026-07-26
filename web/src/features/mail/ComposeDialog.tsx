import { useMemo, useRef, useState, type FormEvent } from "react"
import { AtSign, ChevronDown, Paperclip, Send, X } from "lucide-react"

import { api } from "../../app/api"
import { useDialogBehavior } from "../../app/useDialogBehavior"
import type { MailAccount } from "../../app/types"
import { useI18n } from "../../i18n/I18nProvider"

export interface ComposeDraft {
  to?: string
  cc?: string
  bcc?: string
  subject?: string
  body?: string
}

export function ComposeDialog({ accounts, activeAccountId, draft, onClose, onSent }: {
  accounts: MailAccount[]
  activeAccountId: string | null
  draft?: ComposeDraft | null
  onClose: () => void
  onSent: () => void
}) {
  const { t } = useI18n()
  const defaultAccount = useMemo(
    () => accounts.find((account) => account.id === activeAccountId) || accounts.find((account) => account.isDefault) || accounts[0],
    [accounts, activeAccountId],
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
  const dialogRef = useRef<HTMLElement>(null)
  const busyRef = useRef(false)
  const dirty = Boolean(to || cc || bcc || subject || body)

  function requestClose() {
    if (busyRef.current) return
    if (!dirty || window.confirm(t("discardDraftConfirm"))) onClose()
  }

  useDialogBehavior(dialogRef, requestClose)

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
    <div className="modal-backdrop compose-backdrop" role="presentation">
      <section ref={dialogRef} className="modal-card compose-dialog" role="dialog" aria-modal="true" aria-labelledby="compose-title" tabIndex={-1}>
        <header className="compose-header">
          <div>
            <span className="compose-icon"><AtSign size={18} /></span>
            <h2 id="compose-title">{t("compose")}</h2>
          </div>
          <button className="icon-button" type="button" onClick={requestClose} disabled={busy} aria-label={t("close")}><X size={18} /></button>
        </header>
        <form onSubmit={submit} className="compose-form">
          <div className="compose-line">
            <label className="compose-label" htmlFor="compose-from">{t("from")}</label>
            <div className="compose-account-select">
              <select id="compose-from" value={accountId} onChange={(event) => setAccountId(event.target.value)}>
                {accounts.map((account) => <option key={account.id} value={account.id}>{account.displayName} · {account.email}</option>)}
              </select>
              <ChevronDown size={14} />
            </div>
          </div>
          <div className="compose-line">
            <label className="compose-label" htmlFor="compose-to">{t("to")}</label>
            <input id="compose-to" value={to} onChange={(event) => setTo(event.target.value)} placeholder={t("recipientPlaceholder")} autoComplete="email" data-dialog-initial-focus required />
            <button className="copy-toggle" type="button" aria-expanded={showCopies} onClick={() => setShowCopies((value) => !value)}>{t("ccBcc")}</button>
          </div>
          {showCopies && (
            <>
              <div className="compose-line"><label className="compose-label" htmlFor="compose-cc">{t("cc")}</label><input id="compose-cc" value={cc} onChange={(event) => setCc(event.target.value)} placeholder={t("recipientPlaceholder")} autoComplete="email" /></div>
              <div className="compose-line"><label className="compose-label" htmlFor="compose-bcc">{t("bcc")}</label><input id="compose-bcc" value={bcc} onChange={(event) => setBcc(event.target.value)} placeholder={t("recipientPlaceholder")} autoComplete="email" /></div>
            </>
          )}
          <div className="compose-line subject-line">
            <label className="compose-label" htmlFor="compose-subject">{t("subject")}</label>
            <input id="compose-subject" value={subject} onChange={(event) => setSubject(event.target.value)} placeholder={t("subjectPlaceholder")} />
          </div>
          <textarea
            className="compose-body"
            value={body}
            onChange={(event) => setBody(event.target.value)}
            placeholder={t("messagePlaceholder")}
            required
          />
          {error && <div className="inline-notice error">{error}</div>}
          <footer className="compose-footer">
            <button className="compose-attachment" type="button" disabled title={t("attachmentsComingSoon")} aria-label={t("attachmentsComingSoon")}><Paperclip size={18} /></button>
            <div className="footer-actions">
              <button className="secondary-button" type="button" onClick={requestClose} disabled={busy}>{t("cancel")}</button>
              <button className="primary-button send-button" type="submit" disabled={busy || !accountId || !to || !body}>
                {busy ? <span className="spinner spinner-small" /> : <Send size={16} />}
                {busy ? t("sending") : t("send")}
              </button>
            </div>
          </footer>
        </form>
      </section>
    </div>
  )
}

function addresses(value: string) {
  return value.split(/[;,]/).map((address) => address.trim()).filter(Boolean)
}
