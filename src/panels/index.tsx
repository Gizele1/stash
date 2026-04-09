import React from "react";
import ReactDOM from "react-dom/client";
import { PanelRouter } from "./PanelRouter";
import "./panels.css";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <PanelRouter />
  </React.StrictMode>,
);
