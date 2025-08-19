import { Toaster } from 'sonner'
import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./styles.css"

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <Toaster />
    <App />
  </React.StrictMode>,
);
