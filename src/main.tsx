import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./styles/tokens.css";

function applyInitialTheme() {
  const root = document.documentElement;
  let stored: string | null = null;

  try {
    stored = window.localStorage.getItem("manim4learn.themeMode");
  } catch {
    stored = null;
  }
  const themeMode =
    stored === "light" || stored === "dark" || stored === "system"
      ? stored
      : "system";
  const systemTheme =
    typeof window.matchMedia === "function" &&
    window.matchMedia("(prefers-color-scheme: dark)").matches
      ? "dark"
      : "light";
  const resolvedTheme = themeMode === "system" ? systemTheme : themeMode;

  root.dataset.theme = resolvedTheme;
  root.dataset.themeMode = themeMode;
  root.style.colorScheme = resolvedTheme;
}

applyInitialTheme();

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
