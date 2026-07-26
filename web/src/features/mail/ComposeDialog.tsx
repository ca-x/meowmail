import { useMemo, useState, type FormEvent } from "react"
import { AtSign, ChevronDown, Paperclip, Send, X } from "lucide-react"

import { api } from "../../app/api"
import type { MailAccount } from "../../app/types"
import { useI18n } from "../../i18n/I18nProvider"

export function ComposeDialog({ accounts, activeAccountId, onClose, onSent }: {
  accounts: MailAccount[]
  activeAccountId: string | null
  onClose: () => void
  onSent: () => void
}) {
  const { t } = useI18n()
  const defaultAccount = useMemo(
    () => accounts.find((account) => account.id === activeAccountId) || accounts.find((account) => account.isDefault) || accounts[0],
    [accounts, activeAccountId],
  )
  const [accountId, setAccountId] = useState(defaultAccount?.id || "")
  const [to, setTo] = useState("")
  const [cc, setCc] = useState("")
  const [bcc, setBcc] = useState("")
  const [subject, setSubject] = useState("")
  const [body, setBody] = useState("")
  const [showCopies, setShowCopies] = useState(false)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)

  async function submit(event: FormEvent) {
    event.preventDefault()
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
      setBusy(false)
    }
  }

  return (
    <div className="modal-backdrop compose-backdrop" role="presentation">
      <section className="modal-card compose-dialog" role="dialog" aria-modal="true" aria-labelledby="compose-title">
        <header className="compose-header">
          <div>
            <span className="compose-icon"><AtSign size={18} /></span>
            <h2 id="compose-title">{t("compose")}</h2>
          </div>
          <button className="icon-button" type="button" onClick={onClose} aria-label={t("close")}><X size={18} /></button>
        </header>
        <form onSubmit={submit} className="compose-form">
          <div className="compose-line">
            <span className="compose-label">From</span>
            <div className="compose-account-select">
              <select value={accountId} onChange={(event) => setAccountId(event.target.value)}>
                {accounts.map((account) => <option key={account.id} value={account.id}>{account.displayName} · {account.email}</option>)}
              </select>
              <ChevronDown size={14} />
            </div>
          </div>
          <div className="compose-line">
            <label className="compose-label" htmlFor="compose-to">{t("to")}</label>
            <input id="compose-to" value={to} onChange={(event) => setTo(event.target.value)} placeholder="name@example.com" required />
            <button className="copy-toggle" type="button" onClick={() => setShowCopies((value) => !value)}>Cc Bcc</button>
          </div>
          {showCopies && (
            <>
              <div className="compose-line"><label className="compose-label" htmlFor="compose-cc">{t("cc")}</label><input id="compose-cc" value={cc} onChange={(event) => setCc(event.target.value)} /></div>
              <div className="compose-line"><label className="compose-label" htmlFor="compose-bcc">{t("bcc")}</label><input id="compose-bcc" value={bcc} onChange={(event) => setBcc(event.target.value)} /></div>
            </>
          )}
          <div className="compose-line subject-line">
            <label className="compose-label" htmlFor="compose-subject">{t("subject")}</label>
            <input id="compose-subject" value={subject} onChange={(event) => setSubject(event.target.value)} />
          </div>
          <textarea
            className="compose-body"
            value={body}
            onChange={(event) => setBody(event.target.value)}
            placeholder={`${t("message")}…`}
            required
          />
          {error && <div className="inline-notice error">{error}</div>}
          <footer className="compose-footer">
            <button className="compose-attachment" type="button" disabled title="Attachments are coming next"><Paperclip size={18} /></button>
            <div className="footer-actions">
              <button className="secondary-button" type="button" onClick={onClose}>{t("cancel")}</button>
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
