import { Button } from "@astryxdesign/core/Button"
import { EmptyState } from "@astryxdesign/core/EmptyState"
import { SegmentedControl, SegmentedControlItem } from "@astryxdesign/core/SegmentedControl"
import { Inbox, Plus, RefreshCw } from "lucide-react"

import type { EmailDraft, MailAccount, MailPreferences, MessageSummary } from "../../../app/types"
import { useI18n } from "../../../i18n/I18nProvider"
import { DraftList } from "../DraftList"
import { MessageList } from "../MessageList"
import type { MailFilter } from "./types"

const filters: MailFilter[] = ["inbox", "unread", "starred", "attachments", "drafts"]

export function MessageColumn({ accounts, activeAccount, filter, messages, drafts, selectedId, loading, syncing, deleting = false, draftBusyId, preferences, onChooseFilter, onSync, onRefreshDrafts, onAddAccount, onSelect, onToggleStar, onOpenDraft, onDeleteDraft, onSendDraft }: {
  accounts: MailAccount[]
  activeAccount: MailAccount | null
  filter: MailFilter
  messages: MessageSummary[]
  drafts: EmailDraft[]
  selectedId: string | null
  loading: boolean
  syncing: boolean
  deleting?: boolean
  draftBusyId: string | null
  preferences: MailPreferences
  onChooseFilter: (filter: MailFilter) => void
  onSync: () => void
  onRefreshDrafts: () => void
  onAddAccount: () => void
  onSelect: (message: MessageSummary) => void
  onToggleStar: (message: MessageSummary) => void
  onOpenDraft: (draft: EmailDraft) => void
  onDeleteDraft: (draft: EmailDraft) => void
  onSendDraft: (draft: EmailDraft) => void
}) {
  const { t } = useI18n()
  const isDrafts = filter === "drafts"
  const itemCount = isDrafts ? drafts.length : messages.length

  return (
    <section className="message-column" aria-labelledby="message-column-title">
      <header className="message-column-header">
        <div>
          <p>{activeAccount?.displayName || t("allAccounts")}</p>
          <h1 id="message-column-title">{t(filter)}</h1>
        </div>
        <Button
          label={isDrafts ? t("refreshDrafts") : syncing ? t("syncing") : t("sync")}
          icon={<RefreshCw className={syncing ? "rotating" : undefined} aria-hidden="true" />}
          variant="ghost"
          size="sm"
          isDisabled={!accounts.length || syncing || deleting || Boolean(draftBusyId)}
          onClick={isDrafts ? onRefreshDrafts : onSync}
        />
      </header>

      <div className="message-filter-bar">
        <SegmentedControl value={filter} onChange={(value) => onChooseFilter(value as MailFilter)} label={t("mailFilters")} size="sm">
          {filters.map((value) => <SegmentedControlItem key={value} value={value} label={t(value)} />)}
        </SegmentedControl>
        <span className="message-count" aria-label={t(isDrafts ? "draftCount" : "messageCount", { count: itemCount })}>{itemCount}</span>
      </div>

      {!accounts.length && !loading ? (
        <div className="message-column-empty">
          <EmptyState
            icon={<Inbox aria-hidden="true" />}
            title={t("noAccounts")}
            description={t("noAccountsDescription")}
            actions={<Button label={t("addFirstAccount")} icon={<Plus aria-hidden="true" />} variant="primary" onClick={onAddAccount} />}
          />
        </div>
      ) : isDrafts ? (
        <DraftList
          drafts={drafts}
          accounts={accounts}
          loading={loading}
          busyId={draftBusyId}
          onOpenDraft={onOpenDraft}
          onDeleteDraft={onDeleteDraft}
          onSendDraft={onSendDraft}
        />
      ) : (
        <MessageList
          messages={messages}
          selectedId={selectedId}
          loading={loading}
          preferences={preferences}
          onSelect={onSelect}
          onToggleStar={onToggleStar}
        />
      )}
    </section>
  )
}
