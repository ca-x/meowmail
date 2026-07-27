;(() => {
  const availableThemes = ["neutral", "stone", "butter", "matcha", "chocolate", "gothic", "y2k"]
  let astryxTheme = "neutral"
  try {
    const storedTheme = localStorage.getItem("meowmail-astryx-theme")
    astryxTheme = availableThemes.includes(storedTheme) ? storedTheme : "neutral"
  } catch {
    astryxTheme = "neutral"
  }
  document.documentElement.dataset.astryxTheme = astryxTheme

  let preference = "system"
  try {
    preference = localStorage.getItem("meowmail-theme") || "system"
  } catch {
    preference = "system"
  }

  try {
    const dark = preference === "dark" || (preference === "system" && matchMedia("(prefers-color-scheme: dark)").matches)
    document.documentElement.dataset.theme = dark ? "dark" : "light"
    document.documentElement.dataset.themePreference = preference
  } catch {
    document.documentElement.dataset.theme = "light"
    document.documentElement.dataset.themePreference = "system"
  }

  let storedLocale = null
  try {
    storedLocale = localStorage.getItem("meowmail-locale")
  } catch {
    storedLocale = null
  }
  const locale = storedLocale === "zh-CN" || storedLocale === "en"
    ? storedLocale
    : navigator.language.toLowerCase().startsWith("zh") ? "zh-CN" : "en"
  const metadata = locale === "zh-CN"
    ? { title: "妙邮", description: "多邮件账户 Web 邮件客户端" }
    : { title: "Meowmail", description: "Multi-account Web mail client" }
  document.documentElement.lang = locale
  document.title = metadata.title
  document.querySelector('meta[name="description"]')?.setAttribute("content", metadata.description)
})()
