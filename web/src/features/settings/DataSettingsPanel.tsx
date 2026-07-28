import { Banner } from "@astryxdesign/core/Banner"
import { Button } from "@astryxdesign/core/Button"
import { CheckboxInput } from "@astryxdesign/core/CheckboxInput"
import { FileInput } from "@astryxdesign/core/FileInput"
import { SegmentedControl, SegmentedControlItem } from "@astryxdesign/core/SegmentedControl"
import { TextInput } from "@astryxdesign/core/TextInput"
import { Archive, Download, ShieldCheck, Upload } from "lucide-react"
import { useState } from "react"

import { api } from "../../app/api"
import type { MailAccount, MailPreferences, MigrationArchive, MigrationScope, MigrationSections, SessionResponse } from "../../app/types"
import { useI18n } from "../../i18n/I18nProvider"
import type { MessageKey } from "../../i18n/messages"
import { SettingsPanelHeading } from "./SettingsPanelHeading"
import type { SettingsNotice } from "./settingsTypes"

const defaultSections: MigrationSections = {
  profile: true,
  mailAccounts: true,
  notifications: true,
  cleanup: true,
  preferences: true,
  ai: true,
  calendar: true,
}

export function DataSettingsPanel({ session, onSessionChanged, onMailPreferencesChanged, onAccountsChanged, onNotice }: {
  session: SessionResponse
  onSessionChanged: (session: SessionResponse) => void
  onMailPreferencesChanged: (preferences: MailPreferences) => void
  onAccountsChanged: (accounts: MailAccount[]) => void
  onNotice: (notice: SettingsNotice) => void
}) {
  const { t } = useI18n()
  const initialSections = { ...defaultSections, ai: session.user.aiEnabled }
  const [migrationScope, setMigrationScope] = useState<MigrationScope>("mine")
  const [exportSections, setExportSections] = useState(initialSections)
  const [importSections, setImportSections] = useState(initialSections)
  const [passphrase, setPassphrase] = useState("")
  const [archive, setArchive] = useState<MigrationArchive | null>(null)
  const [archiveFile, setArchiveFile] = useState<File | null>(null)
  const [busy, setBusy] = useState<"export" | "import" | null>(null)

  async function exportConfiguration() {
    if (!passphrase) return
    setBusy("export")
    try {
      const exported = await api.exportConfiguration(passphrase, migrationScope, exportSections)
      const blob = new Blob([JSON.stringify(exported, null, 2)], { type: "application/json" })
      const link = document.createElement("a")
      link.href = URL.createObjectURL(blob)
      link.download = `meowmail-config-${migrationScope}-${new Date().toISOString().slice(0, 10)}.json`
      link.click()
      URL.revokeObjectURL(link.href)
      onNotice({ key: "exportReady" })
    } catch {
      onNotice({ key: "genericError", error: true })
    } finally {
      setBusy(null)
    }
  }

  async function chooseArchive(file: File | File[] | null) {
    const selected = file instanceof File ? file : null
    setArchiveFile(selected)
    if (!selected) {
      setArchive(null)
      return
    }
    try {
      const parsed = JSON.parse(await selected.text()) as MigrationArchive
      if (parsed.format !== "meowmail-migration"
        || parsed.version !== 1
        || !["mine", "allUsers"].includes(parsed.scope)
        || !parsed.sections
        || typeof parsed.encryptedData !== "string") throw new Error("invalid archive")
      setArchive(parsed)
      setImportSections({ ...parsed.sections, ai: session.user.aiEnabled && parsed.sections.ai })
    } catch {
      setArchive(null)
      setArchiveFile(null)
      onNotice({ key: "archiveInvalid", error: true })
    }
  }

  async function importConfiguration() {
    if (!passphrase || !archive) return
    if (archive.scope === "allUsers" && session.user.role !== "admin") {
      onNotice({ key: "adminImportRequired", error: true })
      return
    }
    setBusy("import")
    try {
      const report = await api.importConfiguration(passphrase, importSections, archive)
      const [nextSession, nextPreferences, nextAccounts] = await Promise.all([
        api.session(),
        api.mailPreferences(),
        api.accounts(),
      ])
      onSessionChanged(nextSession)
      onMailPreferencesChanged(nextPreferences)
      onAccountsChanged(nextAccounts)
      onNotice({
        key: "importComplete",
        values: {
          users: report.usersImported,
          accounts: report.accountsImported,
          rules: report.rulesImported + report.autoLabelRulesImported,
          conflicts: report.conflicts.length,
        },
      })
    } catch {
      onNotice({ key: "archiveImportFailed", error: true })
    } finally {
      setBusy(null)
    }
  }

  return (
    <div className="settings-panel-stack">
      <SettingsPanelHeading icon={<Archive />} title={t("configurationTransfer")} description={t("configurationTransferDescription")} />
      <section className="settings-data-block" aria-label={t("configurationTransfer")}>
        <div className="settings-status-row">
          <div>
            <strong>{t("exportScope")}</strong>
            <small>{migrationScope === "allUsers" ? t("allUsersExportDescription") : t("mineExportDescription")}</small>
          </div>
          {session.user.role === "admin" ? (
            <SegmentedControl value={migrationScope} onChange={(value) => setMigrationScope(value as MigrationScope)} label={t("exportScope")} size="sm">
              <SegmentedControlItem value="mine" label={t("onlyMyConfiguration")} />
              <SegmentedControlItem value="allUsers" label={t("allUsers")} />
            </SegmentedControl>
          ) : null}
        </div>
        {migrationScope === "allUsers" && <Banner status="warning" title={t("allUsers")} description={t("allUsersSensitiveNote")} icon={<ShieldCheck aria-hidden="true" />} />}
        <SectionPicker value={exportSections} onChange={setExportSections} t={t} showAi={session.user.aiEnabled} />
        <TextInput
          {...{ autoComplete: "new-password" }}
          type="password"
          label={t("archivePassphrase")}
          labelTooltip={t("archivePassphraseHint")}
          value={passphrase}
          onChange={setPassphrase}
          placeholder={t("archivePassphrasePlaceholder")}
          width="100%"
        />
        <div className="settings-transfer-actions">
          <Button label={t("exportConfiguration")} icon={<Download aria-hidden="true" />} variant="secondary" isLoading={busy === "export"} isDisabled={!passphrase || !hasSection(exportSections) || Boolean(busy)} onClick={() => void exportConfiguration()} />
          <FileInput
            label={t("chooseArchive")}
            isLabelHidden
            value={archiveFile}
            onChange={(file) => void chooseArchive(file)}
            accept="application/json,.json"
            maxSize={16 * 1024 * 1024}
            placeholder={t("chooseArchive")}
            isDisabled={Boolean(busy)}
          />
        </div>
        {archive && (
          <div className="settings-import-block">
            <div className="settings-status-row">
              <div><strong>{archiveFile?.name}</strong><small>{archive.scope === "allUsers" ? t("allUsersArchive") : t("personalArchive")}</small></div>
            </div>
            <SectionPicker value={importSections} available={archive.sections} onChange={setImportSections} t={t} showAi={session.user.aiEnabled} />
            <Button label={t("importSelectedConfiguration")} icon={<Upload aria-hidden="true" />} variant="primary" isLoading={busy === "import"} isDisabled={!passphrase || !hasSection(importSections) || Boolean(busy)} onClick={() => void importConfiguration()} />
          </div>
        )}
      </section>
    </div>
  )
}

function SectionPicker({ value, available, onChange, t, showAi }: {
  value: MigrationSections
  available?: MigrationSections
  onChange: (value: MigrationSections) => void
  t: (key: MessageKey, values?: Record<string, string | number>) => string
  showAi: boolean
}) {
  const sections: Array<[keyof MigrationSections, MessageKey, MessageKey]> = [
    ["profile", "profileAndAvatar", "profileAndAvatarDescription"],
    ["mailAccounts", "mailAccountsAndCredentials", "mailAccountsAndCredentialsDescription"],
    ["notifications", "notificationConfiguration", "notificationConfigurationDescription"],
    ["cleanup", "retentionAndCleanupRules", "retentionAndCleanupRulesDescription"],
    ["preferences", "mailPreferencesAndSignatures", "mailPreferencesAndSignaturesDescription"],
    ["ai", "aiConfiguration", "aiConfigurationDescription"],
    ["calendar", "calendarConfiguration", "calendarConfigurationDescription"],
  ]
  return (
    <div className="settings-section-picker">
      {sections.filter(([key]) => key !== "ai" || showAi).map(([key, label, description]) => {
        const enabled = available?.[key] ?? true
        return (
          <CheckboxInput
            key={key}
            label={t(label)}
            description={t(description)}
            value={value[key] && enabled}
            isDisabled={!enabled}
            onChange={(checked) => onChange({ ...value, [key]: checked })}
          />
        )
      })}
    </div>
  )
}

function hasSection(sections: MigrationSections) {
  return sections.profile || sections.mailAccounts || sections.notifications || sections.cleanup || sections.preferences || sections.ai || sections.calendar
}
