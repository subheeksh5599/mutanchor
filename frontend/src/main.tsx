import React from "react";
import ReactDOM from "react-dom/client";
import "@fontsource/oswald/600.css";
import "@fontsource/instrument-serif/400-italic.css";
import "@fontsource/ibm-plex-mono/400.css";
import "@fontsource/ibm-plex-mono/500.css";
import "./index.css";
import { Grain } from "./components/Grain";
import Docs from "./pages/Docs";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <Grain />
    <Docs />
  </React.StrictMode>,
);
