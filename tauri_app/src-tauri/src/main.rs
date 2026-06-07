// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod alouette_open;
mod commands;
mod events;
mod state;
mod system_manager;

use core_engine::{ProcessManager, ProcessState, ResourceMonitor};
use state::{project_root, AppState};
use std::sync::Arc;
use tauri::Manager;
use tokio::sync::Mutex;

fn main() {
    // Resolve paths relative to the project root (parent of src-tauri)
    // to keep app_data out of Tauri's file watcher scope.
    let log_dir = project_root().join("logs");
    let pm = ProcessManager::new(&log_dir);


    let process_manager = Arc::new(Mutex::new(pm));
    let resource_monitor = Arc::new(ResourceMonitor::new());

    // Setup history database connection pool with WAL mode
    let db_path = commands::agent::resolve_history_db_path();
    if let Some(parent) = db_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let manager = r2d2_sqlite::SqliteConnectionManager::file(&db_path);
    let db_pool = r2d2::Pool::new(manager).expect("Failed to create r2d2 pool");
    {
        if let Ok(conn) = db_pool.get() {
            let _: Result<String, _> = conn.query_row("PRAGMA journal_mode=WAL;", [], |row| row.get(0));
            let _ = conn.execute(
                "CREATE TABLE IF NOT EXISTS history_agen (
                    session_id TEXT PRIMARY KEY,
                    title TEXT NOT NULL,
                    created_at INTEGER NOT NULL,
                    model TEXT NOT NULL,
                    mode TEXT NOT NULL,
                    active_cwd TEXT,
                    backend_history TEXT NOT NULL,
                    frontend_history TEXT NOT NULL
                );",
                [],
            );
        }
    }

    let agent_cancel_flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let default_workspace = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let mut harness_raw = core_engine::agent_harness::AgentHarness::new(&default_workspace);
    harness_raw.cancel_flag = agent_cancel_flag.clone();
    let agent_harness = Arc::new(tokio::sync::Mutex::new(harness_raw));

    let pm_clone = process_manager.clone();
    let rm_clone = resource_monitor.clone();

    tauri::Builder::default()
        .manage(AppState {
            process_manager,
            resource_monitor,
            agent_cancel_flag,
            agent_session: Arc::new(std::sync::Mutex::new(None)),
            agent_loop_state: Arc::new(std::sync::Mutex::new(None)),
            db_pool,
            agent_harness,
        })
        .setup(move |app| {
            // Get the main webview window. Standard API in Tauri v2.
            let window = app
                .get_webview_window("main")
                .ok_or_else(|| tauri::Error::WindowNotFound)?;

            // Initialize all isolated system actions (keep alive, run in background, auto restart)
            crate::system_manager::init_system(&window);

            // Initialize System Tray (Tauri v2 API)
            let toggle = tauri::menu::MenuItem::with_id(
                app,
                "toggle",
                "Show/Hide Window",
                true,
                None::<&str>,
            )?;
            let quit = tauri::menu::MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = tauri::menu::Menu::with_items(app, &[&toggle, &quit])?;

            let icon = match app.default_window_icon() {
                Some(icon) => icon.clone(),
                None => tauri::image::Image::from_bytes(include_bytes!("../icons/32x32.png"))
                    .expect("failed to load default tray icon"),
            };

            let _tray = tauri::tray::TrayIconBuilder::new()
                .icon(icon)
                .menu(&menu)
                .on_menu_event(|app, event| {
                    if event.id == "toggle" {
                        if let Some(window) = app.get_webview_window("main") {
                            let is_visible = window.is_visible().unwrap_or(false);
                            if is_visible {
                                let _ = window.hide();
                            } else {
                                let _ = window.show();
                                let _ = window.unminimize();
                                let _ = window.set_focus();
                            }
                        }
                    } else if event.id == "quit" {
                        app.exit(0);
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::DoubleClick { .. } = event {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let is_visible = window.is_visible().unwrap_or(false);
                            if is_visible {
                                let _ = window.hide();
                            } else {
                                let _ = window.show();
                                let _ = window.unminimize();
                                let _ = window.set_focus();
                            }
                        }
                    }
                })
                .build(app)?;

            // Spawn SQLite background log persister task inside active Tokio runtime
            let pm_for_persister = pm_clone.clone();
            tauri::async_runtime::spawn(async move {
                let pm = pm_for_persister.lock().await;
                pm.spawn_log_persister();
            });

            // 0. Spawn environment preloader task (non-blocking ngầm)
            events::spawn_environment_init(pm_clone.clone());

            let window_clone = window.clone();

            // 1. Spawn Log Event Router Task
            events::spawn_log_router(pm_clone.clone(), window_clone.clone());

            // 2. Spawn Status Event Router Task
            events::spawn_status_router(pm_clone.clone(), rm_clone.clone(), window_clone.clone());

            // 3. Spawn Resource Stats Router Task with Watchdog enforcement
            events::spawn_resource_stats_router(
                rm_clone.clone(),
                pm_clone.clone(),
                window_clone.clone(),
            );

            // 4. Spawn Terminal Event Router Task
            events::spawn_terminal_router(pm_clone.clone(), window_clone.clone());

            // 5. Spawn Alouette Open log monitor task
            alouette_open::spawn_alouette_open_monitor(app.handle().clone());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::process::start_project_process,
            commands::process::stop_project_process,
            commands::process::get_projects,
            commands::process::get_project_logs,
            commands::process::get_project_state,
            commands::process::register_project,
            commands::process::deregister_project,
            commands::terminal::spawn_terminal_session,
            commands::terminal::sync_terminal_input_buf,
            commands::terminal::write_to_terminal_session,
            commands::terminal::kill_terminal_session,
            commands::terminal::check_terminal_session,
            commands::terminal::resize_terminal_session,
            commands::files::get_project_files,
            commands::files::get_all_files_and_folders,
            commands::files::read_file_content,
            commands::files::write_file_content,
            commands::files::create_file,
            commands::files::create_folder,
            commands::files::get_directory_contents,
            commands::network::check_port_status,
            commands::network::force_kill_process,
            commands::network::open_ping_window,
            commands::network::open_admin_window,
            commands::browser::open_browser_window,
            commands::network::send_http_request,
            commands::network::dns_lookup,
            commands::network::ping_host,
            commands::network::ssl_certificate_info,
            commands::network::validate_json_schema,
            commands::network::json_format_tool,
            commands::network::base64_tool,
            commands::network::generate_curl_command,
            commands::network::http_status_info,
            commands::network::hash_tool,
            commands::network::jwt_decode,
            commands::network::timestamp_convert,
            commands::network::response_diff,
            commands::network::prettify_xml,
            commands::settings::get_settings,
            commands::settings::save_settings,
            commands::settings::reset_settings,
            commands::settings::hide_or_close_window,
            commands::sqlite::get_sqlite_tables,
            commands::sqlite::get_sqlite_table_data,
            commands::sqlite::update_sqlite_cell,
            commands::sqlite::insert_sqlite_row,
            commands::sqlite::delete_sqlite_row,
            commands::sqlite::add_sqlite_column,
            commands::sandbox::load_sandbox_configs,
            commands::sandbox::save_sandbox_config,
            commands::sandbox::save_all_sandbox_configs,
            commands::sandbox::delete_sandbox_config,
            commands::sandbox::load_env_simulation_configs,
            commands::sandbox::save_env_simulation_config,
            commands::language::get_language_runtimes,
            commands::language::save_language_runtime,
            commands::language::delete_language_runtime,
            commands::language::install_proto_tool,
            commands::agent::agent_send_message,
            commands::agent::agent_approve_tool,
            commands::agent::agent_reset_session,
            commands::agent::get_custom_ai_config,
            commands::agent::save_custom_ai_config,
            commands::agent::agent_cancel,
            commands::agent::agent_status,
            commands::agent::agent_get_history,
            commands::agent::load_agent_session,
            commands::agent::save_agent_session,
            commands::agent::agent_delete_session,
            toggle_alouette_open,
            is_alouette_open_active,
            commands::git::git_get_status,
            commands::git::git_stage_file,
            commands::git::git_stage_all,
            commands::git::git_unstage_file,
            commands::git::git_unstage_all,
            commands::git::git_discard_file,
            commands::git::git_commit,
            commands::git::git_push,
            commands::git::git_pull,
            commands::git::git_get_log,
            commands::git::git_get_commit_files,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(move |app_handle, event| {
            if let tauri::RunEvent::ExitRequested { api, .. } = event {
                api.prevent_exit();

                let state = app_handle.state::<AppState>();
                let pm_clone = state.process_manager.clone();
                let rm_clone = state.resource_monitor.clone();
                let app_handle_clone = app_handle.clone();

                tauri::async_runtime::spawn(async move {
                    let mut pids: Vec<(String, u32)> = Vec::new();
                    let mut term_pids: Vec<u32> = Vec::new();
                    {
                        let mut pm = pm_clone.lock().await;
                        for (id, inst) in pm.instances.iter_mut() {
                            if let ProcessState::Running { pid } = inst.state {
                                pids.push((id.clone(), pid));
                                if let Some(stop_tx) = inst.stop_sender.take() {
                                    let _ = stop_tx.send(());
                                }
                            }
                        }
                        for (_, session) in pm.terminal_sessions.iter() {
                            term_pids.push(session.pid);
                        }
                    }

                    for (project_id, pid) in pids {
                        core_engine::terminate_process_tree(pid).await;
                        rm_clone.deregister(project_id);
                    }

                    for pid in term_pids {
                        core_engine::terminate_process_tree(pid).await;
                    }

                    app_handle_clone.exit(0);
                });
            }
        });
}

#[tauri::command]
fn toggle_alouette_open(enabled: bool) {
    alouette_open::set_alouette_open_enabled(enabled);
}

#[tauri::command]
fn is_alouette_open_active() -> bool {
    alouette_open::is_alouette_open_enabled()
}
