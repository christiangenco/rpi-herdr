//! Herdr adapter for rpi.
//!
//! Loaded from `~/.rpi/agent/extensions`. Outside a Herdr pane (`HERDR_ENV`
//! unset) every hook returns immediately. Inside one, it reports `idle`,
//! `working`, and `blocked` the same way Pi's own extension does, so the
//! sidebar, notifications, and `herdr agent` commands can see the pane.
//!
//! Herdr 0.9.1 on this machine accepts session identity, not a resume command.
//! Resume-on-restore needs Herdr 0.9.2 or newer.

use std::ffi::c_void;
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use rpi_plugin_sdk::events::EventView;
use rpi_plugin_sdk::{EventTag, StablePluginEvent};

const SOURCE: &str = "rpi";
const AGENT: &str = "rpi";

extern "C" {
    fn atexit(cb: extern "C" fn()) -> i32;
}

extern "C" fn on_process_exit() {
    let _ = rpi_plugin_sdk::guard_or((), release);
}

struct Track {
    working: bool,
    blocked: u32,
    session_id: Option<String>,
    last_state: Option<&'static str>,
    last_session: Option<String>,
}

static TRACK: Mutex<Track> = Mutex::new(Track {
    working: false,
    blocked: 0,
    session_id: None,
    last_state: None,
    last_session: None,
});

static SEQ: AtomicU64 = AtomicU64::new(0);

fn in_herdr() -> bool {
    std::env::var("HERDR_ENV").ok().as_deref() == Some("1")
        && std::env::var("HERDR_PANE_ID").ok().filter(|s| !s.is_empty()).is_some()
        && (std::env::var("HERDR_BIN_PATH").ok().filter(|s| !s.is_empty()).is_some()
            || which_herdr())
}

fn which_herdr() -> bool {
    std::env::var_os("PATH").is_some()
}

fn next_seq() -> u64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
        .saturating_mul(1000);
    let mut cur = SEQ.load(Ordering::Relaxed);
    loop {
        let next = cur.max(now).saturating_add(1);
        match SEQ.compare_exchange_weak(cur, next, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => return next,
            Err(observed) => cur = observed,
        }
    }
}

fn log_line(line: &str) {
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/rpi-herdr.log")
    {
        let _ = writeln!(file, "{line}");
    }
}

fn herdr_bin() -> String {
    std::env::var("HERDR_BIN_PATH").unwrap_or_else(|_| "herdr".to_string())
}

fn pane_id() -> Option<String> {
    std::env::var("HERDR_PANE_ID").ok().filter(|s| !s.is_empty())
}

fn run_herdr(args: &[String]) -> Result<(), String> {
    let bin = herdr_bin();
    let output = Command::new(&bin)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("spawn {bin}: {e}"))?;
    if output.status.success() {
        return Ok(());
    }
    let err = String::from_utf8_lossy(&output.stderr);
    let out = String::from_utf8_lossy(&output.stdout);
    Err(format!(
        "status {} stderr={} stdout={}",
        output.status,
        err.trim(),
        out.trim()
    ))
}

fn note_session(raw: &str) {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(raw) else {
        return;
    };
    let Some(id) = value.get("sessionId").and_then(|v| v.as_str()) else {
        return;
    };
    if id.is_empty() || id.contains(['\'', '\n', '\r']) {
        return;
    }
    if let Ok(mut track) = TRACK.lock() {
        if track.session_id.as_deref() != Some(id) {
            track.session_id = Some(id.to_string());
        }
    }
}

fn desired(track: &Track) -> &'static str {
    if track.blocked > 0 {
        "blocked"
    } else if track.working {
        "working"
    } else {
        "idle"
    }
}

fn publish(force: bool) {
    if !in_herdr() {
        return;
    }
    let Some(pane) = pane_id() else {
        return;
    };
    let (state, session_id, changed) = {
        let Ok(mut track) = TRACK.lock() else {
            return;
        };
        let state = desired(&track);
        let session = track.session_id.clone();
        let changed = force
            || track.last_state != Some(state)
            || track.last_session != session;
        if changed {
            track.last_state = Some(state);
            track.last_session = session.clone();
        }
        (state, session, changed)
    };
    if !changed {
        return;
    }
    let seq = next_seq().to_string();
    let mut args = vec![
        "pane".into(),
        "report-agent".into(),
        pane,
        "--source".into(),
        SOURCE.into(),
        "--agent".into(),
        AGENT.into(),
        "--state".into(),
        state.into(),
        "--seq".into(),
        seq,
    ];
    if state == "blocked" {
        args.push("--message".into());
        args.push("waiting for a decision".into());
    }
    if let Some(id) = session_id.as_deref() {
        args.push("--agent-session-id".into());
        args.push(id.to_string());
    }
    match run_herdr(&args) {
        Ok(()) => log_line(&format!("report {state} session={}", session_id.unwrap_or_default())),
        Err(err) => log_line(&format!("report {state} failed: {err}")),
    }
}

fn release() {
    if !in_herdr() {
        return;
    }
    let Some(pane) = pane_id() else {
        return;
    };
    let seq = next_seq().to_string();
    let args = vec![
        "pane".into(),
        "release-agent".into(),
        pane,
        "--source".into(),
        SOURCE.into(),
        "--agent".into(),
        AGENT.into(),
        "--seq".into(),
        seq,
    ];
    match run_herdr(&args) {
        Ok(()) => log_line("release"),
        Err(err) => log_line(&format!("release failed: {err}")),
    }
}

fn apply(tag: EventTag) {
    let mut shutdown = false;
    if let Ok(mut track) = TRACK.lock() {
        match tag {
            EventTag::SessionStart | EventTag::BeforeTuiStart | EventTag::AgentSettled => {
                track.working = false;
            }
            EventTag::AgentStart | EventTag::TurnStart => {
                track.working = true;
            }
            EventTag::AgentEnd | EventTag::TurnEnd | EventTag::TurnEndPayload => {
                track.working = false;
            }
            EventTag::UiPromptStart => {
                track.blocked = track.blocked.saturating_add(1);
            }
            EventTag::UiPromptEnd => {
                track.blocked = track.blocked.saturating_sub(1);
            }
            EventTag::SessionShutdown => {
                shutdown = true;
            }
            _ => {}
        }
    }
    if shutdown {
        release();
        return;
    }
    publish(false);
}

extern "C" fn on_event(event: StablePluginEvent, _user_data: *mut c_void) -> i32 {
    rpi_plugin_sdk::guard_or(0, || {
        if !in_herdr() {
            return 0;
        }
        let view = unsafe { EventView::from_raw(&event) };
        let tag = view.tag();
        if let Some(text) = view.data_text() {
            note_session(&text);
        }
        log_line(&format!("event {tag:?}"));
        apply(tag);
        0
    })
}

fn register(api: &rpi_plugin_sdk::PluginApi, tag: EventTag) -> i32 {
    let Some(register_event_handler) = api.register_event_handler else {
        return 2;
    };
    register_event_handler(tag, on_event, std::ptr::null_mut())
}

rpi_plugin_sdk::export_plugin!(|api| {
    if !in_herdr() {
        return 0;
    }
    const TAGS: &[EventTag] = &[
        EventTag::SessionStart,
        EventTag::BeforeTuiStart,
        EventTag::BeforeAgentStart,
        EventTag::AgentStart,
        EventTag::AgentEnd,
        EventTag::AgentSettled,
        EventTag::TurnStart,
        EventTag::TurnEnd,
        EventTag::TurnEndPayload,
        EventTag::UiPromptStart,
        EventTag::UiPromptEnd,
        EventTag::SessionShutdown,
    ];
    for tag in TAGS {
        let rc = register(api, *tag);
        if rc != 0 {
            log_line(&format!("register {tag:?} failed: {rc}"));
            return rc;
        }
    }
    log_line("plugin loaded");
    // A missing key, a panic, or quit can skip SessionShutdown. Release so
    // Herdr does not keep a dead agent on the pane.
    unsafe {
        let _ = atexit(on_process_exit);
    }
    publish(true);
    0
});
