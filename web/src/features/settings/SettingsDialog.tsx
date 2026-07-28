import { Button } from "@astryxdesign/core/Button"
import { Dialog, DialogHeader } from "@astryxdesign/core/Dialog"
import { Layout, LayoutContent, LayoutFooter } from "@astryxdesign/core/Layout"
import { Tab, TabList } from "@astryxdesign/core/TabList"
import { useToast } from "@astryxdesign/core/Toast"
import { Bot, CalendarDays, Database, Info, MailOpen, Settings2, ShieldCheck, SlidersHorizontal, Sparkles } from "lucide-react"
import { useCallback, useEffect, useState } from "react"

import type { MailAccount, MailPreferences, SessionResponse } from "../../app/types"
import { useI18n } from "../../i18n/I18nProvider"
import { AiSettingsPanel } from "./AiSettingsPanel"
import { AboutSettingsPanel } from "./AboutSettingsPanel"
import { AutomationSettingsPanel } from "./AutomationSettingsPanel"
import { CalendarSettingsPanel } from "./CalendarSettingsPanel"
import { DataSettingsPanel } from "./DataSettingsPanel"
import { GeneralSettingsPanel } from "./GeneralSettingsPanel"
import { MailSettingsPanel } from "./MailSettingsPanel"
import { SecuritySettingsPanel } from "./SecuritySettingsPanel"
import type { SettingsNotice, SettingsTab } from "./settingsTypes"

const tabs: Array<{ value: SettingsTab; label: "settingsGeneral" | "settingsMail" | "settingsAi" | "settingsCalendar" | "settingsAutomation" | "settingsSecurity" | "settingsData" | "settingsAbout"; icon: typeof Settings2 }> = [
  { value: "general", label: "settingsGeneral", icon: Settings2 },
  { value: "mail", label: "settingsMail", icon: MailOpen },
  { value: "ai", label: "settingsAi", icon: Sparkles },
  { value: "calendar", label: "settingsCalendar", icon: CalendarDays },
  { value: "automation", label: "settingsAutomation", icon: SlidersHorizontal },
  { value: "security", label: "settingsSecurity", icon: ShieldCheck },
  { value: "data", label: "settingsData", icon: Database },
  { value: "about", label: "settingsAbout", icon: Info },
]

export function SettingsDialog({ isOpen = true, initialTab = "general", session, accounts, mailPreferences, onSessionChanged, onMailPreferencesChanged, onAccountsChanged, onCalendarChanged = () => undefined, onLoggedOut, onClose, onOpenAccounts }: {
  isOpen?: boolean
  initialTab?: SettingsTab
  session: SessionResponse
  accounts: MailAccount[]
  mailPreferences: MailPreferences
  onSessionChanged: (session: SessionResponse) => void
  onMailPreferencesChanged: (preferences: MailPreferences) => void
  onAccountsChanged: (accounts: MailAccount[]) => void
  onCalendarChanged?: () => void
  onLoggedOut: () => void
  onClose: () => void
  onOpenAccounts: () => void
}) {
  const { t } = useI18n()
  const showToast = useToast()
  const [activeTab, setActiveTab] = useState<SettingsTab>("general")
  const [visitedTabs, setVisitedTabs] = useState<Set<SettingsTab>>(() => new Set(["general"]))
  const visibleTabs = tabs.filter(({ value }) => value !== "ai" || session.user.aiEnabled)

  useEffect(() => {
    if (!isOpen) return
    const nextTab = initialTab === "ai" && !session.user.aiEnabled ? "security" : initialTab
    setActiveTab(nextTab)
    setVisitedTabs((current) => current.has(nextTab) ? current : new Set([...current, nextTab]))
  }, [initialTab, isOpen, session.user.aiEnabled])

  useEffect(() => {
    if (session.user.aiEnabled || activeTab !== "ai") return
    setActiveTab("security")
    setVisitedTabs((current) => current.has("security") ? current : new Set([...current, "security"]))
  }, [activeTab, session.user.aiEnabled])

  const showNotice = useCallback((notice: SettingsNotice) => {
    showToast({
      body: <SettingsNoticeBody notice={notice} />,
      type: notice.error ? "error" : "info",
      uniqueID: `settings-${notice.key}`,
      collisionBehavior: "overwrite",
      isAutoHide: !notice.error,
      autoHideDuration: 4_000,
    })
  }, [showToast])

  function selectTab(tab: SettingsTab) {
    setActiveTab(tab)
    setVisitedTabs((current) => current.has(tab) ? current : new Set([...current, tab]))
  }

  return (
    <Dialog
      className="settings-dialog"
      isOpen={isOpen}
      onOpenChange={(open) => { if (!open) onClose() }}
      purpose="form"
      width={920}
      maxHeight="calc(100dvh - 24px)"
      padding={0}
      aria-label={t("settings")}
    >
      <Layout
        className="settings-dialog-layout"
        height="fill"
        padding={4}
        header={
          <div className="settings-dialog-header">
            <DialogHeader
              title={t("settings")}
              subtitle={t("brandName")}
              startContent={<span className="settings-dialog-icon"><Bot aria-hidden="true" /></span>}
              onOpenChange={(open) => { if (!open) onClose() }}
            />
            <TabList
              className="settings-tab-list"
              value={activeTab}
              onChange={(value) => selectTab(value as SettingsTab)}
              layout="hug"
              size="md"
              hasDivider
              role="tablist"
              aria-label={t("settingsCategories")}
            >
              {visibleTabs.map(({ value, label, icon: Icon }) => (
                <Tab
                  key={value}
                  id={`settings-tab-${value}`}
                  value={value}
                  label={t(label)}
                  icon={<Icon aria-hidden="true" />}
                  role="tab"
                  aria-selected={activeTab === value}
                  aria-controls={`settings-panel-${value}`}
                />
              ))}
            </TabList>
          </div>
        }
        content={
          <LayoutContent className="settings-dialog-content" padding={0} isScrollable>
            {visitedTabs.has("general") && <SettingsPanel tab="general" activeTab={activeTab}><GeneralSettingsPanel session={session} accounts={accounts} onSessionChanged={onSessionChanged} onOpenAccounts={onOpenAccounts} onNotice={showNotice} /></SettingsPanel>}
            {visitedTabs.has("mail") && <SettingsPanel tab="mail" activeTab={activeTab}><MailSettingsPanel accounts={accounts} mailPreferences={mailPreferences} onMailPreferencesChanged={onMailPreferencesChanged} onAccountsChanged={onAccountsChanged} onNotice={showNotice} /></SettingsPanel>}
            {session.user.aiEnabled && visitedTabs.has("ai") && <SettingsPanel tab="ai" activeTab={activeTab}><AiSettingsPanel accounts={accounts} onNotice={showNotice} /></SettingsPanel>}
            {visitedTabs.has("calendar") && <SettingsPanel tab="calendar" activeTab={activeTab}><CalendarSettingsPanel onNotice={showNotice} onCalendarChanged={onCalendarChanged} /></SettingsPanel>}
            {visitedTabs.has("automation") && <SettingsPanel tab="automation" activeTab={activeTab}><AutomationSettingsPanel accounts={accounts} onNotice={showNotice} /></SettingsPanel>}
            {visitedTabs.has("security") && <SettingsPanel tab="security" activeTab={activeTab}><SecuritySettingsPanel isOpen={isOpen} isActive={activeTab === "security"} session={session} onSessionChanged={onSessionChanged} onLoggedOut={onLoggedOut} onClose={onClose} onNotice={showNotice} /></SettingsPanel>}
            {visitedTabs.has("data") && <SettingsPanel tab="data" activeTab={activeTab}><DataSettingsPanel session={session} onSessionChanged={onSessionChanged} onMailPreferencesChanged={onMailPreferencesChanged} onAccountsChanged={onAccountsChanged} onNotice={showNotice} /></SettingsPanel>}
            {visitedTabs.has("about") && <SettingsPanel tab="about" activeTab={activeTab}><AboutSettingsPanel session={session} /></SettingsPanel>}
          </LayoutContent>
        }
        footer={
          <LayoutFooter className="settings-dialog-footer" padding={3} hasDivider>
            <span>{t("settingsSavedImmediately")}</span>
            <Button label={t("done")} variant="primary" size="lg" onClick={onClose} />
          </LayoutFooter>
        }
      />
    </Dialog>
  )
}

function SettingsNoticeBody({ notice }: { notice: SettingsNotice }) {
  const { t } = useI18n()
  return <>{t(notice.key, notice.values)}</>
}

function SettingsPanel({ tab, activeTab, children }: {
  tab: SettingsTab
  activeTab: SettingsTab
  children: React.ReactNode
}) {
  return (
    <section
      id={`settings-panel-${tab}`}
      className="settings-tab-panel"
      role="tabpanel"
      aria-labelledby={`settings-tab-${tab}`}
      tabIndex={0}
      hidden={activeTab !== tab}
    >
      {children}
    </section>
  )
}
