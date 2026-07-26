import { useState, type FormEvent } from "react"
import { Check, Eye, EyeOff, Globe2, KeyRound, Moon, ShieldCheck, Sparkles, Sun } from "lucide-react"

import { ApiError, api } from "../../app/api"
import { useI18n } from "../../i18n/I18nProvider"
import { useTheme } from "../../theme/ThemeProvider"

export function LoginPage({ onAuthenticated }: { onAuthenticated: () => void }) {
  const [pin, setPin] = useState("")
  const [visible, setVisible] = useState(false)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<"invalid" | "limited" | null>(null)
  const { locale, setLocale, t } = useI18n()
  const { resolved, setMode } = useTheme()

  async function submit(event: FormEvent) {
    event.preventDefault()
    if (!pin || busy) return
    setBusy(true)
    setError(null)
    try {
      await api.login(pin)
      onAuthenticated()
    } catch (cause) {
      setError(cause instanceof ApiError && cause.status === 429 ? "limited" : "invalid")
    } finally {
      setBusy(false)
    }
  }

  return (
    <main className="login-page">
      <div className="login-orb login-orb-one" aria-hidden="true" />
      <div className="login-orb login-orb-two" aria-hidden="true" />
      <header className="login-utility">
        <button
          className="utility-button"
          type="button"
          onClick={() => setLocale(locale === "zh-CN" ? "en" : "zh-CN")}
          aria-label={locale === "zh-CN" ? t("switchToEnglish") : t("switchToChinese")}
        >
          <Globe2 size={16} />
          <span>{locale === "zh-CN" ? "EN" : "中文"}</span>
        </button>
        <button
          className="utility-button icon-only"
          type="button"
          onClick={() => setMode(resolved === "dark" ? "light" : "dark")}
          aria-label={resolved === "dark" ? t("switchToLight") : t("switchToDark")}
        >
          {resolved === "dark" ? <Sun size={17} /> : <Moon size={17} />}
        </button>
      </header>

      <section className="login-layout">
        <div className="login-story">
          <div className="brand-lockup">
            <div className="login-logo-wrap">
              <img src="/meowmail-logo.png" alt={t("brandName")} />
            </div>
            <div>
              <p className="eyebrow">{t("loginEyebrow")}</p>
              <h1>{t("brandName")}</h1>
            </div>
          </div>
          <div className="login-copy">
            <h2>{t("loginTitle")}</h2>
            <p>{t("loginDescription")}</p>
          </div>
          <ul className="login-features">
            <li><ShieldCheck size={18} /><span>{t("loginFeature1")}</span></li>
            <li><Sparkles size={18} /><span>{t("loginFeature2")}</span></li>
            <li><Check size={18} /><span>{t("loginFeature3")}</span></li>
          </ul>
        </div>

        <div className="login-card-wrap">
          <form className="login-card" onSubmit={submit}>
            <div className="login-card-icon" aria-hidden="true"><KeyRound size={22} /></div>
            <div className="login-card-heading">
              <h2>{t("loginTitle")}</h2>
              <p>{t("loginDescription")}</p>
            </div>
            <label className="field-label" htmlFor="pin">{t("pin")}</label>
            <div className={`input-shell ${error ? "input-error" : ""}`}>
              <KeyRound size={17} aria-hidden="true" />
              <input
                id="pin"
                autoFocus
                autoComplete="current-password"
                type={visible ? "text" : "password"}
                value={pin}
                onChange={(event) => setPin(event.target.value)}
                placeholder={t("pinPlaceholder")}
                aria-invalid={Boolean(error)}
              />
              <button
                type="button"
                className="input-action"
                onClick={() => setVisible((value) => !value)}
                aria-label={visible ? t("hidePin") : t("showPin")}
              >
                {visible ? <EyeOff size={17} /> : <Eye size={17} />}
              </button>
            </div>
            <div className="field-message" aria-live="polite">
              {error === "invalid" && t("loginError")}
              {error === "limited" && t("rateLimited")}
            </div>
            <button className="primary-button login-submit" type="submit" disabled={!pin || busy}>
              {busy && <span className="spinner spinner-small" aria-hidden="true" />}
              <span>{busy ? t("signingIn") : t("signIn")}</span>
            </button>
          </form>
        </div>
      </section>
    </main>
  )
}
