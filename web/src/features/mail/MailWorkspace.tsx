import { AppShell } from "@astryxdesign/core/AppShell"
import { Layout, LayoutContent, LayoutPanel } from "@astryxdesign/core/Layout"
import { MobileNav } from "@astryxdesign/core/MobileNav"
import { ResizeHandle, useResizable } from "@astryxdesign/core/Resizable"
import { useEffect, useState } from "react"

import type { SessionResponse } from "../../app/types"
import { useI18n } from "../../i18n/I18nProvider"
import { AccountDialog } from "../accounts/AccountDialog"
import { SettingsDialog } from "../settings/SettingsDialog"
import { ComposeDialog } from "./ComposeDialog"
import { MessageDetail as DetailPane } from "./MessageDetail"
import { MailNavigation } from "./workspace/MailNavigation"
import { MailTopBar } from "./workspace/MailTopBar"
import { MessageColumn } from "./workspace/MessageColumn"
import { useMailWorkspace } from "./workspace/useMailWorkspace"

export function MailWorkspace({ session, onSessionChanged, onLocked, onLoggedOut }: {
  session: SessionResponse
  onSessionChanged: (session: SessionResponse) => void
  onLocked: (session: SessionResponse) => void
  onLoggedOut: () => void
}) {
  const { t } = useI18n()
  const workspace = useMailWorkspace({ onLoggedOut })
  const viewportWidth = useViewportWidth()
  const navigationMax = viewportWidth < 1_400 ? 260 : 320
  const detailMax = Math.max(440, Math.min(920, viewportWidth - navigationMax - 340))
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

  const openSettings = () => {
    workspace.setSidebarOpen(false)
    workspace.setSettingsOpen(true)
  }
  const openAccountDialog = (account: typeof workspace.accountDialog) => {
    workspace.setSidebarOpen(false)
    workspace.setAccountDialog(account)
  }
  const navigation = (
    <MailNavigation
      accounts={workspace.accounts}
      activeAccount={workspace.activeAccount}
      activeAccountId={workspace.activeAccountId}
      filter={workspace.filter}
      unreadCount={workspace.messages.filter((message) => !message.isRead).length}
      onChooseAccount={workspace.chooseAccount}
      onChooseFilter={workspace.chooseFilter}
      onCompose={() => {
        workspace.setSidebarOpen(false)
        workspace.setComposeDraft(null)
      }}
      onEditAccount={(account) => openAccountDialog(account)}
      onAddAccount={() => openAccountDialog(null)}
      onOpenSettings={openSettings}
      onLogout={() => void workspace.logout()}
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
            <LayoutContent className="mail-message-content" padding={0} isScrollable={false} label={t("inbox")}>
              <MessageColumn
                accounts={workspace.accounts}
                activeAccount={workspace.activeAccount}
                filter={workspace.filter}
                messages={workspace.messages}
                selectedId={workspace.selectedId}
                loading={workspace.loading}
                syncing={workspace.syncing}
                preferences={workspace.mailPreferences}
                onChooseFilter={workspace.chooseFilter}
                onSync={() => void workspace.sync()}
                onAddAccount={() => openAccountDialog(null)}
                onSelect={(message) => void workspace.selectMessage(message)}
                onToggleStar={(message) => void workspace.toggleStar(message)}
              />
            </LayoutContent>
          }
          end={
            <>
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
                  preferences={workspace.mailPreferences}
                  onBack={() => workspace.setMobileView("list")}
                  onToggleStar={() => workspace.detail && void workspace.toggleStar(workspace.detail)}
                  onToggleRead={() => void workspace.toggleRead()}
                  onReply={workspace.replyToMessage}
                  onForward={workspace.forwardMessage}
                  onDelete={() => void workspace.deleteMessage()}
                />
              </LayoutPanel>
            </>
          }
        />
      </div>

      <ComposeDialog
        isOpen={workspace.composeDraft !== undefined}
        accounts={workspace.accounts}
        activeAccountId={workspace.activeAccountId}
        preferences={workspace.mailPreferences}
        draft={workspace.composeDraft === undefined ? null : workspace.composeDraft}
        onClose={() => workspace.setComposeDraft(undefined)}
        onSent={() => {
          workspace.setComposeDraft(undefined)
          workspace.notify("sentSuccess")
        }}
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
        onClose={() => workspace.setSettingsOpen(false)}
        onOpenAccounts={() => {
          workspace.setSettingsOpen(false)
          workspace.setAccountDialog(workspace.activeAccount || null)
        }}
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
