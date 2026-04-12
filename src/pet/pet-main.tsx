import React from "react";
import ReactDOM from "react-dom/client";
import { PetWindow } from "./PetWindow";
import "./pet.css";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <PetWindow />
  </React.StrictMode>,
);
