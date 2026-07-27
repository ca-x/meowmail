import { Check } from "lucide-react"

import { useI18n } from "../../i18n/I18nProvider"
import { astryxThemeNames, type AstryxThemeName } from "../../theme/astryxThemes"
import { useTheme } from "../../theme/ThemeProvider"

export function ThemePicker({
  labels,
}: {
  labels: Record<AstryxThemeName, string>
}) {
  const { t } = useI18n()
  const { resolved, themeName, setThemeName } = useTheme()

  return (
    <fieldset className="settings-theme-grid" aria-label={t("themeStyle")}>
      <legend className="visually-hidden">{t("themeStyle")}</legend>
      {astryxThemeNames.map((name) => {
        const selected = themeName === name
        return (
          <label className="settings-theme-option" data-selected={selected ? "true" : undefined} key={name}>
            <input
              className="settings-theme-radio"
              type="radio"
              name="astryx-theme"
              value={name}
              checked={selected}
              onChange={() => setThemeName(name)}
            />
            <span
              className="settings-theme-preview"
              data-astryx-theme={name}
              data-theme={resolved}
              style={{ colorScheme: resolved }}
              aria-hidden="true"
            >
              <span className="settings-theme-preview-sidebar" />
              <span className="settings-theme-preview-content">
                <span className="settings-theme-preview-accent" />
                <span className="settings-theme-preview-line" />
                <span className="settings-theme-preview-line is-short" />
              </span>
            </span>
            <span className="settings-theme-option-label">
              <span>{labels[name]}</span>
              {selected && <Check size={14} strokeWidth={2.4} aria-hidden="true" />}
            </span>
          </label>
        )
      })}
    </fieldset>
  )
}
