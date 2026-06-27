mod github;
mod installer;
mod state;

use futures_util::future::join_all;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, State, WindowEvent};
use tauri_plugin_notification::NotificationExt;

use github::{compare_versions, normalize_version, GitHub};
use state::{AppState, Settings};

#[derive(Debug, Clone, Serialize)]
pub struct AppEntry {
    name: String,
    display_name: String,
    description: String,
    full_name: String,
    html_url: String,
    latest_version: Option<String>,
    installed_version: Option<String>,
    status: String,
    asset_url: Option<String>,
    asset_name: Option<String>,
    released_at: Option<String>,
    is_new: bool,
}

pub struct Cache {
    apps: Mutex<HashMap<String, AppEntry>>,
}

// ---------- helpers ----------

fn prettify(name: &str) -> String {
    name.split('-')
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

async fn resolve_token() -> Option<String> {
    if let Ok(t) = std::env::var("GITHUB_TOKEN") {
        if !t.is_empty() {
            return Some(t);
        }
    }
    // Best effort: reuse the gh CLI session if present.
    let out = tokio::process::Command::new("gh")
        .args(["auth", "token"])
        .output()
        .await
        .ok()?;
    if out.status.success() {
        let t = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !t.is_empty() {
            return Some(t);
        }
    }
    None
}

/// Fetch repos + releases and compute the full app list. Updates known-repos + cache.
async fn compute_apps(app: &AppHandle) -> Result<Vec<AppEntry>, String> {
    let st = app.state::<AppState>();
    let cache = app.state::<Cache>();

    let (user, known, mac_installed) = {
        let d = st.data.lock().map_err(|_| "stato bloccato")?;
        (
            d.settings.username.clone(),
            d.known_repos.clone(),
            d.installed.clone(),
        )
    };
    let first_run = known.is_empty();

    let gh = GitHub::new(resolve_token().await);
    let repos = gh.list_repos(&user).await?;

    let rels = join_all(repos.iter().map(|r| gh.latest_release(&user, &r.name))).await;

    let mut entries = Vec::new();
    for (repo, rel) in repos.iter().zip(rels.into_iter()) {
        let rel = match rel {
            Ok(Some(r)) => r,
            Ok(None) => continue, // no release -> not a distributable app yet
            Err(_) => continue,
        };

        let latest = normalize_version(&rel.tag);
        let has_asset = rel.asset_url.is_some();

        // locally installed version
        let installed_version = if has_asset {
            #[cfg(target_os = "linux")]
            {
                match rel.asset_name.as_deref().and_then(github::pkgname_from_asset) {
                    Some(pkg) => installer::installed_version(&pkg).await,
                    None => None,
                }
            }
            #[cfg(not(target_os = "linux"))]
            {
                mac_installed.get(&repo.name).cloned()
            }
        } else {
            None
        };

        let status = if !has_asset {
            "unsupported"
        } else if let Some(iv) = &installed_version {
            if compare_versions(iv, &latest) == std::cmp::Ordering::Less {
                "update_available"
            } else {
                "installed"
            }
        } else {
            "not_installed"
        };

        let is_new = !first_run && !known.contains(&repo.name);

        entries.push(AppEntry {
            name: repo.name.clone(),
            display_name: prettify(&repo.name),
            description: repo.description.clone(),
            full_name: format!("{user}/{}", repo.name),
            html_url: repo.html_url.clone(),
            latest_version: Some(latest),
            installed_version,
            status: status.to_string(),
            asset_url: rel.asset_url.clone(),
            asset_name: rel.asset_name.clone(),
            released_at: rel.published_at.clone(),
            is_new,
        });
    }

    // refresh cache + persisted known list
    {
        let mut c = cache.apps.lock().map_err(|_| "cache bloccata")?;
        c.clear();
        for e in &entries {
            c.insert(e.name.clone(), e.clone());
        }
    }
    {
        let mut d = st.data.lock().map_err(|_| "stato bloccato")?;
        let mut all: Vec<String> = entries.iter().map(|e| e.name.clone()).collect();
        // keep names we knew even if a fetch hiccup dropped them this round
        for k in &known {
            if !all.contains(k) {
                all.push(k.clone());
            }
        }
        d.known_repos = all;
    }
    st.save();
    let _ = mac_installed; // silence unused on linux

    Ok(entries)
}

// ---------- commands ----------

#[tauri::command]
async fn list_apps(app: AppHandle, _force: bool) -> Result<Vec<AppEntry>, String> {
    compute_apps(&app).await
}

#[tauri::command]
fn get_settings(st: State<AppState>) -> Result<Settings, String> {
    Ok(st.data.lock().map_err(|_| "stato bloccato")?.settings.clone())
}

#[tauri::command]
fn save_settings(st: State<AppState>, settings: Settings) -> Result<(), String> {
    {
        let mut d = st.data.lock().map_err(|_| "stato bloccato")?;
        d.settings = settings;
    }
    st.save();
    Ok(())
}

#[tauri::command]
async fn install_or_update(app: AppHandle, name: String) -> Result<(), String> {
    let entry = {
        let cache = app.state::<Cache>();
        let c = cache.apps.lock().map_err(|_| "cache bloccata")?;
        c.get(&name).cloned()
    };
    let Some(entry) = entry else {
        return Err("App sconosciuta".into());
    };
    let (Some(url), Some(asset)) = (entry.asset_url.clone(), entry.asset_name.clone()) else {
        return Err("Nessun pacchetto disponibile per questa piattaforma".into());
    };

    let run = async {
        let file = installer::download(&app, &name, &url, &asset).await?;
        installer::install_package(&app, &name, &file).await?;
        Ok::<(), String>(())
    };

    match run.await {
        Ok(()) => {
            // macOS has no package db: record the version ourselves
            #[cfg(not(target_os = "linux"))]
            {
                if let Some(v) = &entry.latest_version {
                    let st = app.state::<AppState>();
                    if let Ok(mut d) = st.data.lock() {
                        d.installed.insert(name.clone(), v.clone());
                    }
                    st.save();
                }
            }
            installer::notify_done(&app, &name, "Completato");
            Ok(())
        }
        Err(e) => {
            installer::notify_error(&app, &name, &e);
            Err(e)
        }
    }
}

#[tauri::command]
async fn uninstall_app(app: AppHandle, name: String) -> Result<(), String> {
    let entry = {
        let cache = app.state::<Cache>();
        let c = cache.apps.lock().map_err(|_| "cache bloccata")?;
        c.get(&name).cloned()
    };
    let Some(entry) = entry else {
        return Err("App sconosciuta".into());
    };

    #[cfg(target_os = "linux")]
    {
        let pkg = entry
            .asset_name
            .as_deref()
            .and_then(github::pkgname_from_asset)
            .ok_or("Impossibile determinare il pacchetto")?;
        installer::uninstall(&pkg).await?;
    }
    #[cfg(not(target_os = "linux"))]
    {
        installer::uninstall(&entry.display_name).await?;
        let st = app.state::<AppState>();
        if let Ok(mut d) = st.data.lock() {
            d.installed.remove(&name);
        }
        st.save();
    }
    Ok(())
}

#[tauri::command]
async fn launch_app(app: AppHandle, name: String) -> Result<(), String> {
    let entry = {
        let cache = app.state::<Cache>();
        let c = cache.apps.lock().map_err(|_| "cache bloccata")?;
        c.get(&name).cloned()
    };
    let Some(entry) = entry else {
        return Err("App sconosciuta".into());
    };

    #[cfg(target_os = "linux")]
    let id = entry
        .asset_name
        .as_deref()
        .and_then(github::pkgname_from_asset)
        .unwrap_or_else(|| name.clone());
    #[cfg(not(target_os = "linux"))]
    let id = entry.display_name.clone();

    installer::launch(&id).await
}

// ---------- tray + lifecycle ----------

fn toggle_window(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        match w.is_visible() {
            Ok(true) => {
                let _ = w.hide();
            }
            _ => {
                let _ = w.show();
                let _ = w.unminimize();
                let _ = w.set_focus();
            }
        }
    }
}

fn show_window(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

/// Background loop: re-check on the configured interval and notify on new/updated apps.
async fn periodic_checks(app: AppHandle) {
    loop {
        let minutes = {
            let st = app.state::<AppState>();
            let m = st.data.lock().map(|d| d.settings.auto_check_minutes).unwrap_or(60);
            m
        };
        let wait = if minutes == 0 { 60 } else { minutes };
        tokio::time::sleep(std::time::Duration::from_secs(wait * 60)).await;

        let disabled = {
            let st = app.state::<AppState>();
            st.data.lock().map(|d| d.settings.auto_check_minutes == 0).unwrap_or(true)
        };
        if disabled {
            continue;
        }

        if let Ok(entries) = compute_apps(&app).await {
            notify_changes(&app, &entries);
            let _ = app.emit("apps-changed", ());
        }
    }
}

fn notify_changes(app: &AppHandle, entries: &[AppEntry]) {
    let st = app.state::<AppState>();
    let mut to_save = false;
    let mut new_count = 0;
    let mut upd_count = 0;

    for e in entries {
        let key_ver = e.latest_version.clone().unwrap_or_default();
        let already = {
            let d = st.data.lock();
            d.map(|d| d.notified.get(&e.name) == Some(&key_ver)).unwrap_or(true)
        };
        if e.is_new && !already {
            new_count += 1;
        }
        if e.status == "update_available" && !already {
            upd_count += 1;
        }
        if (e.is_new || e.status == "update_available") && !already {
            if let Ok(mut d) = st.data.lock() {
                d.notified.insert(e.name.clone(), key_ver);
                to_save = true;
            }
        }
    }
    if to_save {
        st.save();
    }

    if new_count > 0 || upd_count > 0 {
        let mut parts = Vec::new();
        if new_count > 0 {
            parts.push(format!("{new_count} nuovo/i progetto/i"));
        }
        if upd_count > 0 {
            parts.push(format!("{upd_count} aggiornamento/i"));
        }
        let _ = app
            .notification()
            .builder()
            .title("Russus Launcher")
            .body(parts.join(" · "))
            .show();
    }
}

fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let open_i = MenuItem::with_id(app, "open", "Apri Launcher", true, None::<&str>)?;
    let refresh_i = MenuItem::with_id(app, "refresh", "Controlla aggiornamenti", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let quit_i = MenuItem::with_id(app, "quit", "Esci", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open_i, &refresh_i, &sep, &quit_i])?;

    let _tray = TrayIconBuilder::with_id("main-tray")
        .icon(app.default_window_icon().cloned().unwrap())
        .tooltip("Russus Launcher")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show_window(app),
            "refresh" => {
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    if let Ok(entries) = compute_apps(&app).await {
                        notify_changes(&app, &entries);
                        let _ = app.emit("apps-changed", ());
                    }
                });
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle_window(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default();

    #[cfg(desktop)]
    {
        builder = builder
            .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
                show_window(app);
            }))
            .plugin(tauri_plugin_autostart::init(
                tauri_plugin_autostart::MacosLauncher::LaunchAgent,
                None,
            ));
    }

    builder
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::load())
        .manage(Cache {
            apps: Mutex::new(HashMap::new()),
        })
        .invoke_handler(tauri::generate_handler![
            list_apps,
            get_settings,
            save_settings,
            install_or_update,
            uninstall_app,
            launch_app
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            build_tray(&handle)?;

            // Run at login so the launcher lives in the tray (best effort).
            #[cfg(desktop)]
            {
                use tauri_plugin_autostart::ManagerExt;
                let mgr = app.autolaunch();
                if let Ok(false) = mgr.is_enabled() {
                    let _ = mgr.enable();
                }
            }

            // Hide to tray instead of quitting when the window is closed.
            if let Some(w) = app.get_webview_window("main") {
                let wc = w.clone();
                w.on_window_event(move |e| {
                    if let WindowEvent::CloseRequested { api, .. } = e {
                        api.prevent_close();
                        let _ = wc.hide();
                    }
                });
                let _ = w.show();
                let _ = w.set_focus();
            }

            // periodic update checks
            let h2 = handle.clone();
            tauri::async_runtime::spawn(periodic_checks(h2));

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
