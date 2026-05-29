//! Heuristic detection of voice/video call applications.

const CALL_APP_HINTS: &[&str] = &[
    "zoom",
    "teams",
    "microsoft teams",
    "facetime",
    "discord",
    "webex",
    "slack",
    "skype",
    "voov",
    "tencent meeting",
    "tencentmeeting",
    "lark",
    "飞书",
    "whatsapp",
    "telegram",
    "wechat",
    "wxwork",
    "企业微信",
    "line",
    "gotomeeting",
    "ringcentral",
    "bluejeans",
    "whereby",
    "meet.google",
];

fn name_matches_call_app(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    CALL_APP_HINTS
        .iter()
        .any(|hint| lower.contains(hint))
}

#[cfg(unix)]
fn running_process_names() -> Vec<String> {
    use std::process::Command;
    let output = Command::new("ps").args(["-A", "-o", "comm="]).output();
    match output {
        Ok(out) if out.status.success() => out
            .stdout
            .split(|&b| b == b'\n')
            .filter_map(|line| {
                let s = String::from_utf8_lossy(line).trim().to_string();
                (!s.is_empty()).then_some(s)
            })
            .collect(),
        _ => Vec::new(),
    }
}

#[cfg(windows)]
fn running_process_names() -> Vec<String> {
    use std::process::Command;
    let output = Command::new("tasklist")
        .args(["/FO", "CSV", "/NH"])
        .output();
    match output {
        Ok(out) if out.status.success() => out
            .stdout
            .split(|&b| b == b'\n')
            .filter_map(|line| {
                let line = String::from_utf8_lossy(line);
                let name = line.split(',').next()?.trim_matches('"');
                (!name.is_empty()).then(|| name.to_string())
            })
            .collect(),
        _ => Vec::new(),
    }
}

#[cfg(not(any(unix, windows)))]
fn running_process_names() -> Vec<String> {
    Vec::new()
}

/// Capture modes that always enable output + microphone.
pub fn wants_dual_capture(mode: &str) -> bool {
    matches!(
        mode.trim().to_lowercase().as_str(),
        "call" | "dual" | "tap+mic" | "mic+output" | "call_dual"
    )
}

/// Returns true when a known voice/video call application appears to be running.
pub fn voice_call_active() -> bool {
    running_process_names()
        .iter()
        .any(|name| name_matches_call_app(name))
}
