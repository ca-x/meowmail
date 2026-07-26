;(() => {
  try {
    const preference = localStorage.getItem("meowmail-theme") || "system"
    const dark = preference === "dark" || (preference === "system" && matchMedia("(prefers-color-scheme: dark)").matches)
    document.documentElement.dataset.theme = dark ? "dark" : "light"
    document.documentElement.dataset.themePreference = preference
  } catch {
    document.documentElement.dataset.theme = "light"
  }
})()
