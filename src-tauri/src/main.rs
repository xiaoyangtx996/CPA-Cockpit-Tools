#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod token_tool;

use tauri::Emitter;

#[derive(Clone, serde::Serialize)]
struct LogEvent {
    text: String,
    #[serde(rename = "type")]
    log_type: String,
}

fn emit_log(app: &tauri::AppHandle, text: &str, log_type: &str) {
    let _ = app.emit("script-log", LogEvent {
        text: text.to_string(),
        log_type: log_type.to_string(),
    });
}

#[tauri::command]
async fn test_tokens(
    app: tauri::AppHandle,
    input_dir: String,
    concurrency: Option<usize>,
) -> Result<String, String> {
    let concurrency = concurrency.unwrap_or(50);
    token_tool::test_tokens(&app, &input_dir, concurrency).await
}

#[tauri::command]
async fn merge_tokens(
    app: tauri::AppHandle,
    input_dir: String,
    output_dir: String,
) -> Result<String, String> {
    token_tool::merge_tokens(&app, &input_dir, &output_dir).await
}

#[tauri::command]
fn stop_script() -> Result<(), String> {
    token_tool::request_cancel();
    Ok(())
}

#[tauri::command]
fn copy_file_to_dir(src_file: String, dst_dir: String, dst_filename: Option<String>) -> Result<String, String> {
    use std::path::PathBuf;

    let src = PathBuf::from(&src_file);
    if !src.exists() {
        return Err(format!("源文件不存在: {}", src_file));
    }

    let dst_dir_path = PathBuf::from(&dst_dir);
    std::fs::create_dir_all(&dst_dir_path).map_err(|e| format!("创建目标目录失败: {}", e))?;

    let filename = dst_filename
        .or_else(|| src.file_name().and_then(|n| n.to_str()).map(|s| s.to_string()))
        .ok_or_else(|| "无法确定目标文件名".to_string())?;

    let dst = dst_dir_path.join(filename);
    std::fs::copy(&src, &dst).map_err(|e| format!("复制文件失败: {}", e))?;
    Ok(dst.to_string_lossy().to_string())
}

#[tauri::command]
fn open_folder(path: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    std::process::Command::new("explorer")
        .arg(&path)
        .spawn()
        .map_err(|e| format!("打开目录失败: {}", e))?;
    #[cfg(target_os = "macos")]
    std::process::Command::new("open")
        .arg(&path)
        .spawn()
        .map_err(|e| format!("打开目录失败: {}", e))?;
    #[cfg(target_os = "linux")]
    std::process::Command::new("xdg-open")
        .arg(&path)
        .spawn()
        .map_err(|e| format!("打开目录失败: {}", e))?;
    Ok(())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            test_tokens,
            merge_tokens,
            stop_script,
            open_folder,
            copy_file_to_dir,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
