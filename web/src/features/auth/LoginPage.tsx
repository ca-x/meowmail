import { Banner } from "@astryxdesign/core/Banner"
import { Button } from "@astryxdesign/core/Button"
import { Card } from "@astryxdesign/core/Card"
import { IconButton } from "@astryxdesign/core/IconButton"
import { useState, type FormEvent } from "react"
import { Check, Eye, EyeOff, Globe2, KeyRound, LogIn, Moon, ShieldCheck, Sparkles, Sun, UserRound } from "lucide-react"

import { ApiError, api } from "../../app/api"
import type { AuthConfig, SessionResponse } from "../../app/types"
import { useI18n } from "../../i18n/I18nProvider"
import { AppBrand } from "../../shared/ui/AppBrand"
import { useTheme } from "../../theme/ThemeProvider"

export function LoginPage({ config, onAuthenticated }: {
  config: AuthConfig
  onAuthenticated: (session: SessionResponse) => void
}) {
  const [username, setUsername] = useState("")
  const [password, setPassword] = useState("")
  const [visible, setVisible] = useState(false)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<"invalid" | "limited" | null>(null)
  const { locale, setLocale, t } = useI18n()
  const { resolved, setMode } = useTheme()

  async function submit(event: FormEvent) {
    event.preventDefault()
    if (!username || !password || busy) return
    setBusy(true)
    setError(null)
    try {
      onAuthenticated(await api.login(username, password))
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
        <Button
          label={locale === "zh-CN" ? t("switchToEnglish") : t("switchToChinese")}
          variant="ghost"
          icon={<Globe2 />}
          onClick={() => setLocale(locale === "zh-CN" ? "en" : "zh-CN")}
        >
          {locale === "zh-CN" ? "EN" : "中文"}
        </Button>
        <IconButton
          label={resolved === "dark" ? t("switchToLight") : t("switchToDark")}
          tooltip={resolved === "dark" ? t("switchToLight") : t("switchToDark")}
          variant="ghost"
          icon={resolved === "dark" ? <Sun /> : <Moon />}
          onClick={() => setMode(resolved === "dark" ? "light" : "dark")}
        />
      </header>

      <section className="login-layout">
        <div className="login-story">
          <AppBrand variant="hero" eyebrow={t("loginEyebrow")} />
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
          <Card width="100%" maxWidth={430} padding={8} className="login-card">
            <form className="login-form" onSubmit={submit}>
              <div className="login-card-icon" aria-hidden="true"><LogIn size={22} /></div>
              <div className="login-card-heading">
                <h2>{t("loginTitle")}</h2>
                <p>{t("loginDescription")}</p>
              </div>
              {config.localEnabled && (
                <>
                  <label className="auth-field-label" htmlFor="username">{t("loginUsername")}</label>
                  <div className={`auth-field-control ${error ? "is-invalid" : ""}`}>
                    <UserRound size={17} aria-hidden="true" />
                    <input
                      id="username"
                      autoFocus
                      autoComplete="username"
                      value={username}
                      onChange={(event) => setUsername(event.target.value)}
                      placeholder={t("loginUsernamePlaceholder")}
                      aria-invalid={Boolean(error)}
                    />
                  </div>
                  <label className="auth-field-label" htmlFor="password">{t("loginPassword")}</label>
                  <div className={`auth-field-control ${error ? "is-invalid" : ""}`}>
                    <KeyRound size={17} aria-hidden="true" />
                    <input
                      id="password"
                      autoComplete="current-password"
                      type={visible ? "text" : "password"}
                      value={password}
                      onChange={(event) => setPassword(event.target.value)}
                      placeholder={t("loginPasswordPlaceholder")}
                      aria-invalid={Boolean(error)}
                    />
                    <IconButton
                      label={visible ? t("hidePassword") : t("showPassword")}
                      tooltip={visible ? t("hidePassword") : t("showPassword")}
                      size="md"
                      variant="ghost"
                      icon={visible ? <EyeOff /> : <Eye />}
                      onClick={() => setVisible((value) => !value)}
                    />
                  </div>
                  {error && (
                    <Banner
                      status="error"
                      title={error === "limited" ? t("rateLimited") : t("loginError")}
                    />
                  )}
                  <Button
                    className="login-submit"
                    type="submit"
                    variant="primary"
                    size="lg"
                    width="100%"
                    label={busy ? t("signingIn") : t("signIn")}
                    isLoading={busy}
                    isDisabled={!username || !password}
                  />
                </>
              )}
              {config.localEnabled && config.oidcEnabled && <div className="login-divider"><span>{t("or")}</span></div>}
              {config.oidcEnabled && (
                <Button
                  href="/api/v1/auth/oidc/start"
                  variant="secondary"
                  size="lg"
                  width="100%"
                  icon={<ShieldCheck />}
                  label={t("signInWithOidc")}
                />
              )}
            </form>
          </Card>
        </div>
      </section>
    </main>
  )
}
