import { useState, type FormEvent } from "react"
import { KeyRound, LockKeyhole, LogOut } from "lucide-react"

import { ApiError, api } from "../../app/api"
import type { SessionResponse } from "../../app/types"
import { useI18n } from "../../i18n/I18nProvider"

export function LockScreen({ session, onUnlocked, onLoggedOut }: {
  session: SessionResponse
  onUnlocked: (session: SessionResponse) => void
  onLoggedOut: () => void
}) {
  const [pin, setPin] = useState("")
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<"invalid" | "limited" | null>(null)
  const { t } = useI18n()

  async function unlock(event: FormEvent) {
    event.preventDefault()
    if (!pin || busy) return
    setBusy(true)
    setError(null)
    try {
      onUnlocked(await api.unlock(pin))
    } catch (cause) {
      setError(cause instanceof ApiError && cause.status === 429 ? "limited" : "invalid")
    } finally {
      setBusy(false)
    }
  }

  return (
    <main className="lock-page">
      <form className="lock-card" onSubmit={unlock}>
        <div className="profile-avatar large">
          {session.user.hasAvatar
            ? <img src={`/api/v1/users/me/avatar?v=${session.user.updatedAt}`} alt="" />
            : session.user.nickname.slice(0, 1).toUpperCase()}
        </div>
        <div className="lock-heading">
          <span><LockKeyhole size={17} />{t("appLocked")}</span>
          <h1>{session.user.nickname}</h1>
          <p>{t("unlockDescription")}</p>
        </div>
        <label className="field-label" htmlFor="unlock-pin">{t("pin")}</label>
        <div className={`input-shell ${error ? "input-error" : ""}`}>
          <KeyRound size={17} aria-hidden="true" />
          <input
            id="unlock-pin"
            autoFocus
            autoComplete="current-password"
            type="password"
            inputMode="numeric"
            value={pin}
            onChange={(event) => setPin(event.target.value)}
            placeholder={t("unlockPinPlaceholder")}
          />
        </div>
        <div className="field-message" aria-live="polite">
          {error === "invalid" && t("unlockError")}
          {error === "limited" && t("rateLimited")}
        </div>
        <button className="primary-button login-submit" type="submit" disabled={!pin || busy}>
          {busy && <span className="spinner spinner-small" />}
          {busy ? t("unlocking") : t("unlock")}
        </button>
        <button className="quiet-button lock-logout" type="button" onClick={onLoggedOut}>
          <LogOut size={15} />{t("logout")}
        </button>
      </form>
    </main>
  )
}
