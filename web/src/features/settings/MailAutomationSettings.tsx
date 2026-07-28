import { Button } from "@astryxdesign/core/Button"
import { NumberInput } from "@astryxdesign/core/NumberInput"
import { SegmentedControl, SegmentedControlItem } from "@astryxdesign/core/SegmentedControl"
import { Switch } from "@astryxdesign/core/Switch"
import { Filter, Save } from "lucide-react"
import { useEffect, useState, type FormEvent } from "react"

import { api } from "../../app/api"
import type { CleanupRule, MailAccount, MailSettings } from "../../app/types"
import { useI18n } from "../../i18n/I18nProvider"
import { ReceiveRulesEditor } from "./ReceiveRulesEditor"
import { SettingsPanelHeading } from "./SettingsPanelHeading"
import type { SettingsNotice } from "./settingsTypes"

const defaults: MailSettings = {
  keepLocalAfterServerDelete: true,
  syncFetchLimit: 50,
}

export function MailAutomationSettings({ accounts, onNotice }: {
  accounts: MailAccount[]
  onNotice: (notice: SettingsNotice) => void
}) {
  const { t } = useI18n()
  const [settings, setSettings] = useState<MailSettings>(defaults)
  const [mode, setMode] = useState<"all" | "limited">("limited")
  const [limit, setLimit] = useState<number | null>(50)
  const [rules, setRules] = useState<CleanupRule[]>([])
  const [busy, setBusy] = useState<"retention" | "sync" | null>(null)

  useEffect(() => {
    Promise.all([api.mailSettings(), api.cleanupRules()])
      .then(([nextSettings, nextRules]) => {
        setSettings(nextSettings)
        setMode(nextSettings.syncFetchLimit === null ? "all" : "limited")
        setLimit(nextSettings.syncFetchLimit ?? 50)
        setRules(nextRules)
      })
      .catch(() => onNotice({ key: "genericError", error: true }))
  }, [onNotice])

  async function toggleRetention(keep: boolean) {
    const previous = settings
    const next = { ...settings, keepLocalAfterServerDelete: keep }
    setSettings(next)
    setBusy("retention")
    try {
      setSettings(await api.updateMailSettings(next))
      onNotice({ key: "retentionSaved" })
    } catch {
      setSettings(previous)
      onNotice({ key: "genericError", error: true })
    } finally {
      setBusy(null)
    }
  }

  async function saveSyncFetchScope(event: FormEvent) {
    event.preventDefault()
    if (mode === "limited" && (limit === null || limit < 1 || limit > 10_000)) {
      onNotice({ key: "syncFetchInvalid", error: true })
      return
    }
    const next = { ...settings, syncFetchLimit: mode === "all" ? null : limit }
    setBusy("sync")
    try {
      const saved = await api.updateMailSettings(next)
      setSettings(saved)
      setMode(saved.syncFetchLimit === null ? "all" : "limited")
      setLimit(saved.syncFetchLimit ?? 50)
      onNotice({ key: "syncFetchSaved" })
    } catch {
      onNotice({ key: "genericError", error: true })
    } finally {
      setBusy(null)
    }
  }

  const nextLimit = mode === "all" ? null : limit
  const syncChanged = nextLimit !== settings.syncFetchLimit

  return (
    <>
      <SettingsPanelHeading icon={<Filter />} title={t("filterAndVacationReply")} description={t("filterAndVacationReplyDescription")} />
      <section className="settings-automation-block" aria-label={t("filterAndVacationReply")}>
        <Switch
          label={t("keepLocalCopies")}
          labelTooltip={t("keepLocalCopiesDescription")}
          value={settings.keepLocalAfterServerDelete}
          onChange={(checked) => void toggleRetention(checked)}
          isLoading={busy === "retention"}
          isDisabled={Boolean(busy)}
          labelPosition="start"
          labelSpacing="spread"
        />
        <form className="settings-sync-form" aria-label={t("syncFetchScope")} onSubmit={saveSyncFetchScope}>
          <div className="settings-status-row">
            <div><strong>{t("syncFetchScope")}</strong><small>{t("syncFetchScopeDescription")}</small></div>
            <SegmentedControl value={mode} onChange={(value) => setMode(value as "all" | "limited")} label={t("syncFetchMode")} size="sm">
              <SegmentedControlItem value="all" label={t("syncFetchAll")} />
              <SegmentedControlItem value="limited" label={t("syncFetchRecent")} />
            </SegmentedControl>
          </div>
          <div className="settings-sync-controls">
            {mode === "limited" ? (
              <NumberInput
                label={t("syncFetchCount")}
                labelTooltip={t("syncFetchCountDescription")}
                value={limit}
                onChange={setLimit}
                min={1}
                max={10_000}
                step={1}
                isIntegerOnly
                hasClear
                width={280}
              />
            ) : <small>{t("syncFetchAllDescription")}</small>}
            <Button label={t("save")} icon={<Save aria-hidden="true" />} variant="secondary" type="submit" isLoading={busy === "sync"} isDisabled={Boolean(busy) || !syncChanged} />
          </div>
        </form>
        <ReceiveRulesEditor rules={rules} accounts={accounts} onRulesChanged={setRules} onNotice={(key, error) => onNotice({ key, error })} />
      </section>
    </>
  )
}
