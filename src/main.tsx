import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./index.css";

function initFontSize() {
  const stored = localStorage.getItem("agent-scope:font-size");
  const validSizes = ["compact", "normal", "large", "xlarge"];
  if (stored && validSizes.includes(stored)) {
    document.documentElement.setAttribute("data-font-size", stored);
  }
}

initFontSize();

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
