use rand::RngCore;
use std::sync::Mutex;
use tauri::Manager;
use tauri_plugin_shell::process::CommandChild;

const CONFIG_FILE: &str = "arx-config.json";
const DEFAULT_PORT: u16 = 50051;

struct AppState {
    admin_key: Mutex<String>,
    root_dir: Mutex<String>,
    setup_complete: Mutex<bool>,
    port: Mutex<u16>,
    sidecar: Mutex<Option<CommandChild>>,
}

fn gen_admin_key() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn default_root_dir() -> String {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    format!("{home}/Documents/ARX Drive")
}

#[tauri::command]
fn get_server_url(state: tauri::State<AppState>) -> String {
    let port = state.port.lock().unwrap();
    format!("http://localhost:{}", *port)
}

#[tauri::command]
fn get_admin_key(state: tauri::State<AppState>) -> String {
    state.admin_key.lock().unwrap().clone()
}

#[tauri::command]
fn get_root_dir(state: tauri::State<AppState>) -> String {
    state.root_dir.lock().unwrap().clone()
}

#[tauri::command]
fn is_setup_complete(state: tauri::State<AppState>) -> bool {
    *state.setup_complete.lock().unwrap()
}

#[tauri::command]
async fn pick_folder(app: tauri::AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let folder = app
        .dialog()
        .file()
        .set_title("Choose Storage Location")
        .blocking_pick_folder();
    Ok(folder.map(|p| p.to_string()))
}

#[tauri::command]
async fn save_root_dir(
    root_dir: String,
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    if root_dir.trim().is_empty() {
        return Err("root_dir cannot be empty".to_string());
    }
    use tauri_plugin_store::StoreExt;
    let store = app.store(CONFIG_FILE).map_err(|e| e.to_string())?;
    store.set("root_dir", serde_json::json!(root_dir.clone()));
    store.save().map_err(|e| e.to_string())?;
    *state.root_dir.lock().unwrap() = root_dir;
    Ok(())
}

#[tauri::command]
async fn mark_setup_complete(
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    use tauri_plugin_store::StoreExt;
    let store = app.store(CONFIG_FILE).map_err(|e| e.to_string())?;
    store.set("setup_complete", serde_json::json!(true));
    store.save().map_err(|e| e.to_string())?;
    *state.setup_complete.lock().unwrap() = true;
    Ok(())
}

#[tauri::command]
async fn reset_setup(
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    use tauri_plugin_store::StoreExt;
    let store = app.store(CONFIG_FILE).map_err(|e| e.to_string())?;
    store.set("setup_complete", serde_json::json!(false));
    store.delete("root_dir");
    store.save().map_err(|e| e.to_string())?;
    *state.setup_complete.lock().unwrap() = false;
    Ok(())
}

fn spawn_sidecar(
    app: &tauri::AppHandle,
    root_dir: &str,
    admin_key: &str,
    port: u16,
) -> Option<CommandChild> {
    use tauri_plugin_shell::ShellExt;
    let cmd = app
        .shell()
        .sidecar("arx-grpc")
        .ok()?
        .env("ROOT_DIR", root_dir)
        .env("ARX_ADMIN_KEY", admin_key)
        .env("PORT", port.to_string())
        .env("RUST_LOG", "info");

    match cmd.spawn() {
        Ok((mut rx, child)) => {
            tauri::async_runtime::spawn(async move {
                while let Some(event) = rx.recv().await {
                    use tauri_plugin_shell::process::CommandEvent;
                    match event {
                        CommandEvent::Stdout(line) => {
                            let s = String::from_utf8_lossy(&line);
                            print!("[arx-grpc] {s}");
                        }
                        CommandEvent::Stderr(line) => {
                            let s = String::from_utf8_lossy(&line);
                            eprint!("[arx-grpc] {s}");
                        }
                        CommandEvent::Terminated(status) => {
                            println!("[arx-grpc] terminated: {status:?}");
                            break;
                        }
                        _ => {}
                    }
                }
            });
            Some(child)
        }
        Err(e) => {
            eprintln!("[arx-grpc] failed to spawn: {e}");
            None
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            use tauri_plugin_store::StoreExt;

            let store = app
                .handle()
                .store(CONFIG_FILE)
                .expect("failed to open config store");

            // Load or generate admin key
            let admin_key = match store
                .get("admin_key")
                .and_then(|v| v.as_str().map(str::to_string))
            {
                Some(k) => k,
                None => {
                    let k = gen_admin_key();
                    store.set("admin_key", serde_json::json!(k.clone()));
                    let _ = store.save();
                    k
                }
            };

            // Load or default root_dir (filter out empty strings)
            let root_dir = store
                .get("root_dir")
                .and_then(|v| {
                    v.as_str()
                        .filter(|s| !s.trim().is_empty())
                        .map(str::to_string)
                })
                .unwrap_or_else(default_root_dir);

            // Load setup_complete flag
            let setup_complete = store
                .get("setup_complete")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            // Start the gRPC sidecar, keep handle for shutdown
            let sidecar_child = spawn_sidecar(app.handle(), &root_dir, &admin_key, DEFAULT_PORT);

            app.manage(AppState {
                admin_key: Mutex::new(admin_key),
                root_dir: Mutex::new(root_dir),
                setup_complete: Mutex::new(setup_complete),
                port: Mutex::new(DEFAULT_PORT),
                sidecar: Mutex::new(sidecar_child),
            });

            Ok(())
        })
        // Kill the sidecar when the last window is destroyed
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Destroyed = event {
                if let Some(state) = window.app_handle().try_state::<AppState>() {
                    if let Ok(mut guard) = state.sidecar.lock() {
                        if let Some(child) = guard.take() {
                            let _ = child.kill();
                            println!("[arx-grpc] killed on window close");
                        }
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_server_url,
            get_admin_key,
            get_root_dir,
            is_setup_complete,
            pick_folder,
            save_root_dir,
            mark_setup_complete,
            reset_setup,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
