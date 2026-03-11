src/
├── assets/          # JARVIS branding, icons, and UI sounds
├── components/      # Global UI (Status badges, Gauge charts, Toggle switches)
├── features/        # Feature-based logic
│   ├── automation/  # Routine builders and scheduling logic
│   ├── devices/     # Device linking and Wake on LAN logic
│   └── nodes/       # Monitoring online/offline node status
├── hooks/           # useWebSocket, useDeviceStatus, useAuth
├── layouts/         # DashboardLayout (Sidebar + System Tray style)
├── lib/             # API clients (Axios instance for REST, WS client)
├── pages/           # Dashboard, NodeSettings, AutomationFlow
├── services/        # The "Bridge" (Calls to Axum API or Tauri/Rust)
├── store/           # Global state (Active nodes, System volume, Spotify state)
├── utils/           # formatBytes, parseCommand, wakeWordHelpers
└── types/           # TS Interfaces for Nodes, Users, and MCP schemas