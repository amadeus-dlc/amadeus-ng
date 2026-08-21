//! `ShardName` — `<host>-<cloneId>.md`。host は小文字化・`[a-z0-9-]` 以外の連続を `-` へ圧縮・
//! trim・48 文字上限・空なら `"host"` (upstream `aidlc-lib.ts:4499`, 03 §3.3)。

use super::clone_id::CloneId;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ShardName(String);

impl ShardName {
    /// ホスト名正規化を含む唯一の構成関数 (E1 — 構成関数以外でこの型は作れない)。
    pub fn compose(raw_host: &str, clone_id: &CloneId) -> ShardName {
        let host = normalize_host(raw_host);
        ShardName(format!("{host}-{}.md", clone_id.as_str()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ShardName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

fn normalize_host(raw: &str) -> String {
    let mut out = String::new();
    let mut pending_dash = false;
    for c in raw.chars() {
        let c = c.to_ascii_lowercase();
        if c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' {
            if c == '-' {
                pending_dash = !out.is_empty();
            } else {
                if pending_dash {
                    out.push('-');
                    pending_dash = false;
                }
                out.push(c);
            }
        } else {
            // 非許容文字の連続は 1 つの区切りに潰す
            pending_dash = !out.is_empty();
        }
    }
    out.truncate(48);
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() { "host".to_string() } else { out }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cid() -> CloneId {
        CloneId::parse("abc123").unwrap()
    }

    #[test]
    fn lowercases_and_squashes_disallowed_runs_into_single_dashes() {
        assert_eq!(ShardName::compose("My Mac.local", &cid()).as_str(), "my-mac-local-abc123.md");
        assert_eq!(ShardName::compose("dev__box!!01", &cid()).as_str(), "dev-box-01-abc123.md");
    }

    #[test]
    fn empty_or_fully_disallowed_hosts_fall_back_to_host() {
        assert_eq!(ShardName::compose("", &cid()).as_str(), "host-abc123.md");
        assert_eq!(ShardName::compose("!!!", &cid()).as_str(), "host-abc123.md");
    }

    #[test]
    fn caps_the_host_part_at_48_chars() {
        let long = "h".repeat(60);
        let name = ShardName::compose(&long, &cid());
        assert_eq!(name.as_str(), format!("{}-abc123.md", "h".repeat(48)));
    }
}
