import { Button } from "@astryxdesign/core/Button"
import { EmptyState } from "@astryxdesign/core/EmptyState"
import { SegmentedControl, SegmentedControlItem } from "@astryxdesign/core/SegmentedControl"
import type { ReactNode } from "react"
import { Inbox, ListFilter, Plus, RefreshCw, Trash2, X } from "lucide-react"

import type { EmailDraft, MailAccount, MailPreferences, MessageSummary } from "../../../app/types"
import { useI18n } from "../../../i18n/I18nProvider"
import { DraftList } from "../DraftList"
import { MessageList } from "../MessageList"
import type { MailFilter } from "./types"

const filters: MailFilter[] = ["inbox", "unread", "starred", "attachments", "sent", "drafts"]

export function MessageColumn({ accounts, activeAccount, filter, searchField, query, onClearSearch, messages, drafts, selectedId, selectedMessageIds, loading, syncing, deleting = false, draftBusyId, preferences, onChooseFilter, onSync, onRefreshDrafts, onAddAccount, onSelect, onToggleStar, onToggleMessageSelection, onClearMessageSelection, onBulkDeleteMessages, onOpenDraft, onDeleteDraft, onSendDraft }: {
  accounts: MailAccount[]
  activeAccount: MailAccount | null
  filter: MailFilter
  searchField: ReactNode
  query: string
  onClearSearch: () => void
  messages: MessageSummary[]
  drafts: EmailDraft[]
  selectedId: string | null
  selectedMessageIds: Set<string>
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
  onToggleMessageSelection: (id: string, selected?: boolean) => void
  onClearMessageSelection: () => void
  onBulkDeleteMessages: () => void
  onOpenDraft: (draft: EmailDraft) => void
  onDeleteDraft: (draft: EmailDraft) => void
  onSendDraft: (draft: EmailDraft) => void
}) {
  const { t } = useI18n()
  const isDrafts = filter === "drafts"
  const itemCount = isDrafts ? drafts.length : messages.length
  const selectedCount = selectedMessageIds.size

  return (
    <section className="message-column" aria-labelledby="message-column-title">
      <header className="message-column-header">
        <div>
          <p>{activeAccount?.displayName || t("allAccounts")}</p>
          <div className="message-column-title">
            <h1 id="message-column-title">{t(filter)}</h1>
            {accounts.length > 0 && !loading && (
              <span className="message-count" aria-label={t(isDrafts ? "draftCount" : "messageCount", { count: itemCount })}>{itemCount}</span>
            )}
          </div>
        </div>
        {accounts.length > 0 && <div className="message-column-tools">
          <label className="compact-mail-filter" title={t("mailFilters")}>
            <ListFilter aria-hidden="true" />
            <select aria-label={t("mailFilters")} value={filter} onChange={(event) => onChooseFilter(event.target.value as MailFilter)}>
              {filters.map((value) => <option key={value} value={value}>{t(value)}</option>)}
            </select>
          </label>
          <Button
            label={isDrafts ? t("refreshDrafts") : syncing ? t("syncing") : t("sync")}
            icon={<RefreshCw className={syncing ? "rotating" : undefined} aria-hidden="true" />}
            variant="ghost"
            size="sm"
            isDisabled={syncing || deleting || Boolean(draftBusyId)}
            onClick={isDrafts ? onRefreshDrafts : onSync}
          />
        </div>}
      </header>

      {searchField && accounts.length > 0 && <div className="message-column-search">{searchField}</div>}
      {accounts.length > 0 && <div className="message-filter-bar">
        <SegmentedControl value={filter} onChange={(value) => onChooseFilter(value as MailFilter)} label={t("mailFilters")} size="sm">
          {filters.map((value) => <SegmentedControlItem key={value} value={value} label={t(value)} />)}
        </SegmentedControl>
      </div>}

      {!isDrafts && selectedCount > 0 && (
        <div className="message-selection-bar" role="status" aria-live="polite">
          <span>{t("selectedMessageCount", { count: selectedCount })}</span>
          <span>
            <Button
              label={deleting ? t("deleting") : t("delete")}
              icon={<Trash2 aria-hidden="true" />}
              variant="destructive"
              size="sm"
              isLoading={deleting}
              isDisabled={deleting || syncing}
              onClick={onBulkDeleteMessages}
            />
            <Button
              label={t("clearSelection")}
              icon={<X aria-hidden="true" />}
              variant="ghost"
              size="sm"
              isDisabled={deleting}
              onClick={onClearMessageSelection}
            />
          </span>
        </div>
      )}

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
          query={query}
          onClearSearch={onClearSearch}
          selectedId={selectedId}
          selectedIds={selectedMessageIds}
          loading={loading}
          preferences={preferences}
          selectionDisabled={deleting}
          onSelect={onSelect}
          onToggleStar={onToggleStar}
          onToggleSelection={onToggleMessageSelection}
        />
      )}
    </section>
  )
}
