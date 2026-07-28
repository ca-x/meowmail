import { AppShell } from "@astryxdesign/core/AppShell"
import { Layout, LayoutContent, LayoutPanel } from "@astryxdesign/core/Layout"
import { MobileNav } from "@astryxdesign/core/MobileNav"
import { ResizeHandle, useResizable } from "@astryxdesign/core/Resizable"
import { useEffect, useRef, useState } from "react"

import type { SessionResponse } from "../../app/types"
import { useI18n } from "../../i18n/I18nProvider"
import { AccountDialog } from "../accounts/AccountDialog"
import { AccountManagerDialog } from "../accounts/AccountManagerDialog"
import { SettingsDialog } from "../settings/SettingsDialog"
import { ComposeDialog, type ComposeWorkspaceRef } from "./ComposeDialog"
import { ContactsDialog } from "./ContactsDialog"
import { MessageDetail as DetailPane } from "./MessageDetail"
import { MailNavigation } from "./workspace/MailNavigation"
import { MailTopBar } from "./workspace/MailTopBar"
import { MessageColumn } from "./workspace/MessageColumn"
import { useMailWorkspace } from "./workspace/useMailWorkspace"
import { useImperativeConfirmDialog } from "../../shared/ui/ImperativeConfirmDialog"

export function MailWorkspace({ session, onSessionChanged, onLocked, onLoggedOut }: {
  session: SessionResponse
  onSessionChanged: (session: SessionResponse) => void
  onLocked: (session: SessionResponse) => void
  onLoggedOut: () => void
}) {
  const { t } = useI18n()
  const workspace = useMailWorkspace({ onLoggedOut })
  const deleteDraftDialog = useImperativeConfirmDialog()
  const composeRef = useRef<ComposeWorkspaceRef | null>(null)
  const composeTriggerRef = useRef<HTMLElement | null>(null)
  const viewportWidth = useViewportWidth()
  const isComposing = workspace.composeDraft !== undefined
  const navigationMax = viewportWidth < 1_400 ? 260 : 320
  const detailMax = Math.max(440, Math.min(920, viewportWidth - navigationMax - 300))
  const navigationPanel = useResizable({
    defaultSize: 248,
    minSizePx: 220,
    maxSizePx: navigationMax,
    autoSaveId: "meowmail-navigation-width",
  })
  const detailPanel = useResizable({
    defaultSize: 680,
    minSizePx: 440,
    maxSizePx: detailMax,
    autoSaveId: "meowmail-detail-width",
  })

  useEffect(() => {
    if (navigationPanel.size > navigationMax) navigationPanel.resize(navigationMax)
  }, [navigationMax, navigationPanel.resize, navigationPanel.size])

  useEffect(() => {
    if (detailPanel.size > detailMax) detailPanel.resize(detailMax)
  }, [detailMax, detailPanel.resize, detailPanel.size])

  const rememberComposeTrigger = () => {
    composeTriggerRef.current = document.activeElement instanceof HTMLElement ? document.activeElement : null
  }
  const closeCompose = (restoreFocus = true) => {
    workspace.setComposeDraft(undefined)
    if (!restoreFocus) return
    window.requestAnimationFrame(() => {
      if (composeTriggerRef.current?.isConnected) composeTriggerRef.current.focus()
    })
  }
  const leaveCompose = async (action: () => void) => {
    if (isComposing) {
      const closed = await composeRef.current?.requestClose({ restoreFocus: false })
      if (closed === false) return
    }
    action()
  }
  const openSettings = () => {
    void leaveCompose(() => {
      workspace.setSidebarOpen(false)
      workspace.setSettingsOpen(true)
    })
  }
  const openAccountDialog = (account: typeof workspace.accountDialog) => {
    void leaveCompose(() => {
      workspace.setSidebarOpen(false)
      workspace.setAccountDialog(account)
    })
  }
  const navigation = (
    <MailNavigation
      accounts={workspace.accounts}
      activeAccount={workspace.activeAccount}
      activeAccountId={workspace.activeAccountId}
      filter={workspace.filter}
      unreadCount={workspace.messages.filter((message) => !message.isRead).length}
      draftCount={workspace.drafts.length}
      onChooseAccount={(id) => void leaveCompose(() => workspace.chooseAccount(id))}
      onChooseFilter={(filter) => void leaveCompose(() => workspace.chooseFilter(filter))}
      onCompose={() => {
        if (isComposing) return
        rememberComposeTrigger()
        workspace.setSidebarOpen(false)
        workspace.setComposeDraft(null)
      }}
      onOpenContacts={() => {
        void leaveCompose(() => {
          workspace.setSidebarOpen(false)
          workspace.setContactsOpen(true)
        })
      }}
      onEditAccount={(account) => openAccountDialog(account)}
      onAddAccount={() => openAccountDialog(null)}
      onOpenSettings={openSettings}
      onLogout={() => void leaveCompose(() => void workspace.logout())}
    />
  )

  return (
    <AppShell
      className="mail-app-shell"
      variant="section"
      contentPadding={0}
      height="fill"
      topNav={
        <MailTopBar
          session={session}
          search={workspace.search}
          searchRef={workspace.searchRef}
          onSearchChange={workspace.setSearch}
          onOpenSettings={openSettings}
        />
      }
      mobileNav={{
        breakpoint: "md",
        isOpen: workspace.sidebarOpen,
        onOpenChange: workspace.setSidebarOpen,
        content: (
          <MobileNav header={t("brandName")} label={t("mailNavigation")} side="start" width={300}>
            {navigation}
          </MobileNav>
        ),
      }}
    >
      <div
        className="mail-workspace-stage"
        data-view={workspace.mobileView}
        data-mode={isComposing ? "compose" : "mail"}
        data-reading-mode={workspace.mailPreferences.readingMode}
        data-list-density={workspace.mailPreferences.listDensity}
      >
        <Layout
          className="mail-workspace-layout"
          height="fill"
          padding={0}
          start={
            <>
              <LayoutPanel
                className="mail-navigation-panel"
                padding={0}
                isScrollable={false}
                role="navigation"
                label={t("mailNavigation")}
                resizable={navigationPanel.props}
              >
                {navigation}
              </LayoutPanel>
              <ResizeHandle
                className="mail-navigation-resize"
                resizable={navigationPanel.props}
                label={t("resizeNavigation")}
                hasDivider
                isAlwaysVisible={false}
              />
            </>
          }
          content={
            isComposing ? (
              <LayoutContent className="compose-workspace-host" padding={0} isScrollable={false} label={t("compose")}>
                <ComposeDialog
                  ref={composeRef}
                  accounts={workspace.accounts}
                  activeAccountId={workspace.activeAccountId}
                  preferences={workspace.mailPreferences}
                  aiEnabled={session.user.aiEnabled}
                  draft={workspace.composeDraft ?? null}
                  onClose={closeCompose}
                  onSent={() => {
                    closeCompose()
                    void workspace.loadDrafts()
                    workspace.notify("sentSuccess")
                  }}
                  onDraftSaved={(scheduled) => {
                    void workspace.loadDrafts()
                    workspace.notify(scheduled ? "scheduledDraftSaved" : "draftSaved")
                  }}
                />
              </LayoutContent>
            ) : (
              <LayoutContent className="mail-message-content" padding={0} isScrollable={false} label={t("inbox")}>
                <MessageColumn
                  accounts={workspace.accounts}
                  activeAccount={workspace.activeAccount}
                  filter={workspace.filter}
                  messages={workspace.messages}
                  drafts={workspace.drafts}
                  selectedId={workspace.selectedId}
                  selectedMessageIds={workspace.selectedMessageIds}
                  loading={workspace.loading}
                  syncing={workspace.syncing}
                  deleting={workspace.deleting}
                  draftBusyId={workspace.draftBusyId}
                  preferences={workspace.mailPreferences}
                  onChooseFilter={workspace.chooseFilter}
                  onSync={() => void workspace.sync()}
                  onRefreshDrafts={() => void workspace.loadDrafts()}
                  onAddAccount={() => openAccountDialog(null)}
                  onSelect={(message) => void workspace.selectMessage(message)}
                  onToggleStar={(message) => void workspace.toggleStar(message)}
                  onToggleMessageSelection={workspace.toggleMessageSelection}
                  onClearMessageSelection={workspace.clearMessageSelection}
                  onBulkDeleteMessages={() => void workspace.bulkDeleteMessages()}
                  onOpenDraft={(draft) => {
                    rememberComposeTrigger()
                    workspace.openDraft(draft)
                  }}
                  onSendDraft={(draft) => void workspace.sendDraft(draft)}
                  onDeleteDraft={(draft) => {
                    void deleteDraftDialog.confirm({
                      title: t("deleteDraftTitle"),
                      description: t("deleteDraftConfirm"),
                      cancelLabel: t("cancel"),
                      actionLabel: t("delete"),
                      actionVariant: "destructive",
                    }).then((confirmed) => {
                      if (confirmed) void workspace.deleteDraft(draft)
                    })
                  }}
                />
              </LayoutContent>
            )
          }
          end={
            isComposing ? undefined : <>
              <ResizeHandle
                className="mail-detail-resize"
                resizable={detailPanel.props}
                label={t("resizeReadingPane")}
                hasDivider
                isAlwaysVisible={false}
                isReversed
              />
              <LayoutPanel
                className="mail-detail-panel"
                padding={0}
                isScrollable={false}
                role="complementary"
                label={t("messageReadingPane")}
                resizable={detailPanel.props}
              >
                <DetailPane
                  message={workspace.detail}
                  thread={workspace.thread}
                  loading={workspace.detailLoading}
                  isDeleting={workspace.deleting}
                  preferences={workspace.mailPreferences}
                  aiEnabled={session.user.aiEnabled}
                  onBack={() => workspace.setMobileView("list")}
                  onToggleStar={() => workspace.detail && void workspace.toggleStar(workspace.detail)}
                  onToggleRead={() => void workspace.toggleRead()}
                  onReply={() => {
                    rememberComposeTrigger()
                    workspace.replyToMessage()
                  }}
                  onForward={() => {
                    rememberComposeTrigger()
                    workspace.forwardMessage()
                  }}
                  onDelete={() => void workspace.deleteMessage()}
                />
              </LayoutPanel>
            </>
          }
        />
      </div>
      <ContactsDialog
        isOpen={workspace.contactsOpen}
        onClose={() => workspace.setContactsOpen(false)}
      />
      <SettingsDialog
        isOpen={workspace.settingsOpen}
        session={session}
        accounts={workspace.accounts}
        mailPreferences={workspace.mailPreferences}
        onSessionChanged={onSessionChanged}
        onMailPreferencesChanged={workspace.setMailPreferences}
        onAccountsChanged={workspace.setAccounts}
        onLocked={onLocked}
        onLoggedOut={onLoggedOut}
        onClose={() => workspace.setSettingsOpen(false)}
        onOpenAccounts={() => {
          workspace.setSettingsOpen(false)
          workspace.setAccountManagerOpen(true)
        }}
      />
      <AccountManagerDialog
        isOpen={workspace.accountManagerOpen}
        accounts={workspace.accounts}
        onClose={() => workspace.setAccountManagerOpen(false)}
        onChanged={workspace.loadAccounts}
        onNotice={workspace.notify}
      />
      <AccountDialog
        isOpen={workspace.accountDialog !== undefined}
        account={workspace.accountDialog ?? null}
        onClose={() => workspace.setAccountDialog(undefined)}
        onSaved={(saved) => {
          workspace.setAccountDialog(undefined)
          void workspace.loadAccounts()
          workspace.chooseAccount(saved.id)
          workspace.notify("savedSuccess")
        }}
        onDeleted={() => {
          workspace.setAccountDialog(undefined)
          void workspace.loadAccounts()
          workspace.notify("deletedSuccess")
        }}
      />
      {deleteDraftDialog.element}
    </AppShell>
  )
}

function useViewportWidth() {
  const [width, setWidth] = useState(() => window.innerWidth)

  useEffect(() => {
    const update = () => setWidth(window.innerWidth)
    window.addEventListener("resize", update)
    return () => window.removeEventListener("resize", update)
  }, [])

  return width
}
