import { createContext, useContext, useEffect, useMemo, useState, type ReactNode } from "react"

import { readStoredValue, writeStoredValue } from "../app/storage"
import { messages, type Locale, type MessageKey } from "./messages"

interface I18nValue {
  locale: Locale
  setLocale: (locale: Locale) => void
  t: (key: MessageKey, values?: Record<string, string | number>) => string
}

const I18nContext = createContext<I18nValue | null>(null)

function initialLocale(): Locale {
  const stored = readStoredValue("meowmail-locale")
  if (stored === "zh-CN" || stored === "en") return stored
  return navigator.language.toLowerCase().startsWith("zh") ? "zh-CN" : "en"
}

export function I18nProvider({ children }: { children: ReactNode }) {
  const [locale, setLocaleState] = useState<Locale>(initialLocale)

  useEffect(() => {
    document.documentElement.lang = locale
    document.title = messages[locale].brandName
    document.querySelector<HTMLMetaElement>('meta[name="description"]')?.setAttribute("content", messages[locale].metaDescription)
  }, [locale])

  const value = useMemo<I18nValue>(() => ({
    locale,
    setLocale(next) {
      writeStoredValue("meowmail-locale", next)
      setLocaleState(next)
    },
    t(key, values) {
      let text: string = messages[locale][key]
      for (const [name, value] of Object.entries(values || {})) {
        text = text.replaceAll(`{${name}}`, String(value))
      }
      return text
    },
  }), [locale])
  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>
}

export function useI18n() {
  const value = useContext(I18nContext)
  if (!value) throw new Error("useI18n must be used inside I18nProvider")
  return value
}
