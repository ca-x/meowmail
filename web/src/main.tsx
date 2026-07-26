import { StrictMode } from "react"
import { createRoot } from "react-dom/client"

import { App } from "./app/App"
import { I18nProvider } from "./i18n/I18nProvider"
import { ThemeProvider } from "./theme/ThemeProvider"
import "./theme/tokens.css"
import "./styles/app.css"

const root = document.getElementById("root")
if (!root) throw new Error("Meowmail root element is missing")

createRoot(root).render(
  <StrictMode>
    <ThemeProvider>
      <I18nProvider>
        <App />
      </I18nProvider>
    </ThemeProvider>
  </StrictMode>,
)
