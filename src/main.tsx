import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./index.css";

function initFontSize() {
  const stored = localStorage.getItem("ptv:font-size");
  if (stored === "compact" || stored === "normal" || stored === "large") {
    document.documentElement.setAttribute("data-font-size", stored);
  }
}

initFontSize();

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
