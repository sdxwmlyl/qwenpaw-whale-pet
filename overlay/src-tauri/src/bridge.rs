//! 内置状态桥：替代原来的 Python bridge.exe（PyInstaller 会被杀软误报）
//! 对外契约与旧版一致：GET /state、POST /action

use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::Read;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const QWENPAW: &str = "http://127.0.0.1:64445";
const PORT: u16 = 7799;
const CELEBRATE_SECS: f64 = 8.0;

fn now_ts() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

fn http_get(path: &str, timeout_ms: u64) -> Option<Value> {
    let url = format!("{}{}", QWENPAW, path);
    let resp = ureq::get(&url)
        .set("User-Agent", "whale-pet")
        .timeout(Duration::from_millis(timeout_ms))
        .call()
        .ok()?;
    resp.into_json::<Value>().ok()
}

fn classify(status: &str) -> &'static str {
    match status.to_ascii_lowercase().as_str() {
        "running" | "working" | "busy" => "running",
        "waiting" | "waiting_approval" | "approval" | "pending" | "needs_approval" => "waiting",
        _ => "idle",
    }
}

fn tool_pose(tool: &str) -> &'static str {
    match tool {
        "read_file" | "glob_search" | "grep_search" | "memory_search" | "recall_history"
        | "file_reader" => "review",
        "write_file" | "edit_file" => "idea",
        "execute_shell_command" | "execute_shell" => "debug",
        "web_search" | "web_fetch" | "browser" => "thinking",
        "spawn_subagent" => "boss",
        "chat_with_agent" | "submit_to_agent" => "meeting",
        "list_agents" | "check_agent_task" => "slack",
        _ => "tool",
    }
}

/// 取最后一条 user 之后的消息，抽工具名 / usage / 是否有错误
fn extract(messages: &[Value]) -> (Option<String>, Option<Value>, bool) {
    let start = messages
        .iter()
        .rposition(|m| m.get("role").and_then(|r| r.as_str()) == Some("user"))
        .unwrap_or(0);
    let mut tool: Option<String> = None;
    let mut usage: Option<Value> = None;
    let mut err = false;
    let mut seen: std::collections::HashSet<(u64, u64, u64)> = std::collections::HashSet::new();
    let mut out_sum: u64 = 0;
    for m in &messages[start..] {
        let ty = m.get("type").and_then(|t| t.as_str()).unwrap_or("");
        if ty == "plugin_call" {
            if let Some(arr) = m.get("content").and_then(|c| c.as_array()) {
                for b in arr {
                    if let Some(name) = b
                        .get("data")
                        .and_then(|d| d.get("name"))
                        .and_then(|n| n.as_str())
                    {
                        tool = Some(name.to_string());
                    }
                }
            }
        }
        if ty == "plugin_call_output" {
            let st = m
                .get("status")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            let blob = m.get("content").map(|c| c.to_string()).unwrap_or_default();
            if ["error", "failed", "failure"].contains(&st.as_str())
                || blob.to_lowercase().contains("traceback")
            {
                err = true;
            }
        }
        if let Some(u) = m
            .get("metadata")
            .and_then(|x| x.get("metadata"))
            .and_then(|x| x.get("qwenpaw_turn_usage"))
            .and_then(|x| x.get("usage"))
        {
            // 后端把每个请求的 usage 复制到该请求的所有消息上；
            // 按 (prompt, last_prompt, completion) 去重后再求和，才是“本轮总输出”
            let key = (
                u.get("prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
                u.get("last_prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
                u.get("completion_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
            );
            if seen.insert(key) {
                out_sum += key.2;
            }
            usage = Some(u.clone());
        }
    }
    if let Some(u) = usage.clone() {
        let mut v = u.clone();
        v["completion_tokens"] = json!(out_sum);
        usage = Some(v);
    }
    (tool, usage, err)
}

struct Cache {
    cid: String,
    at: Instant,
    interval: Duration,
    tool: Option<String>,
    usage: Option<Value>,
    err: bool,
}

fn sane_stats(v: &mut Value) {
    let low = |k: &str| v[k].as_f64().unwrap_or(0.0) < 1.0;
    if low("battery") || low("mood") || low("satiety") {
        v["battery"] = json!(72.0);
        v["mood"] = json!(80.0);
        v["satiety"] = json!(65.0);
    }
}


pub fn start(state_dir: PathBuf) {
    let state = Arc::new(Mutex::new(json!({
        "ver": "v2.2-rust",
        "status": "idle",
        "tool": null,
        "pose": "idle",
        "tokens": {"prompt": 0, "output": 0, "cache_hit": 0, "context": 0},
        "last_turn": null,
        "finished_at": 0,
        "stats": {"battery": 72, "mood": 80, "satiety": 65, "tokens": 0, "tasks": 0},
        "error": false,
        "permission": false,
        "approval_pending": 0,
    })));

    // 读回三个数值
    let stats_path = state_dir.join("stats.json");
    if let Ok(txt) = std::fs::read_to_string(&stats_path) {
        if let Ok(v) = serde_json::from_str::<Value>(&txt) {
            if let Ok(mut s) = state.lock() {
                let mut v = v;
                sane_stats(&mut v);
                s["stats"] = v;
            }
        }
    }

    {
        let st = Arc::clone(&state);
        thread::spawn(move || poller(st));
    }
    {
        let st = Arc::clone(&state);
        let sp = stats_path.clone();
        thread::spawn(move || serve(st, sp));
    }
}

fn poller(state: Arc<Mutex<Value>>) {
    let mut prev: HashMap<String, String> = HashMap::new();
    let mut started: HashMap<String, Instant> = HashMap::new();
    let mut cache = Cache {
        cid: String::new(),
        at: Instant::now() - Duration::from_secs(60),
        interval: Duration::from_secs(8),
        tool: None,
        usage: None,
        err: false,
    };
    let mut turn_started = Instant::now();
    let mut last_tick = Instant::now();
    let mut celebrate_until = Instant::now() - Duration::from_secs(1);
    let mut last_stats_tick = Instant::now();

    loop {
        let now = Instant::now();
        if let Some(chats) = http_get("/api/chats", 4000).and_then(|v| v.as_array().cloned()) {
            let pend = http_get("/api/approval/list", 3000)
                .and_then(|v| {
                    v.get("count")
                        .and_then(|c| c.as_u64())
                        .or_else(|| v.get("pending_approvals").and_then(|a| a.as_array()).map(|a| a.len() as u64))
                })
                .unwrap_or(0);

            let mut next_prev: HashMap<String, String> = HashMap::new();
            let mut running: Vec<(String, Instant)> = Vec::new();
            let mut waiting_any = false;
            let mut celebrate = false;

            for c in &chats {
                if c.get("archived").and_then(|a| a.as_bool()).unwrap_or(false) {
                    continue;
                }
                let id = c.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                if id.is_empty() {
                    continue;
                }
                let st = classify(c.get("status").and_then(|v| v.as_str()).unwrap_or(""));
                let prev_st = prev.get(&id).cloned();
                if st == "waiting" {
                    waiting_any = true;
                }
                if st == "running" {
                    if prev_st.as_deref() != Some("running") {
                        started.insert(id.clone(), now);
                    }
                    let s = *started.get(&id).unwrap_or(&now);
                    running.push((id.clone(), s));
                }
                if prev_st.as_deref() == Some("running") && st != "running" {
                    celebrate = true;
                }
                next_prev.insert(id, st.to_string());
            }
            prev = next_prev;

            // 权限（含演示钩子）
            let demo_on = {
                let s = state.lock().unwrap();
                s.get("perm_demo_until")
                    .and_then(|v| v.as_f64())
                    .map(|t| now_ts() < t)
                    .unwrap_or(false)
            };
            if let Ok(mut s) = state.lock() {
                s["permission"] = json!(waiting_any || pend > 0 || demo_on);
                s["approval_pending"] = json!(if pend > 0 { pend } else if demo_on { 1 } else { 0 });
            }

            if celebrate {
                if let Ok(mut s) = state.lock() {
                    let dur = now.duration_since(turn_started).as_secs_f64().max(0.5);
                    let out = s["tokens"]["output"].as_f64().unwrap_or(0.0);
                    s["status"] = json!("idle");
                    s["pose"] = json!("success");
                    s["finished_at"] = json!(now_ts());
                    s["tool"] = Value::Null;
                    s["chat"] = Value::Null;
                    s["turn"] = json!({
                        "seconds": (dur * 10.0).round() / 10.0,
                        "output": out,
                        "tps": if out > 0.0 { ((out / dur) * 10.0).round() / 10.0 } else { 0.0 },
                        "cache_hit": s["tokens"]["cache_hit"].clone(),
                    });
                    s["last_turn"] = s["turn"].clone();
                    let tasks = s["stats"]["tasks"].as_f64().unwrap_or(0.0) + 1.0;
                    s["stats"]["tasks"] = json!(tasks);
                    let mood = (s["stats"]["mood"].as_f64().unwrap_or(0.0) + 4.0).min(100.0);
                    s["stats"]["mood"] = json!(mood);
                    let bat = (s["stats"]["battery"].as_f64().unwrap_or(0.0) - 3.0).max(0.0);
                    s["stats"]["battery"] = json!(bat);
                }
                celebrate_until = now + Duration::from_secs_f64(CELEBRATE_SECS);
                turn_started = now;
            } else if !running.is_empty() && now >= celebrate_until {
                running.sort_by(|a, b| b.1.cmp(&a.1));
                let (cid, _) = running[0].clone();
                let need = cache.cid != cid || now.duration_since(cache.at) >= cache.interval;
                if need {
                    let t0 = Instant::now();
                    if let Some(detail) = http_get(&format!("/api/chats/{}", cid), 15000) {
                        if let Some(msgs) = detail.get("messages").and_then(|m| m.as_array()) {
                            let (tool, usage, err) = extract(msgs);
                            cache.tool = tool;
                            cache.usage = usage;
                            cache.err = err;
                        }
                    }
                    let spent = t0.elapsed();
                    cache.interval = if spent > Duration::from_millis(800) {
                        Duration::from_secs(30)
                    } else if spent > Duration::from_millis(400) {
                        Duration::from_secs(15)
                    } else {
                        Duration::from_secs(8)
                    };
                    cache.cid = cid.clone();
                    cache.at = now;
                }
                let tool = cache.tool.clone();
                let pose = tool.as_deref().map(tool_pose).unwrap_or("thinking");
                if let Ok(mut s) = state.lock() {
                    if s["status"].as_str() != Some("working") {
                        turn_started = now;
                    }
                    s["status"] = json!("working");
                    s["chat"] = json!(cid.chars().take(8).collect::<String>());
                    s["tool"] = json!(tool);
                    s["pose"] = json!(pose);
                    s["error"] = json!(cache.err);
                    if let Some(u) = &cache.usage {
                        s["tokens"] = json!({
                            "prompt": u.get("last_prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
                            "output": u.get("completion_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
                            "cache_hit": ((u.get("cache_hit_rate").and_then(|v| v.as_f64()).unwrap_or(0.0)) * 100.0).round() / 100.0,
                            "context": u.get("context_size").and_then(|v| v.as_u64()).unwrap_or(0),
                        });
                        s["last_turn"] = s["tokens"].clone();
                    }
                }
            } else {
                if let Ok(mut s) = state.lock() {
                    if now >= celebrate_until {
                        s["status"] = json!("idle");
                        s["chat"] = Value::Null;
                    }
                    let fin = s["finished_at"].as_f64().unwrap_or(0.0);
                    if now_ts() - fin > 6.0 && s["pose"].as_str() == Some("success") {
                        s["pose"] = json!("idle");
                    }
                }
            }
        }

        // 数值随时间衰减
        if now.duration_since(last_tick) >= Duration::from_secs(60) {
            last_tick = now;
            if let Ok(mut s) = state.lock() {
                let mut sat = s["stats"]["satiety"].as_f64().unwrap_or(65.0) - 1.0;
                sat = sat.max(15.0);                      // 地板：不要掉到 0（掉到 0 会一直蔫着）
                s["stats"]["satiety"] = json!(sat);
                if s["status"].as_str() == Some("working") {
                    let b = (s["stats"]["battery"].as_f64().unwrap_or(72.0) - 1.0).max(20.0);
                    let m = (s["stats"]["mood"].as_f64().unwrap_or(80.0) - 1.0).max(20.0);
                    s["stats"]["battery"] = json!(b);
                    s["stats"]["mood"] = json!(m);
                } else {
                    let b = (s["stats"]["battery"].as_f64().unwrap_or(72.0) + 0.5).min(100.0); // 空闲回电
                    s["stats"]["battery"] = json!(b);
                }
                s["food_low"] = json!(sat < 20.0);
            }
            last_stats_tick = now;
        }
        let _ = last_stats_tick;

        thread::sleep(Duration::from_secs(2));
    }
}

fn serve(state: Arc<Mutex<Value>>, stats_path: PathBuf) {
    let server = match tiny_http::Server::http(("127.0.0.1", PORT)) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("bridge: cannot bind {}: {}", PORT, e);
            return;
        }
    };
    for mut req in server.incoming_requests() {
        let url = req.url().to_string();
        let method = req.method().clone();
        if url.starts_with("/state") || url.starts_with("/health") {
            let body = {
                let s = state.lock().unwrap();
                if url.starts_with("/health") {
                    json!({"ok": true})
                } else {
                    s.clone()
                }
            };
            let _ = req.respond(
                tiny_http::Response::from_string(body.to_string()).with_header(
                    tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap(),
                ),
            );
        } else if url.starts_with("/action") && method == tiny_http::Method::Post {
            let mut buf = String::new();
            let _ = req.as_reader().take(8192).read_to_string(&mut buf);
            let act = serde_json::from_str::<Value>(&buf)
                .ok()
                .and_then(|v| v.get("act").and_then(|a| a.as_str()).map(|s| s.to_string()))
                .unwrap_or_default();
            {
                let mut s = state.lock().unwrap();
                match act.as_str() {
                    "feed" => {
                        let v = (s["stats"]["satiety"].as_f64().unwrap_or(0.0) + 18.0).min(100.0);
                        s["stats"]["satiety"] = json!(v);
                        let m = (s["stats"]["mood"].as_f64().unwrap_or(0.0) + 6.0).min(100.0);
                        s["stats"]["mood"] = json!(m);
                    }
                    "headpat" => {
                        let m = (s["stats"]["mood"].as_f64().unwrap_or(0.0) + 5.0).min(100.0);
                        s["stats"]["mood"] = json!(m);
                    }
                    "sleep" => {
                        let b = (s["stats"]["battery"].as_f64().unwrap_or(0.0) + 10.0).min(100.0);
                        s["stats"]["battery"] = json!(b);
                    }
                    "test_sound" | "beep" => s["beep"] = json!(now_ts()),
                    "demo_permission" => s["perm_demo_until"] = json!(now_ts() + 8.0),
                    _ => {}
                }
                let stats = s["stats"].clone();
                if let Ok(txt) = serde_json::to_string_pretty(&stats) {
                    let _ = std::fs::write(&stats_path, txt);
                }
                let body = json!({"ok": true, "stats": stats});
                let _ = req.respond(
                    tiny_http::Response::from_string(body.to_string()).with_header(
                        tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap(),
                    ),
                );
            }
        } else {
            let _ = req.respond(tiny_http::Response::from_string("not found").with_status_code(404));
        }
    }
}
