import { useToast } from "@astryxdesign/core/Toast"
import { useCallback, useEffect, useMemo, useRef, useState } from "react"

import { ApiError, api } from "../../../app/api"
import { defaultMailPreferences } from "../../../app/mailPreferences"
import { readStoredValue, removeStoredValue, writeStoredValue } from "../../../app/storage"
import type { EmailDraft, MailAccount, MailPreferences, MessageDetail, MessageSummary } from "../../../app/types"
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
  const [drafts, setDrafts] = useState<EmailDraft[]>([])
  const [mailPreferences, setMailPreferences] = useState<MailPreferences>(defaultMailPreferences)
  const [selectedId, setSelectedId] = useState<string | null>(null)
  const [selectedMessageIds, setSelectedMessageIds] = useState<Set<string>>(() => new Set())
  const [detail, setDetail] = useState<MessageDetail | null>(null)
  const [thread, setThread] = useState<MessageDetail[]>([])
  const [filter, setFilter] = useState<MailFilter>("inbox")
  const [search, setSearch] = useState("")
  const [query, setQuery] = useState("")
  const [loading, setLoading] = useState(true)
  const [detailLoading, setDetailLoading] = useState(false)
  const [syncing, setSyncing] = useState(false)
  const [refreshingAttachments, setRefreshingAttachments] = useState(false)
  const [deleting, setDeleting] = useState(false)
  const [draftBusyId, setDraftBusyId] = useState<string | null>(null)
  const [composeDraft, setComposeDraft] = useState<ComposeDraft | null | undefined>(undefined)
  const [settingsOpen, setSettingsOpen] = useState(false)
  const [contactsOpen, setContactsOpen] = useState(false)
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
    if (filter === "drafts") {
      setLoading(false)
      return
    }
    setLoading(true)
    try {
      const params = new URLSearchParams({ folder: filter === "sent" ? "Sent" : "INBOX", limit: "120" })
      if (activeAccountId) params.set("accountId", activeAccountId)
      if (filter === "unread") params.set("unread", "true")
      if (filter === "starred") params.set("starred", "true")
      if (filter === "attachments") params.set("hasAttachment", "true")
      if (query) params.set("q", query)
      const next = await api.messages(params)
      setMessages(next)
      setSelectedMessageIds((selected) => {
        if (!selected.size) return selected
        const available = new Set(next.map((message) => message.id))
        const kept = new Set([...selected].filter((id) => available.has(id)))
        return kept.size === selected.size ? selected : kept
      })
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

  const loadDrafts = useCallback(async () => {
    try {
      setDrafts(await api.drafts())
    } catch {
      notify("genericError", undefined, "error")
    }
  }, [notify])

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
  useEffect(() => { void loadDrafts() }, [loadDrafts])
  useEffect(() => {
    const timer = window.setTimeout(() => setQuery(search.trim()), 250)
    return () => window.clearTimeout(timer)
  }, [search])

  useEffect(() => {
    function keyboard(event: KeyboardEvent) {
      const target = event.target as HTMLElement | null
      if (composeDraft !== undefined || settingsOpen || contactsOpen || accountManagerOpen || accountDialog !== undefined) return
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
  }, [accountDialog, accountManagerOpen, accounts.length, composeDraft, contactsOpen, messages, mobileView, selectMessage, settingsOpen, sidebarOpen])

  const chooseAccount = useCallback((id: string | null) => {
    setActiveAccountId(id)
    selectedIdRef.current = null
    setSelectedId(null)
    setDetail(null)
    setThread([])
    setSelectedMessageIds(new Set())
    setMobileView("list")
    setSidebarOpen(false)
    if (id) writeStoredValue("meowmail-account", id)
    else removeStoredValue("meowmail-account")
  }, [])

  const chooseFilter = useCallback((next: MailFilter) => {
    setFilter(next)
    setSelectedMessageIds(new Set())
    if (next === "drafts") {
      selectedIdRef.current = null
      setSelectedId(null)
      setDetail(null)
      setThread([])
      void loadDrafts()
    }
    setSidebarOpen(false)
    setMobileView("list")
  }, [loadDrafts])

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
    if (!accounts.length || syncing || deletingRef.current) return
    setSyncing(true)
    try {
      const targets = activeAccount ? [activeAccount] : accounts
      const results = await Promise.all(targets.map((account) => api.syncAccount(account.id)))
      const count = results.reduce((total, result) => total + result.inserted, 0)
      await Promise.all([loadAccounts(), loadMessages()])
      notify(count > 0 ? "refreshed" : "noNewMail", { count })
    } catch (error) {
      notify(error instanceof ApiError && error.status === 409 ? "mailboxBusy" : "genericError", undefined, "error")
    } finally {
      setSyncing(false)
    }
  }, [accounts, activeAccount, loadAccounts, loadMessages, notify, syncing])

  const refreshAttachments = useCallback(async () => {
    if (!detail || refreshingAttachments || deletingRef.current) return
    const targetId = detail.id
    setRefreshingAttachments(true)
    try {
      const refreshed = await api.refreshMessage(targetId)
      if (selectedIdRef.current !== targetId) return
      setDetail(refreshed)
      setThread((items) => items.map((item) => item.id === refreshed.id ? refreshed : item))
      setMessages((items) => items.map((item) => item.id === refreshed.id ? refreshed : item))
      notify("attachmentMetadataRefreshed")
    } catch (error) {
      if (selectedIdRef.current === targetId) {
        notify(error instanceof ApiError && error.status === 409 ? "mailboxBusy" : "genericError", undefined, "error")
      }
    } finally {
      setRefreshingAttachments(false)
    }
  }, [detail, notify, refreshingAttachments])

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
      const deletedId = detail.id
      await api.deleteMessage(deletedId)
      const nextMessages = messages.filter((message) => message.id !== deletedId)
      setMessages((items) => items.filter((message) => message.id !== deletedId))
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

  const toggleMessageSelection = useCallback((id: string, selected?: boolean) => {
    setSelectedMessageIds((current) => {
      const next = new Set(current)
      const shouldSelect = selected ?? !next.has(id)
      if (shouldSelect) next.add(id)
      else next.delete(id)
      return next
    })
  }, [])

  const clearMessageSelection = useCallback(() => {
    setSelectedMessageIds(new Set())
  }, [])

  const bulkDeleteMessages = useCallback(async () => {
    if (!selectedMessageIds.size || deletingRef.current) return
    deletingRef.current = true
    setDeleting(true)
    const ids = [...selectedMessageIds]
    const idSet = new Set(ids)
    let deleted = 0
    try {
      for (const id of ids) {
        await api.deleteMessage(id)
        deleted += 1
      }
      setMessages((items) => items.filter((message) => !idSet.has(message.id)))
      setSelectedMessageIds(new Set())
      if (detail && idSet.has(detail.id)) {
        selectedIdRef.current = null
        setSelectedId(null)
        setDetail(null)
        setThread([])
        setMobileView("list")
      }
      notify("messagesDeletedSuccess", { count: deleted })
    } catch {
      await loadMessages()
      notify(deleted > 0 ? "messagesDeletePartial" : "genericError", { count: deleted }, "error")
    } finally {
      deletingRef.current = false
      setDeleting(false)
    }
  }, [detail, loadMessages, notify, selectedMessageIds])

  const openDraft = useCallback((draft: EmailDraft) => {
    setSidebarOpen(false)
    setComposeDraft({
      id: draft.id,
      accountId: draft.accountId,
      to: draft.to.join(", "),
      cc: draft.cc.join(", "),
      bcc: draft.bcc.join(", "),
      subject: draft.subject,
      body: draft.textBody,
      htmlBody: draft.htmlBody,
      editorDocument: draft.editorDocument,
      attachments: draft.attachments,
      signatureId: draft.signatureId,
      applySignature: draft.applySignature,
      scheduledAt: draft.scheduledAt,
    })
  }, [])

  const deleteDraft = useCallback(async (draft: EmailDraft) => {
    if (draftBusyId) return
    setDraftBusyId(draft.id)
    try {
      await api.deleteDraft(draft.id)
      setDrafts((items) => items.filter((item) => item.id !== draft.id))
      notify("draftDeleted")
    } catch {
      notify("genericError", undefined, "error")
    } finally {
      setDraftBusyId(null)
    }
  }, [draftBusyId, notify])

  const sendDraft = useCallback(async (draft: EmailDraft) => {
    if (draftBusyId) return
    setDraftBusyId(draft.id)
    try {
      await api.sendDraft(draft.id)
      setDrafts((items) => items.filter((item) => item.id !== draft.id))
      notify("sentSuccess")
    } catch {
      await loadDrafts()
      notify("genericError", undefined, "error")
    } finally {
      setDraftBusyId(null)
    }
  }, [draftBusyId, loadDrafts, notify])

  const logout = useCallback(async () => {
    try {
      await api.logout()
      onLoggedOut()
    } catch {
      notify("genericError", undefined, "error")
    }
  }, [notify, onLoggedOut])

  return {
    accounts, setAccounts, activeAccountId, activeAccount, messages, drafts, mailPreferences, setMailPreferences,
    selectedId, selectedMessageIds, detail, thread, filter, search, setSearch, loading, detailLoading, syncing, refreshingAttachments, deleting,
    draftBusyId, composeDraft, setComposeDraft, settingsOpen, setSettingsOpen, contactsOpen, setContactsOpen, accountManagerOpen, setAccountManagerOpen, accountDialog, setAccountDialog,
    mobileView, setMobileView, sidebarOpen, setSidebarOpen, searchRef, notify, loadAccounts, loadDrafts,
    chooseAccount, chooseFilter, selectMessage, toggleStar, toggleRead, sync, refreshAttachments, replyToMessage,
    forwardMessage, deleteMessage, toggleMessageSelection, clearMessageSelection, bulkDeleteMessages, openDraft, deleteDraft, sendDraft, logout,
  }
}

function prefixedSubject(prefix: "Re:" | "Fwd:" | "回复：" | "转发：", subject: string) {
  const normalized = subject.trim()
  if (!normalized) return prefix
  const alreadyPrefixed = /^(re|fwd?|fw)\s*:/i.test(normalized) || /^(回复|转发)：/.test(normalized)
  return alreadyPrefixed ? normalized : prefix.endsWith("：") ? `${prefix}${normalized}` : `${prefix} ${normalized}`
}
