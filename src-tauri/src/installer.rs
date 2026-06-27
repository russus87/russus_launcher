use crate::state::cache_dir;
use futures_util::StreamExt;
use serde::Serialize;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};
use tokio::io::AsyncWriteExt;

#[derive(Debug, Clone, Serialize)]
pub struct Progress {
    pub app: String,
    pub phase: String, // download | install | done | error
    pub message: String,
    pub percent: i32, // -1 = indeterminate
}

fn emit(app: &AppHandle, name: &str, phase: &str, message: &str, percent: i32) {
    let _ = app.emit(
        "progress",
        Progress {
            app: name.to_string(),
            phase: phase.to_string(),
            message: message.to_string(),
            percent,
        },
    );
}

/// Download an asset to the cache dir, streaming progress events.
pub async fn download(
    app: &AppHandle,
    name: &str,
    url: &str,
    filename: &str,
) -> Result<PathBuf, String> {
    let dir = cache_dir();
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| e.to_string())?;
    let dest = dir.join(filename);

    let client = reqwest::Client::builder()
        .user_agent("russus-launcher")
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client.get(url).send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("Download fallito: {}", resp.status()));
    }
    let total = resp.content_length().unwrap_or(0);
    let mut file = tokio::fs::File::create(&dest)
        .await
        .map_err(|e| e.to_string())?;
    let mut downloaded: u64 = 0;
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        file.write_all(&chunk).await.map_err(|e| e.to_string())?;
        downloaded += chunk.len() as u64;
        let pct = if total > 0 {
            ((downloaded as f64 / total as f64) * 100.0) as i32
        } else {
            -1
        };
        emit(
            app,
            name,
            "download",
            &format!("Scaricamento… {}", human(downloaded, total)),
            pct,
        );
    }
    file.flush().await.map_err(|e| e.to_string())?;
    Ok(dest)
}

fn human(done: u64, total: u64) -> String {
    let mb = |b: u64| format!("{:.1} MB", b as f64 / 1_048_576.0);
    if total > 0 {
        format!("{} / {}", mb(done), mb(total))
    } else {
        mb(done)
    }
}

/// Install (or update) a downloaded package. Blocking command run on a thread.
pub async fn install_package(app: &AppHandle, name: &str, file: &PathBuf) -> Result<(), String> {
    emit(app, name, "install", "Installazione…", -1);

    #[cfg(target_os = "linux")]
    {
        // pacman needs root: use pkexec (polkit GUI prompt).
        let status = tokio::process::Command::new("pkexec")
            .args(["pacman", "-U", "--noconfirm"])
            .arg(file)
            .status()
            .await
            .map_err(|e| format!("Impossibile avviare pkexec/pacman: {e}"))?;
        if !status.success() {
            return Err("Installazione annullata o fallita (pacman -U).".to_string());
        }
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        return install_dmg(file).await;
    }

    #[allow(unreachable_code)]
    {
        let _ = file;
        Err("Piattaforma non supportata".into())
    }
}

#[cfg(target_os = "macos")]
async fn install_dmg(file: &PathBuf) -> Result<(), String> {
    // Mount, copy the .app to /Applications, detach.
    let mount = format!("/Volumes/russus-launcher-{}", std::process::id());
    let out = tokio::process::Command::new("hdiutil")
        .args(["attach", "-nobrowse", "-mountpoint", &mount])
        .arg(file)
        .output()
        .await
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err("hdiutil attach fallito".into());
    }
    let res = (|| async {
        let mut entries = tokio::fs::read_dir(&mount).await.map_err(|e| e.to_string())?;
        let mut app_path = None;
        while let Some(e) = entries.next_entry().await.map_err(|e| e.to_string())? {
            if e.file_name().to_string_lossy().ends_with(".app") {
                app_path = Some(e.path());
                break;
            }
        }
        let app_path = app_path.ok_or("Nessuna .app nel dmg")?;
        let dest = PathBuf::from("/Applications").join(app_path.file_name().unwrap());
        let _ = tokio::fs::remove_dir_all(&dest).await;
        let status = tokio::process::Command::new("cp")
            .arg("-R")
            .arg(&app_path)
            .arg("/Applications/")
            .status()
            .await
            .map_err(|e| e.to_string())?;
        if !status.success() {
            return Err("Copia in /Applications fallita".to_string());
        }
        Ok::<(), String>(())
    })()
    .await;
    let _ = tokio::process::Command::new("hdiutil")
        .args(["detach", &mount])
        .status()
        .await;
    res
}

/// Query the locally installed version (linux: pacman; macOS handled via state).
#[cfg(target_os = "linux")]
pub async fn installed_version(pkgname: &str) -> Option<String> {
    let out = tokio::process::Command::new("pacman")
        .args(["-Q", pkgname])
        .output()
        .await
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout);
    // "oxiterm 0.7.1-1"
    let ver = s.split_whitespace().nth(1)?;
    Some(ver.split('-').next().unwrap_or(ver).to_string())
}

#[cfg(not(target_os = "linux"))]
pub async fn installed_version(_pkgname: &str) -> Option<String> {
    None
}

#[cfg(target_os = "linux")]
pub async fn uninstall(pkgname: &str) -> Result<(), String> {
    let status = tokio::process::Command::new("pkexec")
        .args(["pacman", "-R", "--noconfirm", pkgname])
        .status()
        .await
        .map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err("Disinstallazione annullata o fallita.".into())
    }
}

#[cfg(target_os = "macos")]
pub async fn uninstall(app_name: &str) -> Result<(), String> {
    let dest = PathBuf::from("/Applications").join(format!("{app_name}.app"));
    tokio::fs::remove_dir_all(&dest)
        .await
        .map_err(|e| e.to_string())
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub async fn uninstall(_id: &str) -> Result<(), String> {
    Err("Piattaforma non supportata".into())
}

/// Launch an installed app.
pub async fn launch(id: &str) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        // Tauri apps install a binary named after the package on $PATH.
        tokio::process::Command::new(id)
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("Avvio non riuscito ({id}): {e}"))
    }
    #[cfg(target_os = "macos")]
    {
        tokio::process::Command::new("open")
            .args(["-a", id])
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = id;
        Err("Piattaforma non supportata".into())
    }
}

pub fn notify_done(app: &AppHandle, name: &str, msg: &str) {
    emit(app, name, "done", msg, 100);
}

pub fn notify_error(app: &AppHandle, name: &str, msg: &str) {
    emit(app, name, "error", msg, -1);
}
