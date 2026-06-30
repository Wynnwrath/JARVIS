pub mod commands;
pub mod domain;
pub mod handlers;
pub mod infrastructure;

use crate::commands::agent::*;
use crate::commands::chat::*;
use crate::commands::config::*;
use crate::commands::documents::*;
use crate::commands::hardware::*;
use crate::commands::permission::*;
use crate::commands::sandbox::*;
use crate::commands::system::*;
use crate::commands::voice::*;
use crate::infrastructure::database::PermissionRepository;
use crate::infrastructure::permission_gate::AppPermissionGate;
use std::sync::Arc;
use tauri::Manager;

/// Loads `config.toml`, resolves the sandbox directory, and manages config state.
fn setup_config(
    app: &tauri::AppHandle,
) -> Result<domain::config::AppConfig, Box<dyn std::error::Error>> {
    let mut config = if let Ok(config_dir) = app.path().app_config_dir() {
        let config_path = config_dir.join("config.toml");
        domain::config::AppConfig::load_from(&config_path).unwrap_or_default()
    } else {
        domain::config::AppConfig::default()
    };

    if config.sandbox_dir == "." || config.sandbox_dir.is_empty() {
        if let Ok(data_dir) = app.path().app_data_dir() {
            let resolved = data_dir.join("sandbox");
            let _ = std::fs::create_dir_all(&resolved);
            if let Ok(canonical) = std::fs::canonicalize(&resolved) {
                config.sandbox_dir = canonical.to_string_lossy().into_owned();
            }
        }
    }

    Ok(config)
}

/// Initialises SQLite, runs migrations, and creates the connection pool.
fn setup_database(app: &tauri::AppHandle, db_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let data_dir = app.path().app_data_dir()?;
    std::fs::create_dir_all(&data_dir)?;
    let db_path = data_dir.join(db_name);
    let db_path_str = db_path.to_string_lossy().into_owned();
    infrastructure::database::run_migrations(&db_path_str)?;
    infrastructure::database::init_pool(&db_path_str);
    Ok(())
}

/// Initialises voice transcription state, gracefully degrading on failure.
fn setup_voice(
    app: &tauri::AppHandle,
    silence_threshold_rms: f32,
    silence_duration_ms: u64,
    model_path: String,
) {
    let voice_state = match handlers::voice::init_voice_state(
        silence_threshold_rms,
        silence_duration_ms,
        model_path,
    ) {
        Ok(vs) => domain::voice::ManagedVoiceState(Some(vs)),
        Err(e) => {
            tracing::warn!(error = %e, "voice initialization failed");
            domain::voice::ManagedVoiceState(None)
        }
    };
    app.manage(voice_state);
}

/// Spawns the background system telemetry worker thread.
fn setup_telemetry(app: &tauri::AppHandle) {
    let system_service = infrastructure::system::LocalSystemInfoService::new();
    app.manage(system_service);

    let app_handle = app.clone();
    std::thread::spawn(move || {
        infrastructure::system::start_telemetry_worker(app_handle);
    });
}

/// Sets up the permission subsystem and preloads preferences asynchronously.
fn setup_permissions(app: &tauri::AppHandle) {
    let pool = infrastructure::database::global_pool();
    let prefs_repo = Arc::new(PermissionRepository::new(pool));
    let gate = Arc::new(AppPermissionGate::new(Arc::clone(&prefs_repo), app.clone()));
    app.manage(gate.clone());
    app.manage(prefs_repo);

    let gate_for_preload = gate.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(e) = gate_for_preload.preload_preferences().await {
            tracing::warn!(error = %e, "failed to preload permission preferences");
        }
    });
}

/// Spawns the background agent prebuild task.
fn setup_agent_prebuild(app: &tauri::AppHandle) {
    let prebuild_app = app.clone();
    tauri::async_runtime::spawn(async move {
        crate::infrastructure::agent::prebuild_agent(prebuild_app).await;
    });
}

/// Application entry-point for the Tauri desktop app.
///
/// Sets up:
/// - Configuration loaded from `config.toml` (with defaults for missing fields).
/// - SQLite database for chat session persistence.
/// - Voice transcription worker (gracefully degrades if the model is unavailable).
/// - Background system telemetry worker that emits `"system-telemetry"` events.
/// - All Tauri command handlers listed in `invoke_handler`.
///
/// Plugins: `tauri-plugin-media`, `tauri-plugin-opener`, `tauri-plugin-dialog`.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(debug_assertions)]
    let _ = tracing_subscriber::fmt().try_init();

    tauri::Builder::default()
        .plugin(tauri_plugin_media::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(domain::chat::ChatState::default())
        .setup(|app| {
            let handle = app.handle();
            let config = setup_config(handle)?;
            let silence_threshold_rms = config.silence_threshold_rms;
            let silence_duration_ms = config.silence_duration_ms;
            let model_path = config.transcription_model_path.clone();
            let db_name = config.database_name.clone();

            app.manage(tokio::sync::RwLock::new(config));

            setup_database(handle, &db_name)?;
            setup_voice(
                handle,
                silence_threshold_rms,
                silence_duration_ms,
                model_path,
            );
            setup_telemetry(handle);
            setup_permissions(handle);
            setup_agent_prebuild(handle);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Agent
            restart_agent,
            // Config
            get_config,
            update_config,
            // Chat
            prompt,
            stream_prompt,
            count_tokens,
            create_session,
            list_sessions,
            get_history,
            rename_session,
            delete_session,
            get_chat_providers,
            set_chat_provider,
            // Voice (jarvis-transcriber, pure Rust)
            start_voice_listener,
            stop_voice_listener,
            get_voice_status,
            // System Telemetry
            get_system_info,
            // Current user
            get_current_user,
            // Hardware Controls
            get_hardware_state,
            set_system_volume,
            set_volume_muted,
            set_wifi_enabled,
            set_bluetooth_enabled,
            // Document Commands
            read_document,
            write_document,
            list_directory,
            glob_search,
            grep_search,
            // Permission System
            respond_to_permission,
            get_permission_preferences,
            set_permission_preference,
            delete_permission_preference,
            // Sandbox
            add_sandbox_root,
            remove_sandbox_root,
            list_sandbox_roots,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
