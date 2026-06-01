import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./index.css";
import { DiskPulseI18nProvider } from "./i18n";
import { ThemeProvider } from "./hooks/useTheme";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <DiskPulseI18nProvider>
      <ThemeProvider>
        <App />
      </ThemeProvider>
    </DiskPulseI18nProvider>
  </React.StrictMode>,
);
