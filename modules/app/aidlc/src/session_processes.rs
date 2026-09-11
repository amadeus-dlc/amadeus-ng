//! 本家と同じ50ms/64世代のsession ancestry観測。
use crate::layout::Layout;
use core_infrastructure::canon_json::{JsonValue, ObjectMembers, SerializationProfile, serialize};
use std::{
    collections::BTreeSet,
    fs,
    io::Read as _,
    path::Path,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

struct Identity {
    parent: u32,
    start: String,
}
impl Identity {
    const fn new(parent: u32, start: String) -> Self {
        Self { parent, start }
    }
    fn read(pid: u32, deadline: Instant) -> Option<Self> {
        if pid <= 1 || Instant::now() >= deadline {
            return None;
        }
        let platform = platform();
        if platform == "linux" {
            let raw = fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
            let (_, rest) = raw.rsplit_once(')')?;
            let fields: Vec<_> = rest.split_whitespace().collect();
            return Some(Self::new(
                fields.get(1)?.parse().ok()?,
                fields.get(19)?.to_string(),
            ));
        }
        if platform != "darwin"
            || std::env::var("AIDLC_TEST_PS_DENIED").as_deref() == Ok("1")
            || deadline.saturating_duration_since(Instant::now()) <= Duration::from_millis(1)
        {
            return None;
        }
        let mut child = Command::new("ps")
            .args(["-o", "ppid=,lstart=", "-p", &pid.to_string()])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;
        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    if !status.success() {
                        return None;
                    }
                    let mut bytes = Vec::new();
                    child.stdout.take()?.read_to_end(&mut bytes).ok()?;
                    let text = String::from_utf8_lossy(&bytes);
                    let text = core_infrastructure::ecmascript::trim(&text);
                    let (parent, start) = text.split_once(char::is_whitespace)?;
                    let start = core_infrastructure::ecmascript::trim(start);
                    if start.is_empty() {
                        return None;
                    }
                    return Some(Self::new(parent.parse().ok()?, start.to_string()));
                }
                Ok(None) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(1))
                }
                _ => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return None;
                }
            }
        }
    }
}
fn platform() -> String {
    std::env::var("AIDLC_TEST_SESSION_PLATFORM")
        .ok()
        .filter(|value| ["linux", "darwin", "win32"].contains(&value.as_str()))
        .unwrap_or_else(|| {
            match std::env::consts::OS {
                "linux" => "linux",
                "macos" => "darwin",
                _ => "win32",
            }
            .to_string()
        })
}
fn entry(project: &Path, pid: u32) -> Option<(String, Option<String>)> {
    let value: serde_json::Value = serde_json::from_slice(
        &fs::read(
            project
                .join("aidlc/.aidlc-sessions/pids")
                .join(pid.to_string()),
        )
        .ok()?,
    )
    .ok()?;
    let session = value.get("sessionId")?.as_str()?;
    if !Layout::valid_session_id(session) {
        return None;
    }
    let start = match value.get("startTime")? {
        serde_json::Value::Null => None,
        serde_json::Value::String(value) => Some(value.clone()),
        _ => return None,
    };
    Some((session.to_string(), start))
}
pub(crate) fn write_ancestry(project: &Path, session: &str) {
    if platform() == "win32" || !Layout::valid_session_id(session) {
        return;
    }
    let deadline = Instant::now() + Duration::from_millis(50);
    let directory = project.join("aidlc/.aidlc-sessions/pids");
    if let Ok(entries) = fs::read_dir(&directory) {
        for file in entries.filter_map(Result::ok) {
            if Instant::now() >= deadline {
                break;
            }
            let name = file.file_name().to_string_lossy().into_owned();
            if name.is_empty() || !name.bytes().all(|byte| byte.is_ascii_digit()) {
                continue;
            }
            let Ok(pid) = name.parse::<u32>() else {
                continue;
            };
            let stored = entry(project, pid);
            let identity = Identity::read(pid, deadline);
            let stale = stored.is_none()
                || !core_infrastructure::process::is_alive(pid)
                || stored
                    .as_ref()
                    .and_then(|(_, start)| start.as_ref())
                    .is_some_and(|start| {
                        identity
                            .as_ref()
                            .is_none_or(|identity| &identity.start != start)
                    });
            if stale {
                let _ = fs::remove_file(file.path());
            }
        }
    }
    let mut pid = core_infrastructure::process::parent_id().unwrap_or(0);
    let mut seen = BTreeSet::new();
    for _ in 0..64 {
        if pid <= 1 || !seen.insert(pid) || Instant::now() >= deadline {
            break;
        }
        let Some(identity) = Identity::read(pid, deadline) else {
            break;
        };
        if Instant::now() < deadline && core_infrastructure::process::is_alive(pid) {
            let observed = Identity::read(pid, deadline);
            let mut fields = ObjectMembers::new();
            fields.insert("sessionId", JsonValue::String(session.into()));
            fields.insert(
                "startTime",
                observed.map_or(JsonValue::Null, |identity| {
                    JsonValue::String(identity.start)
                }),
            );
            if fs::create_dir_all(&directory).is_ok() {
                let _ = fs::write(
                    directory.join(pid.to_string()),
                    format!(
                        "{}\n",
                        serialize(
                            &JsonValue::Object(fields),
                            SerializationProfile::ContractCompact
                        )
                    ),
                );
            }
        }
        pid = identity.parent;
    }
}
pub(crate) fn resolve(project: &Path) -> Option<String> {
    if platform() == "win32" || !project.join("aidlc/.aidlc-sessions/pids").exists() {
        return None;
    }
    let deadline = Instant::now() + Duration::from_millis(50);
    let mut pid = core_infrastructure::process::parent_id()?;
    let mut seen = BTreeSet::new();
    for _ in 0..64 {
        if pid <= 1 || !seen.insert(pid) || Instant::now() >= deadline {
            return None;
        }
        let identity = Identity::read(pid, deadline)?;
        if !core_infrastructure::process::is_alive(pid) {
            return None;
        }
        if let Some((session, start)) = entry(project, pid)
            && start.is_none_or(|start| start == identity.start)
        {
            return Some(session);
        }
        pid = identity.parent;
    }
    None
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;

    fn write_pid(project: &Path, name: &str, body: &str) {
        let directory = project.join("aidlc/.aidlc-sessions/pids");
        fs::create_dir_all(&directory).expect("pids");
        fs::write(directory.join(name), body).expect("pid 記録");
    }

    /// pid 記録は session 名の文法と `startTime` の型で読む・読まないが決まる。
    #[test]
    fn a_pid_entry_needs_a_valid_session_and_a_null_or_text_start_time() {
        let root = tempfile::tempdir().expect("一時ディレクトリ");
        assert_eq!(entry(root.path(), 4242), None, "記録が無い");
        write_pid(
            root.path(),
            "4242",
            r#"{"sessionId":"bad session","startTime":null}"#,
        );
        assert_eq!(entry(root.path(), 4242), None, "文法外の session");
        write_pid(
            root.path(),
            "4242",
            r#"{"sessionId":"s-1","startTime":null}"#,
        );
        assert_eq!(entry(root.path(), 4242), Some(("s-1".into(), None)));
        write_pid(
            root.path(),
            "4242",
            r#"{"sessionId":"s-1","startTime":"1000"}"#,
        );
        assert_eq!(
            entry(root.path(), 4242),
            Some(("s-1".into(), Some("1000".into())))
        );
        write_pid(root.path(), "4242", r#"{"sessionId":"s-1","startTime":7}"#);
        assert_eq!(
            entry(root.path(), 4242),
            None,
            "数値の startTime は読まない"
        );
        write_pid(root.path(), "4242", "{broken");
        assert_eq!(entry(root.path(), 4242), None);
    }

    /// 掃除は数字でない名前を飛ばし、死んだ pid と読めない記録を消し、自プロセスの
    /// 祖先を書き足す。文法外の session では何もしない。
    #[test]
    fn ancestry_cleanup_removes_stale_entries_and_keeps_live_ones() {
        let root = tempfile::tempdir().expect("一時ディレクトリ");
        let pids = root.path().join("aidlc/.aidlc-sessions/pids");
        write_pid(root.path(), "notes.txt", "ignored\n");
        // 存在しない大きな pid の記録は死んだものとして消える。
        write_pid(
            root.path(),
            "4000000000",
            r#"{"sessionId":"s-dead","startTime":null}"#,
        );
        write_pid(root.path(), "99999999", "{broken");
        write_ancestry(root.path(), "bad session");
        assert!(
            pids.join("4000000000").exists(),
            "文法外の session では掃除しない"
        );
        write_ancestry(root.path(), "s-live");
        assert!(pids.join("notes.txt").exists(), "数字でない名前は触らない");
        assert!(
            !pids.join("4000000000").exists(),
            "死んだ pid の記録は消える"
        );
        assert!(!pids.join("99999999").exists(), "読めない記録は消える");
        assert_eq!(
            resolve(root.path()).as_deref(),
            Some("s-live"),
            "書き足した祖先の記録から session を引ける"
        );
    }
}
