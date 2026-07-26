import { createContext, useContext, useEffect, useMemo, useState, type ReactNode } from "react"

import { readStoredValue, writeStoredValue } from "../app/storage"

export type ThemeMode = "system" | "light" | "dark"

interface ThemeValue {
  mode: ThemeMode
  resolved: "light" | "dark"
  setMode: (mode: ThemeMode) => void
}

const ThemeContext = createContext<ThemeValue | null>(null)

function storedMode(): ThemeMode {
  const value = readStoredValue("meowmail-theme")
  return value === "light" || value === "dark" ? value : "system"
}

function resolve(mode: ThemeMode) {
  return mode === "system"
    ? matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light"
    : mode
}

export function ThemeProvider({ children }: { children: ReactNode }) {
  const [mode, setModeState] = useState<ThemeMode>(storedMode)
  const [resolved, setResolved] = useState<"light" | "dark">(() => resolve(mode))

  useEffect(() => {
    const media = matchMedia("(prefers-color-scheme: dark)")
    const apply = () => {
      const next = resolve(mode)
      document.documentElement.dataset.theme = next
      document.documentElement.dataset.themePreference = mode
      setResolved(next)
    }
    apply()
    media.addEventListener("change", apply)
    return () => media.removeEventListener("change", apply)
  }, [mode])

  const value = useMemo<ThemeValue>(() => ({
    mode,
    resolved,
    setMode(next) {
      writeStoredValue("meowmail-theme", next)
      setModeState(next)
    },
  }), [mode, resolved])
  return <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>
}

export function useTheme() {
  const value = useContext(ThemeContext)
  if (!value) throw new Error("useTheme must be used inside ThemeProvider")
  return value
}
