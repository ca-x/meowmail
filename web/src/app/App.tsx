import { useEffect, useState } from "react"

import { LoginPage } from "../features/auth/LoginPage"
import { MailWorkspace } from "../features/mail/MailWorkspace"
import { useI18n } from "../i18n/I18nProvider"
import { ApiError, api } from "./api"

type AuthState = { status: "loading" } | { status: "guest" } | { status: "ready" }

export function App() {
  const [auth, setAuth] = useState<AuthState>({ status: "loading" })
  const [pathname, setPathname] = useState(() => window.location.pathname)
  const { t } = useI18n()

  useEffect(() => {
    const update = () => setPathname(window.location.pathname)
    window.addEventListener("popstate", update)
    return () => window.removeEventListener("popstate", update)
  }, [])

  useEffect(() => {
    const controller = new AbortController()
    api.session().then(
      () => setAuth({ status: "ready" }),
      (error: unknown) => {
        if (!controller.signal.aborted) {
          setAuth(error instanceof ApiError && error.status === 401 ? { status: "guest" } : { status: "guest" })
        }
      },
    )
    return () => controller.abort()
  }, [])

  useEffect(() => {
    const target = auth.status === "guest"
      ? "/login"
      : auth.status === "ready" && !pathname.startsWith("/mail")
        ? "/mail/inbox"
        : null
    if (target && pathname !== target) {
      window.history.replaceState(null, "", target)
      setPathname(target)
    }
  }, [auth.status, pathname])

  if (auth.status === "loading") {
    return (
      <main className="boot-screen" aria-live="polite">
        <img src="/meowmail-logo.png" alt="" />
        <span className="spinner" aria-hidden="true" />
        <p>{t("loading")}</p>
      </main>
    )
  }

  if (auth.status === "guest") {
    return (
      <LoginPage
        onAuthenticated={() => {
          setAuth({ status: "ready" })
          window.history.replaceState(null, "", "/mail/inbox")
          setPathname("/mail/inbox")
        }}
      />
    )
  }

  return <MailWorkspace onLoggedOut={() => setAuth({ status: "guest" })} />
}
