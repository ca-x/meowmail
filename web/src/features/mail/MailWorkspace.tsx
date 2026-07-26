import { useCallback, useEffect, useMemo, useState } from "react"
import {
  Bell, ChevronDown, CircleUserRound, FileText, Inbox, Languages, LogOut, MailPlus,
  Menu, Moon, Paperclip, Plus, RefreshCw, Search, Send, Settings, Star, Trash2,
} from "lucide-react"

import { api } from "../../app/api"
import { readStoredValue, removeStoredValue, writeStoredValue } from "../../app/storage"
import type { MailAccount, MessageDetail, MessageSummary } from "../../app/types"
import { AccountDialog } from "../accounts/AccountDialog"
import { SettingsDialog } from "../settings/SettingsDialog"
import { useI18n } from "../../i18n/I18nProvider"
import { useTheme } from "../../theme/ThemeProvider"
import { ComposeDialog } from "./ComposeDialog"
import { MessageDetail as DetailPane } from "./MessageDetail"
import { MessageList } from "./MessageList"

type Filter = "inbox" | "unread" | "starred" | "attachments"

export function MailWorkspace({ onLoggedOut }: { onLoggedOut: () => void }) {
  const { locale, setLocale, t } = useI18n()
  const { resolved, setMode } = useTheme()
  const [accounts, setAccounts] = useState<MailAccount[]>([])
  const [activeAccountId, setActiveAccountId] = useState<string | null>(() => readStoredValue("meowmail-account"))
  const [messages, setMessages] = useState<MessageSummary[]>([])
  const [selectedId, setSelectedId] = useState<string | null>(null)
  const [detail, setDetail] = useState<MessageDetail | null>(null)
  const [filter, setFilter] = useState<Filter>("inbox")
  const [search, setSearch] = useState("")
  const [query, setQuery] = useState("")
  const [loading, setLoading] = useState(true)
  const [detailLoading, setDetailLoading] = useState(false)
  const [syncing, setSyncing] = useState(false)
  const [composeOpen, setComposeOpen] = useState(false)
  const [settingsOpen, setSettingsOpen] = useState(false)
  const [accountDialog, setAccountDialog] = useState<MailAccount | null | undefined>(undefined)
  const [mobileView, setMobileView] = useState<"list" | "detail">("list")
  const [sidebarOpen, setSidebarOpen] = useState(false)
  const [toast, setToast] = useState<string | null>(null)

  const activeAccount = useMemo(
    () => accounts.find((account) => account.id === activeAccountId) || null,
    [accounts, activeAccountId],
  )

  const loadAccounts = useCallback(async () => {
    const next = await api.accounts()
    setAccounts(next)
    setActiveAccountId((current) => {
      if (current && next.some((account) => account.id === current)) return current
      const fallback = next.find((account) => account.isDefault)?.id || next[0]?.id || null
      if (fallback) writeStoredValue("meowmail-account", fallback)
      return fallback
    })
  }, [])

  const loadMessages = useCallback(async () => {
    setLoading(true)
    try {
      const params = new URLSearchParams({ folder: "INBOX", limit: "120" })
      if (activeAccountId) params.set("accountId", activeAccountId)
      if (filter === "unread") params.set("unread", "true")
      if (filter === "starred") params.set("starred", "true")
      if (filter === "attachments") params.set("hasAttachment", "true")
      if (query) params.set("q", query)
      const next = await api.messages(params)
      setMessages(next)
      if (selectedId && !next.some((message) => message.id === selectedId)) {
        setSelectedId(null)
        setDetail(null)
      }
    } catch {
      showToast(t("genericError"))
    } finally {
      setLoading(false)
    }
  }, [activeAccountId, filter, query, selectedId, t])

  useEffect(() => { loadAccounts().catch(() => showToast(t("genericError"))) }, [loadAccounts, t])
  useEffect(() => { loadMessages() }, [loadMessages])
  useEffect(() => {
    const timer = window.setTimeout(() => setQuery(search.trim()), 250)
    return () => window.clearTimeout(timer)
  }, [search])

  useEffect(() => {
    function keyboard(event: KeyboardEvent) {
      const target = event.target as HTMLElement | null
      if (target?.matches("input, textarea, select, [contenteditable='true']")) return
      if (event.key.toLowerCase() === "c" && accounts.length) {
        event.preventDefault()
        setComposeOpen(true)
      }
      if (["j", "k", "ArrowDown", "ArrowUp"].includes(event.key) && messages.length) {
        event.preventDefault()
        const direction = event.key === "j" || event.key === "ArrowDown" ? 1 : -1
        const index = Math.max(0, messages.findIndex((message) => message.id === selectedId))
        const next = messages[Math.min(messages.length - 1, Math.max(0, index + direction))]
        if (next) void selectMessage(next)
      }
      if (event.key === "Escape" && mobileView === "detail") setMobileView("list")
    }
    window.addEventListener("keydown", keyboard)
    return () => window.removeEventListener("keydown", keyboard)
  })

  async function selectMessage(message: MessageSummary) {
    setSelectedId(message.id)
    setMobileView("detail")
    setDetailLoading(true)
    try {
      const loaded = await api.message(message.id)
      setDetail(loaded)
      if (!loaded.isRead) {
        const updated = await api.updateMessage(loaded.id, { isRead: true })
        setMessages((items) => items.map((item) => item.id === updated.id ? updated : item))
        setDetail((value) => value ? { ...value, isRead: true } : value)
      }
    } catch {
      showToast(t("genericError"))
    } finally {
      setDetailLoading(false)
    }
  }

  async function toggleStar(message: MessageSummary) {
    const updated = await api.updateMessage(message.id, { isStarred: !message.isStarred }).catch(() => null)
    if (!updated) return showToast(t("genericError"))
    setMessages((items) => items.map((item) => item.id === updated.id ? updated : item))
    setDetail((value) => value?.id === updated.id ? { ...value, isStarred: updated.isStarred } : value)
  }

  async function toggleDetailStar() {
    if (detail) await toggleStar(detail)
  }

  async function toggleRead() {
    if (!detail) return
    const updated = await api.updateMessage(detail.id, { isRead: !detail.isRead }).catch(() => null)
    if (!updated) return showToast(t("genericError"))
    setDetail({ ...detail, isRead: updated.isRead })
    setMessages((items) => items.map((item) => item.id === updated.id ? updated : item))
  }

  async function sync() {
    if (!accounts.length || syncing) return
    setSyncing(true)
    try {
      const targets = activeAccount ? [activeAccount] : accounts
      const results = []
      for (const account of targets) results.push(await api.syncAccount(account.id))
      const count = results.reduce((total, result) => total + result.inserted, 0)
      await Promise.all([loadAccounts(), loadMessages()])
      showToast(t("refreshed", { count }))
    } catch {
      showToast(t("genericError"))
    } finally {
      setSyncing(false)
    }
  }

  async function logout() {
    await api.logout().catch(() => undefined)
    onLoggedOut()
  }

  function chooseAccount(id: string | null) {
    setActiveAccountId(id)
    setSelectedId(null)
    setDetail(null)
    setMobileView("list")
    if (id) writeStoredValue("meowmail-account", id)
    else removeStoredValue("meowmail-account")
  }

  function showToast(message: string) {
    setToast(message)
    window.setTimeout(() => setToast((current) => current === message ? null : current), 4_000)
  }

  return (
    <main className="mail-app">
      <header className="app-topbar">
        <div className="topbar-brand">
          <button className="icon-button mobile-menu" type="button" onClick={() => setSidebarOpen((value) => !value)} aria-label="Menu"><Menu size={19} /></button>
          <img src="/meowmail-logo.png" alt="" />
          <div><strong>{t("brandName")}</strong><span>{t("brandSubtitle")}</span></div>
        </div>
        <div className="global-search">
          <Search size={17} aria-hidden="true" />
          <input value={search} onChange={(event) => setSearch(event.target.value)} placeholder={t("search")} aria-label={t("search")} />
          <kbd>⌘ K</kbd>
        </div>
        <div className="topbar-actions">
          <button className="icon-button" type="button" onClick={() => setLocale(locale === "zh-CN" ? "en" : "zh-CN")} aria-label={t("language")}><Languages size={18} /></button>
          <button className="icon-button" type="button" onClick={() => setMode(resolved === "dark" ? "light" : "dark")} aria-label={t("theme")}><Moon size={18} /></button>
          <button className="icon-button notification-button" type="button" onClick={() => setSettingsOpen(true)} aria-label={t("notifications")}><Bell size={18} /><i /></button>
          <button className="avatar-button" type="button" onClick={() => activeAccount ? setAccountDialog(activeAccount) : setAccountDialog(null)} aria-label={t("accountMenu")}>
            {activeAccount ? activeAccount.displayName.slice(0, 1).toUpperCase() : <CircleUserRound size={20} />}
          </button>
        </div>
      </header>

      <div className={`workspace ${sidebarOpen ? "sidebar-open" : ""}`} data-view={mobileView}>
        <aside className="mail-sidebar">
          <div className="account-switcher">
            <button className="current-account" type="button" onClick={() => activeAccount ? setAccountDialog(activeAccount) : setAccountDialog(null)}>
              <span className="account-avatar">{activeAccount?.displayName.slice(0, 1).toUpperCase() || "M"}</span>
              <span><strong>{activeAccount?.displayName || t("allAccounts")}</strong><small>{activeAccount?.email || `${accounts.length} ${t("accounts")}`}</small></span>
              <ChevronDown size={15} />
            </button>
          </div>
          <button className="compose-button" type="button" disabled={!accounts.length} onClick={() => setComposeOpen(true)}><MailPlus size={18} /><span>{t("compose")}</span><kbd>C</kbd></button>
          <nav className="folder-nav" aria-label="Mail folders">
            <FolderButton active={filter === "inbox"} icon={<Inbox size={17} />} label={t("inbox")} onClick={() => setFilter("inbox")} count={messages.filter((message) => !message.isRead).length} />
            <FolderButton active={filter === "starred"} icon={<Star size={17} />} label={t("starred")} onClick={() => setFilter("starred")} />
            <FolderButton active={filter === "unread"} icon={<FileText size={17} />} label={t("unread")} onClick={() => setFilter("unread")} />
            <FolderButton active={filter === "attachments"} icon={<Paperclip size={17} />} label={t("attachments")} onClick={() => setFilter("attachments")} />
            <div className="nav-divider" />
            <FolderButton icon={<Send size={17} />} label={t("sent")} disabled />
            <FolderButton icon={<Trash2 size={17} />} label={t("trash")} disabled />
          </nav>
          <div className="sidebar-accounts">
            <div className="sidebar-section-title"><span>{t("accounts")}</span><button type="button" onClick={() => setAccountDialog(null)} aria-label={t("addAccount")}><Plus size={15} /></button></div>
            <button className={!activeAccountId ? "active" : ""} type="button" onClick={() => chooseAccount(null)}><span className="mini-account all">∞</span><span>{t("allAccounts")}</span></button>
            {accounts.map((account) => (
              <button key={account.id} className={activeAccountId === account.id ? "active" : ""} type="button" onClick={() => chooseAccount(account.id)}>
                <span className="mini-account">{account.displayName.slice(0, 1).toUpperCase()}</span>
                <span>{account.displayName}<small>{account.email}</small></span>
                {account.isDefault && <i />}
              </button>
            ))}
          </div>
          <footer className="sidebar-footer">
            <button type="button" onClick={() => setSettingsOpen(true)}><Settings size={16} />{t("settings")}</button>
            <button type="button" onClick={logout}><LogOut size={16} />{t("logout")}</button>
          </footer>
        </aside>

        <section className="message-column">
          <header className="message-column-header">
            <div><p>{activeAccount?.displayName || t("allAccounts")}</p><h1>{t(filter)}</h1></div>
            <button className="sync-button" type="button" onClick={sync} disabled={!accounts.length || syncing}>
              <RefreshCw size={16} className={syncing ? "rotating" : ""} />
              <span>{syncing ? t("syncing") : t("sync")}</span>
            </button>
          </header>
          <div className="list-filter-bar">
            {(["inbox", "unread", "starred", "attachments"] as Filter[]).map((value) => (
              <button key={value} type="button" className={filter === value ? "active" : ""} onClick={() => setFilter(value)}>{t(value)}</button>
            ))}
            <span>{messages.length}</span>
          </div>
          {!accounts.length && !loading ? (
            <div className="first-account-empty">
              <div className="empty-logo"><img src="/meowmail-logo.png" alt="" /></div>
              <h2>{t("noAccounts")}</h2>
              <p>{t("noAccountsDescription")}</p>
              <button className="primary-button" type="button" onClick={() => setAccountDialog(null)}><Plus size={16} />{t("addFirstAccount")}</button>
            </div>
          ) : (
            <MessageList messages={messages} selectedId={selectedId} loading={loading} onSelect={selectMessage} onToggleStar={toggleStar} />
          )}
        </section>

        <section className="detail-column">
          <DetailPane
            message={detail}
            loading={detailLoading}
            onBack={() => setMobileView("list")}
            onToggleStar={toggleDetailStar}
            onToggleRead={toggleRead}
            onReply={() => setComposeOpen(true)}
          />
        </section>
      </div>

      {composeOpen && <ComposeDialog accounts={accounts} activeAccountId={activeAccountId} onClose={() => setComposeOpen(false)} onSent={() => { setComposeOpen(false); showToast(t("sentSuccess")) }} />}
      {settingsOpen && <SettingsDialog onClose={() => setSettingsOpen(false)} onOpenAccounts={() => { setSettingsOpen(false); setAccountDialog(activeAccount || null) }} />}
      {accountDialog !== undefined && (
        <AccountDialog
          account={accountDialog}
          onClose={() => setAccountDialog(undefined)}
          onSaved={(saved) => { setAccountDialog(undefined); void loadAccounts(); chooseAccount(saved.id); showToast(t("savedSuccess")) }}
          onDeleted={() => { setAccountDialog(undefined); void loadAccounts(); showToast(t("deletedSuccess")) }}
        />
      )}
      <div className={`toast ${toast ? "visible" : ""}`} role="status" aria-live="polite"><span>{toast}</span></div>
      {sidebarOpen && <button className="sidebar-scrim" type="button" onClick={() => setSidebarOpen(false)} aria-label={t("close")} />}
    </main>
  )
}

function FolderButton({ icon, label, active = false, count, onClick, disabled = false }: {
  icon: React.ReactNode
  label: string
  active?: boolean
  count?: number
  onClick?: () => void
  disabled?: boolean
}) {
  return <button type="button" className={active ? "active" : ""} onClick={onClick} disabled={disabled}>{icon}<span>{label}</span>{Boolean(count) && <b>{count}</b>}</button>
}
