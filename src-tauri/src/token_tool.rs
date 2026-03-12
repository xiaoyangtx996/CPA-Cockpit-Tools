use std::path::{Path, PathBuf};
use std::fs;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Mutex;
use serde::{Deserialize, Serialize};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use tauri::Emitter;

static CANCEL_REQUESTED: AtomicBool = AtomicBool::new(false);

pub fn request_cancel() {
    CANCEL_REQUESTED.store(true, Ordering::Relaxed);
}

fn reset_cancel() {
    CANCEL_REQUESTED.store(false, Ordering::Relaxed);
}

fn is_cancel_requested() -> bool {
    CANCEL_REQUESTED.load(Ordering::Relaxed)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TokenInfo {
    access_token: String,
    id_token: String,
    chatgpt_account_id: String,
    email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestResult {
    filename: String,
    email: String,
    account_id: String,
    category: String,
    status_code: Option<u16>,
    error: String,
    quota_info: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MergeItem {
    id: String,
    email: Option<String>,
    tokens: TokenData,
    created_at: Option<i64>,
    tags: Vec<String>,
    last_used: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TokenData {
    id_token: String,
    access_token: String,
    refresh_token: String,
}

fn emit_log(app: &tauri::AppHandle, text: &str, log_type: &str) {
    let _ = app.emit("script-log", super::LogEvent {
        text: text.to_string(),
        log_type: log_type.to_string(),
    });
}

/// 从JWT payload解码
fn decode_jwt_payload(token: &str) -> Option<serde_json::Value> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    
    let payload = parts[1];
    // 添加padding
    let padding = (4 - payload.len() % 4) % 4;
    let padded = format!("{}{}", payload, "=".repeat(padding));
    
    URL_SAFE_NO_PAD.decode(&padded).ok()
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .and_then(|json| serde_json::from_str(&json).ok())
}

/// 从JSON文件提取token信息
fn extract_token_info(data: &serde_json::Value) -> TokenInfo {
    let access_token = data.get("access_token")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    
    let id_token = data.get("id_token")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    
    let (chatgpt_account_id, email) = if !id_token.is_empty() {
        if let Some(payload) = decode_jwt_payload(&id_token) {
            let account_id = payload.get("https://api.openai.com/auth")
                .and_then(|v| v.get("chatgpt_account_id"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let email = payload.get("email")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            (account_id, email)
        } else {
            ("".to_string(), "".to_string())
        }
    } else {
        ("".to_string(), "".to_string())
    };
    
    TokenInfo {
        access_token,
        id_token,
        chatgpt_account_id,
        email,
    }
}

/// 测试单个token
async fn test_single_token(
    client: &reqwest::Client,
    token_info: &TokenInfo,
    filename: &str,
) -> TestResult {
    if is_cancel_requested() {
        return TestResult {
            filename: filename.to_string(),
            email: token_info.email.clone(),
            account_id: token_info.chatgpt_account_id.clone(),
            category: "canceled".to_string(),
            status_code: None,
            error: "canceled".to_string(),
            quota_info: serde_json::json!({}),
        };
    }

    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("Authorization", format!("Bearer {}", token_info.access_token).parse().unwrap());
    headers.insert("Content-Type", "application/json".parse().unwrap());
    headers.insert("User-Agent", "codex_cli_rs/0.76.0".parse().unwrap());
    
    if !token_info.chatgpt_account_id.is_empty() {
        headers.insert("Chatgpt-Account-Id", token_info.chatgpt_account_id.clone().parse().unwrap());
    }
    
    let url = "https://chatgpt.com/backend-api/wham/usage";
    
    let result = client.get(url).headers(headers).timeout(std::time::Duration::from_secs(30)).send().await;
    
    match result {
        Ok(resp) => {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            
            let (category, error, quota_info) = match status {
                401 => ("error".to_string(), "token_invalid_401".to_string(), serde_json::json!({})),
                403 => ("error".to_string(), "token_forbidden_403".to_string(), serde_json::json!({})),
                200 => {
                    if let Ok(data) = serde_json::from_str::<serde_json::Value>(&body) {
                        let remaining = data.get("usage").and_then(|u| u.get("remaining")).and_then(|r| r.as_i64());
                        let quota_info = serde_json::json!({
                            "usage": data.get("usage").cloned().unwrap_or(serde_json::json!({})),
                            "limits": data.get("limits").cloned().unwrap_or(serde_json::json!({})),
                            "remaining": remaining,
                        });
                        
                        if let Some(r) = remaining {
                            if r <= 0 {
                                ("no_quota".to_string(), "no_quota".to_string(), quota_info)
                            } else {
                                ("valid".to_string(), "".to_string(), quota_info)
                            }
                        } else {
                            ("valid".to_string(), "".to_string(), quota_info)
                        }
                    } else {
                        ("valid".to_string(), "".to_string(), serde_json::json!({}))
                    }
                }
                _ => ("error".to_string(), format!("http_{}", status), serde_json::json!({})),
            };
            
            TestResult {
                filename: filename.to_string(),
                email: token_info.email.clone(),
                account_id: token_info.chatgpt_account_id.clone(),
                category,
                status_code: Some(status),
                error,
                quota_info,
            }
        }
        Err(e) => TestResult {
            filename: filename.to_string(),
            email: token_info.email.clone(),
            account_id: token_info.chatgpt_account_id.clone(),
            category: "error".to_string(),
            status_code: None,
            error: e.to_string(),
            quota_info: serde_json::json!({}),
        },
    }
}

/// 测试tokens主函数
pub async fn test_tokens(
    app: &tauri::AppHandle,
    input_dir: &str,
    concurrency: usize,
) -> Result<String, String> {
    reset_cancel();

    let input_path = Path::new(input_dir);
    let output_path = PathBuf::from(format!("{}_result", input_dir));
    
    // 创建输出目录
    fs::create_dir_all(&output_path).map_err(|e| format!("创建输出目录失败: {}", e))?;
    
    emit_log(app, &format!("输入目录: {}", input_dir), "info");
    emit_log(app, &format!("输出目录: {}", output_path.display()), "info");
    emit_log(app, &format!("并发数: {}", concurrency), "info");
    
    // 加载JSON文件
    let mut files = Vec::new();
    for entry in fs::read_dir(input_path).map_err(|e| format!("读取目录失败: {}", e))? {
        let entry = entry.map_err(|e| format!("读取文件失败: {}", e))?;
        let path = entry.path();
        if path.extension().map_or(false, |ext| ext == "json") {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(data) = serde_json::from_str::<serde_json::Value>(&content) {
                    files.push((path.file_name().unwrap().to_string_lossy().to_string(), data, path));
                }
            }
        }
    }
    
    let total = files.len();
    emit_log(app, &format!("共发现 {} 个token文件", total), "info");
    
    if total == 0 {
        emit_log(app, "没有发现token文件", "warn");
        return Ok("没有发现token文件".to_string());
    }
    
    // HTTP客户端
    let client = Arc::new(reqwest::Client::new());
    let results = Arc::new(Mutex::new(Vec::new()));
    let counters = Arc::new(Mutex::new((0usize, 0usize, 0usize, 0usize))); // valid, no_quota, error, completed
    
    // 并发测试
    let mut tasks = Vec::new();
    let semaphore = Arc::new(tokio::sync::Semaphore::new(concurrency));
    
    for (filename, data, src_path) in files {
        let app = app.clone();
        let client = client.clone();
        let results = results.clone();
        let counters = counters.clone();
        let semaphore = semaphore.clone();
        let output_path = output_path.to_path_buf();

        let task = tokio::spawn(async move {
            if is_cancel_requested() {
                return;
            }
            let _permit = semaphore.acquire().await.unwrap();

            if is_cancel_requested() {
                return;
            }
            
            let token_info = extract_token_info(&data);
            let result = test_single_token(&client, &token_info, &filename).await;

            if is_cancel_requested() {
                return;
            }
            
            // 确定分类目录
            let (cat_dir, log_type) = match result.category.as_str() {
                "valid" => ("成功", "success"),
                "no_quota" => ("无额度", "no_quota"),
                _ => {
                    if result.status_code == Some(401) || result.error.contains("401") {
                        ("401失效", "error")
                    } else if result.status_code == Some(403) || result.error.contains("403") {
                        ("403禁止", "error")
                    } else {
                        ("其他错误", "error")
                    }
                }
            };
            
            // 创建分类目录并复制文件
            let cat_path = output_path.join(cat_dir);
            let files_path = cat_path.join("文件");
            fs::create_dir_all(&files_path).ok();
            fs::copy(&src_path, files_path.join(&filename)).ok();
            
            // 写入日志
            let log_msg = match result.category.as_str() {
                "valid" => format!("[成功] {} | {} | quota={:?}", filename, token_info.email, result.quota_info.get("remaining")),
                "no_quota" => format!("[无额度] {} | {}", filename, token_info.email),
                _ => format!("[异常] {} | {} | status={:?} | {}", filename, token_info.email, result.status_code, result.error),
            };
            emit_log(&app, &log_msg, log_type);
            
            // 更新计数
            let mut counters = counters.lock().await;
            match result.category.as_str() {
                "valid" => counters.0 += 1,
                "no_quota" => counters.1 += 1,
                "canceled" => {}
                _ => counters.2 += 1,
            }
            counters.3 += 1;
            
            if counters.3 % 100 == 0 || counters.3 == total {
                emit_log(&app, &format!("进度: {}/{} | 有额度:{} | 无额度:{} | 异常:{}", 
                    counters.3, total, counters.0, counters.1, counters.2), "info");
            }
            
            results.lock().await.push(result);
        });
        
        tasks.push(task);
    }
    
    // 等待所有任务完成
    futures::future::join_all(tasks).await;

    if is_cancel_requested() {
        emit_log(app, "任务已取消", "warn");
        return Err("任务已取消".to_string());
    }
    
    // 保存结果
    let results = results.lock().await.clone();
    let counters = counters.lock().await;
    
    let summary = serde_json::json!({
        "total": total,
        "success": counters.0,
        "no_quota": counters.1,
        "error": counters.2,
    });
    
    let output_json = output_path.join("test_results.json");
    fs::write(&output_json, serde_json::to_string_pretty(&serde_json::json!({
        "summary": summary,
        "details": results,
    })).unwrap()).ok();
    
    emit_log(app, &format!("完成! 成功:{} 无额度:{} 异常:{}", counters.0, counters.1, counters.2), "success");
    
    Ok(format!("{}_result", input_dir))
}

/// 合并tokens主函数
pub async fn merge_tokens(
    app: &tauri::AppHandle,
    input_dir: &str,
    output_dir: &str,
) -> Result<String, String> {
    reset_cancel();

    let input_path = Path::new(input_dir);
    let output_path = Path::new(output_dir);
    
    fs::create_dir_all(&output_path).map_err(|e| format!("创建输出目录失败: {}", e))?;
    
    emit_log(app, &format!("输入目录: {}", input_dir), "info");
    emit_log(app, &format!("递归扫描: {}", input_path.display()), "info");
    emit_log(app, &format!("输出目录: {}", output_dir), "info");
    
    let mut items = Vec::new();
    let mut count = 0usize;
    let mut skipped = 0usize;

    let id_re = regex::Regex::new(r"token_(oc[a-f0-9]+)_").map_err(|e| e.to_string())?;
    let ts_re = regex::Regex::new(r"_(\d+)\.json$").map_err(|e| e.to_string())?;

    for entry in walkdir::WalkDir::new(input_path).into_iter() {
        if is_cancel_requested() {
            emit_log(app, "任务已取消", "warn");
            return Err("任务已取消".to_string());
        }

        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                skipped += 1;
                emit_log(app, &format!("跳过(读取失败): {}", e), "warn");
                continue;
            }
        };

        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        let filename = match path.file_name().and_then(|n| n.to_str()) {
            Some(s) => s.to_string(),
            None => {
                skipped += 1;
                emit_log(app, &format!("跳过(文件名不可解析): {}", path.display()), "warn");
                continue;
            }
        };

        if path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase() != "json" {
            continue;
        }

        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                skipped += 1;
                emit_log(app, &format!("跳过(读取内容失败): {} | {}", filename, e), "warn");
                continue;
            }
        };

        let data = match serde_json::from_str::<serde_json::Value>(&content) {
            Ok(d) => d,
            Err(e) => {
                skipped += 1;
                emit_log(app, &format!("跳过(JSON解析失败): {} | {}", filename, e), "warn");
                continue;
            }
        };

        // 至少包含 access_token 或 refresh_token 或 id_token 才认为是token文件
        let has_any_token = data.get("access_token").and_then(|v| v.as_str()).unwrap_or("") != ""
            || data.get("refresh_token").and_then(|v| v.as_str()).unwrap_or("") != ""
            || data.get("id_token").and_then(|v| v.as_str()).unwrap_or("") != "";
        if !has_any_token {
            skipped += 1;
            emit_log(app, &format!("跳过(非token JSON): {}", filename), "info");
            continue;
        }

        let token_info = extract_token_info(&data);

        let timestamp = ts_re
            .captures(&filename)
            .and_then(|cap| cap.get(1))
            .and_then(|m| m.as_str().parse::<i64>().ok());

        let id = id_re
            .captures(&filename)
            .and_then(|cap| cap.get(1))
            .map(|m| m.as_str().to_string())
            .unwrap_or_else(|| filename.clone());

        items.push(MergeItem {
            id,
            email: if token_info.email.is_empty() { None } else { Some(token_info.email) },
            tokens: TokenData {
                id_token: token_info.id_token,
                access_token: token_info.access_token,
                refresh_token: data
                    .get("refresh_token")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
            },
            created_at: timestamp,
            tags: vec!["20260309".to_string()],
            last_used: timestamp,
        });

        count += 1;
        emit_log(app, &format!("已处理: {}", filename), "info");
    }
    
    // 按时间戳排序
    items.sort_by_key(|x| x.created_at.unwrap_or(0));
    
    // 保存结果
    let output_file = output_path.join("cockpit_accounts.json");
    fs::write(&output_file, serde_json::to_string_pretty(&items).unwrap())
        .map_err(|e| format!("写入文件失败: {}", e))?;
    
    emit_log(
        app,
        &format!("处理完成! 共处理 {} 个文件 (跳过 {})", count, skipped),
        "success",
    );
    emit_log(app, &format!("结果已保存到: {}", output_file.display()), "info");

    Ok(output_dir.to_string())
}
