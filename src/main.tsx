import React from "react";
import ReactDOM from "react-dom/client";
import { VoiceProvider } from '@/context/VoiceContext';
import { ThemeProvider } from '@/context/ThemeContext';
import { PermissionProvider } from '@/features/permission';
import App from "./App";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <ThemeProvider>
      <VoiceProvider>
        <PermissionProvider>
          <App />
        </PermissionProvider>
      </VoiceProvider>
    </ThemeProvider>
  </React.StrictMode>
);
