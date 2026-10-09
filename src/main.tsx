import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "@stillmvd/tauri-ship/ship.css";
import "./styles.css";

const NAV_KEYS = new Set(["Tab", "ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight", "Home", "End", "PageUp", "PageDown", "Delete", "Enter", "Escape"]);
document.addEventListener("keydown", (e) => {
  if (NAV_KEYS.has(e.key)) document.documentElement.setAttribute("data-keys", "");
}, true);
document.addEventListener("pointerdown", () => document.documentElement.removeAttribute("data-keys"), true);

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
