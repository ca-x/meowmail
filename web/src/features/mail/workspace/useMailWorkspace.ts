import { useToast } from "@astryxdesign/core/Toast"
import { useCallback, useEffect, useMemo, useRef, useState } from "react"

import { api } from "../../../app/api"
import { defaultMailPreferences } from "../../../app/mailPreferences"
import { readStoredValue, removeStoredValue, writeStoredValue } from "../../../app/storage"
import type { MailAccount, MailPreferences, MessageDetail, MessageSummary } from "../../../app/types"
import { useI18n } from "../../../i18n/I18nProvider"
import type { MessageKey } from "../../../i18n/messages"
import type { ComposeDraft } from "../ComposeDialog"
import type { MailFilter, MailMobileView } from "./types"

export function useMailWorkspace({ onLoggedOut }: { onLoggedOut: () => void }) {
  const { t } = useI18n()
  const showAstryxToast = useToast()
  const [accounts, setAccounts] = useState<MailAccount[]>([])
  const [activeAccountId, setActiveAccountId] = useState<string | null>(() => readStoredValue("meowmail-account"))
  const [messages, setMessages] = useState<MessageSummary[]>([])
  const [mailPreferences, setMailPreferences] = useState<MailPreferences>(defaultMailPreferences)
  const [selectedId, setSelectedId] = useState<string | null>(null)
  const [detail, setDetail] = useState<MessageDetail | null>(null)
  const [thread, setThread] = useState<MessageDetail[]>([])
  const [filter, setFilter] = useState<MailFilter>("inbox")
  const [search, setSearch] = useState("")
  const [query, setQuery] = useState("")
  const [loading, setLoading] = useState(true)
  const [detailLoading, setDetailLoading] = useState(false)
  const [syncing, setSyncing] = useState(false)
  const [deleting, setDeleting] = useState(false)
  const [composeDraft, setComposeDraft] = useState<ComposeDraft | null | undefined>(undefined)
  const [settingsOpen, setSettingsOpen] = useState(false)
  const [accountManagerOpen, setAccountManagerOpen] = useState(false)
  const [accountDialog, setAccountDialog] = useState<MailAccount | null | undefined>(undefined)
  const [mobileView, setMobileView] = useState<MailMobileView>("list")
  const [sidebarOpen, setSidebarOpen] = useState(false)
  const searchRef = useRef<HTMLInputElement>(null)
  const selectedIdRef = useRef<string | null>(null)
  const deletingRef = useRef(false)

  const activeAccount = useMemo(
    () => accounts.find((account) => account.id === activeAccountId) || null,
    [accounts, activeAccountId],
  )

  const notify = useCallback((key: MessageKey, values?: Record<string, string | number>, type: "info" | "error" = "info") => {
    showAstryxToast({
      body: t(key, values),
      type,
      uniqueID: key,
      collisionBehavior: "overwrite",
      isAutoHide: type === "info",
      autoHideDuration: 4_000,
    })
  }, [showAstryxToast, t])

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
      const currentSelection = selectedIdRef.current
      if (currentSelection && !next.some((message) => message.id === currentSelection)) {
        selectedIdRef.current = null
        setSelectedId(null)
        setDetail(null)
        setThread([])
      }
    } catch {
      notify("genericError", undefined, "error")
    } finally {
      setLoading(false)
    }
  }, [activeAccountId, filter, notify, query])

  const selectMessage = useCallback(async (message: MessageSummary) => {
    setSidebarOpen(false)
    selectedIdRef.current = message.id
    setSelectedId(message.id)
    setMobileView("detail")
    setDetailLoading(true)
    try {
      const loadedThread = mailPreferences.conversationMode
        ? await api.messageThread(message.id)
        : [await api.message(message.id)]
      const loaded = loadedThread.find((item) => item.id === message.id) || loadedThread.at(-1)
      if (!loaded) throw new Error("message is missing")
      setThread(loadedThread)
      setDetail(loaded)
      if (!loaded.isRead) {
        const updated = await api.updateMessage(loaded.id, { isRead: true })
        setMessages((items) => items.map((item) => item.id === updated.id ? updated : item))
        setDetail((value) => value ? { ...value, isRead: true } : value)
        setThread((items) => items.map((item) => item.id === updated.id ? { ...item, isRead: true } : item))
      }
    } catch {
      notify("genericError", undefined, "error")
    } finally {
      setDetailLoading(false)
    }
  }, [mailPreferences.conversationMode, notify])

  useEffect(() => { selectedIdRef.current = selectedId }, [selectedId])
  useEffect(() => { loadAccounts().catch(() => notify("genericError", undefined, "error")) }, [loadAccounts, notify])
  useEffect(() => { api.mailPreferences().then(setMailPreferences).catch(() => notify("genericError", undefined, "error")) }, [notify])
  useEffect(() => { void loadMessages() }, [loadMessages])
  useEffect(() => {
    const timer = window.setTimeout(() => setQuery(search.trim()), 250)
    return () => window.clearTimeout(timer)
  }, [search])

  useEffect(() => {
    function keyboard(event: KeyboardEvent) {
      const target = event.target as HTMLElement | null
      if (composeDraft !== undefined || settingsOpen || accountManagerOpen || accountDialog !== undefined) return
      if (event.defaultPrevented) return
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
        if (searchRef.current) {
          event.preventDefault()
          searchRef.current.focus()
          searchRef.current.select()
        }
        return
      }
      if (target?.closest("input, textarea, select, button, a, [contenteditable='true'], [role='tree'], [role='listbox'], [role='menu'], [role='tablist']")) return
      if (event.key.toLowerCase() === "c" && accounts.length) {
        event.preventDefault()
        setComposeDraft(null)
      }
      if (["j", "k", "ArrowDown", "ArrowUp"].includes(event.key) && messages.length) {
        event.preventDefault()
        const direction = event.key === "j" || event.key === "ArrowDown" ? 1 : -1
        const currentIndex = messages.findIndex((message) => message.id === selectedIdRef.current)
        const nextIndex = currentIndex < 0
          ? direction > 0 ? 0 : messages.length - 1
          : Math.min(messages.length - 1, Math.max(0, currentIndex + direction))
        const next = messages[nextIndex]
        if (next) void selectMessage(next)
      }
      if (event.key === "Escape") {
        if (sidebarOpen) setSidebarOpen(false)
        else if (mobileView === "detail") setMobileView("list")
      }
    }
    window.addEventListener("keydown", keyboard)
    return () => window.removeEventListener("keydown", keyboard)
  }, [accountDialog, accountManagerOpen, accounts.length, composeDraft, messages, mobileView, selectMessage, settingsOpen, sidebarOpen])

  const chooseAccount = useCallback((id: string | null) => {
    setActiveAccountId(id)
    selectedIdRef.current = null
    setSelectedId(null)
    setDetail(null)
    setThread([])
    setMobileView("list")
    setSidebarOpen(false)
    if (id) writeStoredValue("meowmail-account", id)
    else removeStoredValue("meowmail-account")
  }, [])

  const chooseFilter = useCallback((next: MailFilter) => {
    setFilter(next)
    setSidebarOpen(false)
    setMobileView("list")
  }, [])

  const toggleStar = useCallback(async (message: MessageSummary) => {
    const updated = await api.updateMessage(message.id, { isStarred: !message.isStarred }).catch(() => null)
    if (!updated) return notify("genericError", undefined, "error")
    setMessages((items) => items.map((item) => item.id === updated.id ? updated : item))
    setDetail((value) => value?.id === updated.id ? { ...value, isStarred: updated.isStarred } : value)
    setThread((items) => items.map((item) => item.id === updated.id ? { ...item, isStarred: updated.isStarred } : item))
  }, [notify])

  const toggleRead = useCallback(async () => {
    if (!detail) return
    const updated = await api.updateMessage(detail.id, { isRead: !detail.isRead }).catch(() => null)
    if (!updated) return notify("genericError", undefined, "error")
    setDetail({ ...detail, isRead: updated.isRead })
    setThread((items) => items.map((item) => item.id === updated.id ? { ...item, isRead: updated.isRead } : item))
    setMessages((items) => items.map((item) => item.id === updated.id ? updated : item))
  }, [detail, notify])

  const sync = useCallback(async () => {
    if (!accounts.length || syncing) return
    setSyncing(true)
    try {
      const targets = activeAccount ? [activeAccount] : accounts
      const results = await Promise.all(targets.map((account) => api.syncAccount(account.id)))
      const count = results.reduce((total, result) => total + result.inserted, 0)
      await Promise.all([loadAccounts(), loadMessages()])
      notify("refreshed", { count })
    } catch {
      notify("genericError", undefined, "error")
    } finally {
      setSyncing(false)
    }
  }, [accounts, activeAccount, loadAccounts, loadMessages, notify, syncing])

  const replyToMessage = useCallback(() => {
    if (!detail) return
    const sender = detail.senderName ? `${detail.senderName} <${detail.senderEmail}>` : detail.senderEmail
    const quoted = detail.bodyText.split("\n").map((line) => `> ${line}`).join("\n")
    setComposeDraft({
      accountId: detail.accountId,
      to: detail.replyToEmail || detail.senderEmail,
      subject: prefixedSubject(mailPreferences.subjectPrefixLanguage === "chinese" ? "回复：" : "Re:", detail.subject),
      body: mailPreferences.attachOriginalOnReply ? `\n\n${t("replyOriginalHeader", { sender })}\n${quoted}` : "",
    })
  }, [detail, mailPreferences.attachOriginalOnReply, mailPreferences.subjectPrefixLanguage, t])

  const forwardMessage = useCallback(() => {
    if (!detail) return
    const subject = detail.subject || t("noSubject")
    const sender = detail.senderName ? `${detail.senderName} <${detail.senderEmail}>` : detail.senderEmail
    setComposeDraft({
      accountId: detail.accountId,
      subject: prefixedSubject(mailPreferences.subjectPrefixLanguage === "chinese" ? "转发：" : "Fwd:", detail.subject),
      body: `\n\n---------- ${t("forwardedMessage")} ----------\n${t("sender")}: ${sender}\n${t("subject")}: ${subject}\n\n${detail.bodyText || detail.preview}`,
    })
  }, [detail, mailPreferences.subjectPrefixLanguage, t])

  const deleteMessage = useCallback(async () => {
    if (!detail || deletingRef.current) return
    deletingRef.current = true
    setDeleting(true)
    const currentIndex = messages.findIndex((message) => message.id === detail.id)
    try {
      await api.deleteMessage(detail.id)
      const nextMessages = messages.filter((message) => message.id !== detail.id)
      setMessages(nextMessages)
      notify("messageDeletedSuccess")
      if (mailPreferences.afterAction === "nextMessage" && nextMessages.length) {
        const next = nextMessages[Math.min(currentIndex, nextMessages.length - 1)]
        if (next) await selectMessage(next)
      } else {
        selectedIdRef.current = null
        setSelectedId(null)
        setDetail(null)
        setThread([])
        setMobileView("list")
      }
    } catch {
      notify("genericError", undefined, "error")
    } finally {
      deletingRef.current = false
      setDeleting(false)
    }
  }, [detail, mailPreferences.afterAction, messages, notify, selectMessage])

  const logout = useCallback(async () => {
    await api.logout().catch(() => undefined)
    onLoggedOut()
  }, [onLoggedOut])

  return {
    accounts, setAccounts, activeAccountId, activeAccount, messages, mailPreferences, setMailPreferences,
    selectedId, detail, thread, filter, search, setSearch, loading, detailLoading, syncing, deleting,
    composeDraft, setComposeDraft, settingsOpen, setSettingsOpen, accountManagerOpen, setAccountManagerOpen, accountDialog, setAccountDialog,
    mobileView, setMobileView, sidebarOpen, setSidebarOpen, searchRef, notify, loadAccounts,
    chooseAccount, chooseFilter, selectMessage, toggleStar, toggleRead, sync, replyToMessage,
    forwardMessage, deleteMessage, logout,
  }
}

function prefixedSubject(prefix: "Re:" | "Fwd:" | "回复：" | "转发：", subject: string) {
  const normalized = subject.trim()
  if (!normalized) return prefix
  const alreadyPrefixed = /^(re|fwd?|fw)\s*:/i.test(normalized) || /^(回复|转发)：/.test(normalized)
  return alreadyPrefixed ? normalized : prefix.endsWith("：") ? `${prefix}${normalized}` : `${prefix} ${normalized}`
}
