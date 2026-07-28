import { Avatar } from "@astryxdesign/core/Avatar"
import { Banner } from "@astryxdesign/core/Banner"
import { Button } from "@astryxdesign/core/Button"
import { Card } from "@astryxdesign/core/Card"
import { useState, type FormEvent } from "react"
import { KeyRound, LockKeyhole, LogOut } from "lucide-react"

import { ApiError, api } from "../../app/api"
import type { SessionResponse } from "../../app/types"
import { useI18n } from "../../i18n/I18nProvider"

export function LockScreen({ session, onUnlocked, onLoggedOut }: {
  session: SessionResponse
  onUnlocked: (session: SessionResponse) => void
  onLoggedOut: () => Promise<void>
}) {
  const [pin, setPin] = useState("")
  const [busy, setBusy] = useState<"unlock" | "logout" | null>(null)
  const [error, setError] = useState<"invalid" | "limited" | "logout" | null>(null)
  const { t } = useI18n()

  async function unlock(event: FormEvent) {
    event.preventDefault()
    if (!pin || busy) return
    setBusy("unlock")
    setError(null)
    try {
      onUnlocked(await api.unlock(pin))
    } catch (cause) {
      setError(cause instanceof ApiError && cause.status === 429 ? "limited" : "invalid")
    } finally {
      setBusy(null)
    }
  }

  async function logout() {
    if (busy) return
    setBusy("logout")
    setError(null)
    try {
      await onLoggedOut()
    } catch {
      setError("logout")
      setBusy(null)
    }
  }

  return (
    <main className="lock-page">
      <Card width="100%" maxWidth={390} padding={8} className="lock-card">
        <form className="lock-form" onSubmit={unlock}>
          <span className="lock-avatar">
            <Avatar
              size="lg"
              name={session.user.nickname}
              src={session.user.hasAvatar ? `/api/v1/users/me/avatar?v=${session.user.updatedAt}` : undefined}
            />
          </span>
          <div className="lock-heading">
            <span><LockKeyhole size={17} />{t("appLocked")}</span>
            <h1>{session.user.nickname}</h1>
            <p>{t("unlockDescription")}</p>
          </div>
          <label className="auth-field-label" htmlFor="unlock-pin">{t("pin")}</label>
          <div className={`auth-field-control ${error ? "is-invalid" : ""}`}>
            <KeyRound size={17} aria-hidden="true" />
            <input
              id="unlock-pin"
              autoFocus
              autoComplete="current-password"
              type="password"
              inputMode="numeric"
              value={pin}
              onChange={(event) => setPin(event.target.value)}
              disabled={Boolean(busy)}
              placeholder={t("unlockPinPlaceholder")}
              aria-invalid={Boolean(error)}
            />
          </div>
          {error && (
            <Banner
              status="error"
              title={error === "limited" ? t("rateLimited") : error === "logout" ? t("logoutError") : t("unlockError")}
            />
          )}
          <Button
            className="login-submit"
            type="submit"
            variant="primary"
            size="lg"
            width="100%"
            label={busy === "unlock" ? t("unlocking") : t("unlock")}
            isLoading={busy === "unlock"}
            isDisabled={!pin || Boolean(busy)}
          />
          <Button
            className="lock-logout"
            variant="ghost"
            icon={<LogOut />}
            label={t("logoutAndSignInAgain")}
            isLoading={busy === "logout"}
            isDisabled={Boolean(busy)}
            onClick={() => void logout()}
          />
        </form>
      </Card>
    </main>
  )
}
