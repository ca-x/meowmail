import { useEffect, useState, type FormEvent } from "react"
import { CheckCircle2, ChevronDown, MailPlus, Server, ShieldCheck, Trash2, X } from "lucide-react"

import { api } from "../../app/api"
import type { AccountInput, ConnectionSecurity, MailAccount, ProxyKind } from "../../app/types"
import { useI18n } from "../../i18n/I18nProvider"

interface Props {
  account: MailAccount | null
  onClose: () => void
  onSaved: (account: MailAccount) => void
  onDeleted: () => void
}

function emptyInput(): AccountInput {
  return {
    displayName: "",
    email: "",
    username: "",
    password: "",
    imap: { host: "", port: 993, security: "tls" },
    smtp: { host: "", port: 465, security: "tls" },
    proxy: { kind: "direct" },
    isDefault: false,
  }
}

function fromAccount(account: MailAccount): AccountInput {
  return {
    displayName: account.displayName,
    email: account.email,
    username: account.username,
    password: "",
    imap: { ...account.imap },
    smtp: { ...account.smtp },
    proxy: {
      kind: account.proxy.kind,
      host: account.proxy.host || "",
      port: account.proxy.port || undefined,
      username: account.proxy.username || "",
      password: "",
    },
    isDefault: account.isDefault,
  }
}

export function AccountDialog({ account, onClose, onSaved, onDeleted }: Props) {
  const { t } = useI18n()
  const [input, setInput] = useState<AccountInput>(() => account ? fromAccount(account) : emptyInput())
  const [busy, setBusy] = useState<"save" | "test" | "delete" | null>(null)
  const [message, setMessage] = useState<string | null>(null)

  useEffect(() => setInput(account ? fromAccount(account) : emptyInput()), [account])

  function preset(kind: "gmail" | "outlook" | "custom") {
    if (kind === "custom") return setInput((value) => ({ ...value, imap: { ...value.imap, host: "" }, smtp: { ...value.smtp, host: "" } }))
    setInput((value) => ({
      ...value,
      displayName: value.displayName || (kind === "gmail" ? "Gmail" : "Outlook"),
      imap: kind === "gmail"
        ? { host: "imap.gmail.com", port: 993, security: "tls" }
        : { host: "outlook.office365.com", port: 993, security: "tls" },
      smtp: kind === "gmail"
        ? { host: "smtp.gmail.com", port: 465, security: "tls" }
        : { host: "smtp.office365.com", port: 587, security: "starttls" },
    }))
  }

  async function submit(event: FormEvent) {
    event.preventDefault()
    setBusy("save")
    setMessage(null)
    try {
      const payload = normalize(input, Boolean(account))
      const saved = account
        ? await api.updateAccount(account.id, payload)
        : await api.createAccount(payload)
      onSaved(saved)
    } catch {
      setMessage(t("genericError"))
    } finally {
      setBusy(null)
    }
  }

  async function testConnection() {
    setBusy("test")
    setMessage(null)
    try {
      if (account && !input.password && !input.proxy.password) await api.testSavedAccount(account.id)
      else await api.testAccount(normalize(input, false))
      setMessage(t("connectionOk"))
    } catch {
      setMessage(t("genericError"))
    } finally {
      setBusy(null)
    }
  }

  async function removeAccount() {
    if (!account || !confirm(`${t("delete")} ${account.displayName}?`)) return
    setBusy("delete")
    try {
      await api.deleteAccount(account.id)
      onDeleted()
    } catch {
      setMessage(t("genericError"))
      setBusy(null)
    }
  }

  return (
    <div className="modal-backdrop" role="presentation" onMouseDown={(event) => event.target === event.currentTarget && onClose()}>
      <section className="modal-card account-dialog" role="dialog" aria-modal="true" aria-labelledby="account-dialog-title">
        <header className="modal-header">
          <div className="modal-title-group">
            <span className="modal-icon"><MailPlus size={20} /></span>
            <div><p>{t("accounts")}</p><h2 id="account-dialog-title">{account ? t("editAccount") : t("addAccount")}</h2></div>
          </div>
          <button className="icon-button" type="button" onClick={onClose} aria-label={t("close")}><X size={18} /></button>
        </header>

        <form onSubmit={submit} className="modal-form">
          <div className="preset-row" aria-label={t("preset")}>
            <button type="button" onClick={() => preset("gmail")}><span className="preset-dot gmail" />Gmail</button>
            <button type="button" onClick={() => preset("outlook")}><span className="preset-dot outlook" />Outlook</button>
            <button type="button" onClick={() => preset("custom")}><Server size={15} />{t("custom")}</button>
          </div>

          <div className="form-section">
            <div className="form-grid two-columns">
              <Field label={t("displayName")} value={input.displayName} onChange={(displayName) => setInput({ ...input, displayName })} required />
              <Field label={t("email")} value={input.email} onChange={(email) => setInput({ ...input, email, username: input.username || email })} type="email" required />
              <Field label={t("username")} value={input.username} onChange={(username) => setInput({ ...input, username })} required />
              <Field label={t("password")} hint={account ? t("passwordKeep") : undefined} value={input.password || ""} onChange={(password) => setInput({ ...input, password })} type="password" required={!account} />
            </div>
          </div>

          <div className="form-section server-section">
            <ServerFields
              title={t("imapServer")}
              value={input.imap}
              onChange={(imap) => setInput({ ...input, imap })}
            />
            <ServerFields
              title={t("smtpServer")}
              value={input.smtp}
              onChange={(smtp) => setInput({ ...input, smtp })}
            />
          </div>

          <div className="form-section">
            <div className="section-heading"><ShieldCheck size={17} /><h3>{t("proxy")}</h3></div>
            <div className="segmented-control proxy-kind">
              {(["direct", "http", "socks5"] as ProxyKind[]).map((kind) => (
                <button
                  key={kind}
                  type="button"
                  className={input.proxy.kind === kind ? "active" : ""}
                  onClick={() => setInput({ ...input, proxy: { ...input.proxy, kind } })}
                >
                  {t(kind)}
                </button>
              ))}
            </div>
            {input.proxy.kind !== "direct" && (
              <div className="form-grid two-columns proxy-fields">
                <Field label={t("host")} value={input.proxy.host || ""} onChange={(host) => setInput({ ...input, proxy: { ...input.proxy, host } })} required />
                <Field label={t("port")} value={String(input.proxy.port || "")} onChange={(port) => setInput({ ...input, proxy: { ...input.proxy, port: Number(port) || undefined } })} type="number" required />
                <Field label={t("proxyUsername")} value={input.proxy.username || ""} onChange={(username) => setInput({ ...input, proxy: { ...input.proxy, username } })} />
                <Field label={t("proxyPassword")} value={input.proxy.password || ""} onChange={(password) => setInput({ ...input, proxy: { ...input.proxy, password } })} type="password" />
              </div>
            )}
          </div>

          <label className="check-row">
            <input type="checkbox" checked={input.isDefault} onChange={(event) => setInput({ ...input, isDefault: event.target.checked })} />
            <span className="custom-check"><CheckCircle2 size={14} /></span>
            <span>{t("defaultAccount")}</span>
          </label>

          {message && <div className="inline-notice" aria-live="polite">{message}</div>}

          <footer className="modal-footer">
            <div>
              {account && (
                <button className="danger-button" type="button" onClick={removeAccount} disabled={Boolean(busy)}>
                  <Trash2 size={16} />{t("delete")}
                </button>
              )}
            </div>
            <div className="footer-actions">
              <button className="secondary-button" type="button" onClick={testConnection} disabled={Boolean(busy)}>
                {busy === "test" && <span className="spinner spinner-small" />}
                {busy === "test" ? t("testing") : t("testConnection")}
              </button>
              <button className="primary-button" type="submit" disabled={Boolean(busy)}>
                {busy === "save" && <span className="spinner spinner-small" />}
                {busy === "save" ? t("saving") : t("save")}
              </button>
            </div>
          </footer>
        </form>
      </section>
    </div>
  )
}

function Field({ label, hint, value, onChange, type = "text", required = false }: {
  label: string
  hint?: string
  value: string
  onChange: (value: string) => void
  type?: string
  required?: boolean
}) {
  return (
    <label className="form-field">
      <span>{label}</span>
      <input type={type} value={value} onChange={(event) => onChange(event.target.value)} required={required} />
      {hint && <small>{hint}</small>}
    </label>
  )
}

function ServerFields({ title, value, onChange }: {
  title: string
  value: AccountInput["imap"]
  onChange: (value: AccountInput["imap"]) => void
}) {
  const { t } = useI18n()
  return (
    <div className="server-card">
      <div className="section-heading"><Server size={17} /><h3>{title}</h3></div>
      <Field label={t("host")} value={value.host} onChange={(host) => onChange({ ...value, host })} required />
      <div className="server-row">
        <Field label={t("port")} value={String(value.port)} onChange={(port) => onChange({ ...value, port: Number(port) || 0 })} type="number" required />
        <label className="form-field">
          <span>{t("security")}</span>
          <div className="select-shell">
            <select value={value.security} onChange={(event) => onChange({ ...value, security: event.target.value as ConnectionSecurity })}>
              <option value="tls">{t("tls")}</option>
              <option value="starttls">{t("starttls")}</option>
            </select>
            <ChevronDown size={15} />
          </div>
        </label>
      </div>
    </div>
  )
}

function normalize(input: AccountInput, editing: boolean): AccountInput {
  const next: AccountInput = structuredClone(input)
  if (editing && !next.password) delete next.password
  if (!next.proxy.password) delete next.proxy.password
  if (!next.proxy.username) delete next.proxy.username
  if (next.proxy.kind === "direct") next.proxy = { kind: "direct" }
  return next
}
