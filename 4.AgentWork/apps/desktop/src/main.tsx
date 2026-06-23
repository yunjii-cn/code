import React from "react";
import ReactDOM from "react-dom/client";
import { BrowserRouter } from "react-router-dom";
import App from "./App";
import "./styles/globals.css";

(function bootstrapLang() {
  try {
    const stored = localStorage.getItem("yunji-locale");
    const nav = (navigator.language || "").toLowerCase();
    const lang = stored || (nav.startsWith("zh") ? "zh-CN" : "en-US");
    document.documentElement.lang = lang;
  } catch {
    document.documentElement.lang = "zh-CN";
  }
})();

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <BrowserRouter>
      <App />
    </BrowserRouter>
  </React.StrictMode>
);