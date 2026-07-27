import { Card } from "@astryxdesign/core/Card"
import { CheckboxInput } from "@astryxdesign/core/CheckboxInput"
import { SegmentedControl, SegmentedControlItem } from "@astryxdesign/core/SegmentedControl"
import { Switch } from "@astryxdesign/core/Switch"
import { BookOpen } from "lucide-react"

import type { MailPreferences } from "../../app/types"
import { useI18n } from "../../i18n/I18nProvider"

export function MailReadingPreferences({ preferences, onChange }: {
  preferences: MailPreferences
  onChange: (preferences: MailPreferences) => void
}) {
  const { t } = useI18n()

  return (
    <section className="mail-preference-section" aria-labelledby="reading-settings-title">
      <div className="mail-preference-heading">
        <BookOpen aria-hidden="true" />
        <div><h3 id="reading-settings-title">{t("readingSettings")}</h3><p>{t("readingSettingsDescription")}</p></div>
      </div>
      <Card className="mail-preference-card" padding={4}>
        <div className="mail-preference-choice-row">
          <div><strong>{t("readingMode")}</strong><small>{t("readingModeDescription")}</small></div>
          <SegmentedControl value={preferences.readingMode} onChange={(readingMode) => onChange({ ...preferences, readingMode: readingMode as MailPreferences["readingMode"] })} label={t("readingMode")} size="sm">
            <SegmentedControlItem value="preview" label={t("previewMode")} />
            <SegmentedControlItem value="list" label={t("listMode")} />
          </SegmentedControl>
        </div>
        <div className="mail-preference-choice-row">
          <strong>{t("listDensity")}</strong>
          <SegmentedControl value={preferences.listDensity} onChange={(listDensity) => onChange({ ...preferences, listDensity: listDensity as MailPreferences["listDensity"] })} label={t("listDensity")} size="sm">
            <SegmentedControlItem value="default" label={t("densityDefault")} />
            <SegmentedControlItem value="compact" label={t("densityCompact")} />
          </SegmentedControl>
        </div>
        <Switch label={t("conversationMode")} description={t("conversationModeDescription")} value={preferences.conversationMode} onChange={(conversationMode) => onChange({ ...preferences, conversationMode })} labelPosition="start" labelSpacing="spread" />
        <Switch label={t("aggregatePromotions")} description={t("aggregatePromotionsDescription")} value={preferences.aggregatePromotions} onChange={(aggregatePromotions) => onChange({ ...preferences, aggregatePromotions })} labelPosition="start" labelSpacing="spread" />
        <div className="mail-preference-check-grid">
          <CheckboxInput label={t("showMessageSummary")} value={preferences.showSummary} onChange={(showSummary) => onChange({ ...preferences, showSummary })} />
          <CheckboxInput label={t("showMessageSize")} value={preferences.showMessageSize} onChange={(showMessageSize) => onChange({ ...preferences, showMessageSize })} />
          <CheckboxInput label={t("showAttachmentPreviewOption")} value={preferences.showAttachmentPreview} onChange={(showAttachmentPreview) => onChange({ ...preferences, showAttachmentPreview })} />
        </div>
        <div className="mail-preference-choice-row">
          <strong>{t("afterMessageAction")}</strong>
          <SegmentedControl value={preferences.afterAction} onChange={(afterAction) => onChange({ ...preferences, afterAction: afterAction as MailPreferences["afterAction"] })} label={t("afterMessageAction")} size="sm">
            <SegmentedControlItem value="nextMessage" label={t("readNextMessage")} />
            <SegmentedControlItem value="messageList" label={t("returnToMessageList")} />
          </SegmentedControl>
        </div>
        <Switch label={t("plainTextReading")} description={t("plainTextReadingDescription")} value={preferences.plainTextReading} onChange={(plainTextReading) => onChange({ ...preferences, plainTextReading })} labelPosition="start" labelSpacing="spread" />
      </Card>
    </section>
  )
}
