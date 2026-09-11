//! main の会話履歴と、その兄弟の sub-agent ファイル群。
//!
//! 配置は `<projects>/<slug>/<session>.jsonl` が main、
//! `<projects>/<slug>/<session>/subagents/agent-<agentId>.jsonl` が sub-agent
//! （本家 `subagentDir` / `readMetaSidecar` / `foldTranscriptIntoLedger`）。
use super::fold_source::FoldSource;
use harness_claude::FoldMode;
use std::path::{Path, PathBuf};

/// 1 session 分の会話履歴の所在。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TranscriptSession {
    transcript: String,
}

impl TranscriptSession {
    /// 封筒が運んだ main のパスから組む。
    pub(crate) fn new(transcript: &str) -> Self {
        Self {
            transcript: transcript.to_string(),
        }
    }

    /// main と、見つかった sub-agent ファイルを畳む順（main → ファイル名順）に並べる。
    pub(crate) fn sources(&self, mode: FoldMode) -> Vec<FoldSource> {
        let mut sources = vec![FoldSource::main(&self.transcript, mode.seals_main())];
        let directory = subagent_dir(&self.transcript);
        let mut names: Vec<String> = std::fs::read_dir(&directory)
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .map(|entry| entry.file_name().to_string_lossy().into_owned())
                    .filter(|name| name.starts_with("agent-") && name.ends_with(".jsonl"))
                    .collect()
            })
            .unwrap_or_default();
        names.sort();
        for name in names {
            let agent_id = name
                .strip_prefix("agent-")
                .and_then(|rest| rest.strip_suffix(".jsonl"))
                .unwrap_or_default()
                .to_string();
            let path = directory.join(&name);
            sources.push(FoldSource::subagent(
                path.clone(),
                agent_id,
                sidecar_agent_type(&path),
                mode.seals_subagents(),
            ));
        }
        sources
    }
}

/// sidecar `agent-<id>.meta.json` の `agentType`。無い・壊れている・空なら `None`。
fn sidecar_agent_type(jsonl: &Path) -> Option<String> {
    let name = jsonl.file_name()?.to_string_lossy().into_owned();
    let meta = jsonl.with_file_name(format!(
        "{}.meta.json",
        name.strip_suffix(".jsonl").unwrap_or(&name)
    ));
    let bytes = std::fs::read(meta).ok()?;
    let value: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    value
        .get("agentType")
        .and_then(serde_json::Value::as_str)
        .filter(|agent_type| !agent_type.is_empty())
        .map(str::to_string)
}

/// `<dir>/<session>/subagents`（`<session>` は main のファイル名から `.jsonl` を除いたもの）。
fn subagent_dir(transcript: &str) -> PathBuf {
    let main = Path::new(transcript);
    let name = main
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let session = name.strip_suffix(".jsonl").unwrap_or(&name);
    match main
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        Some(parent) => parent.join(session).join("subagents"),
        None => PathBuf::from(session).join("subagents"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_session_lists_main_then_each_sidecar_in_name_order() {
        let directory = tempfile::tempdir().expect("一時ディレクトリ");
        let main = directory.path().join("s.jsonl");
        std::fs::write(&main, "").expect("テストのファイル");
        let sub = directory.path().join("s/subagents");
        std::fs::create_dir_all(&sub).expect("テストのディレクトリ");
        std::fs::write(sub.join("agent-two.jsonl"), "").expect("テストのファイル");
        std::fs::write(sub.join("agent-one.jsonl"), "").expect("テストのファイル");
        std::fs::write(
            sub.join("agent-one.meta.json"),
            r#"{"agentType":"aidlc-developer-agent"}"#,
        )
        .expect("テストのファイル");
        std::fs::write(sub.join("agent-three.meta.json"), r#"{"agentType":"x"}"#)
            .expect("テストのファイル");
        std::fs::write(sub.join("other.jsonl"), "").expect("テストのファイル");
        let main_text = main.to_string_lossy().into_owned();
        let sources = TranscriptSession::new(&main_text).sources(FoldMode::SealMain);
        assert_eq!(
            sources,
            vec![
                FoldSource::main(&main_text, true),
                FoldSource::subagent(
                    sub.join("agent-one.jsonl"),
                    "one".into(),
                    Some("aidlc-developer-agent".into()),
                    false
                ),
                FoldSource::subagent(sub.join("agent-two.jsonl"), "two".into(), None, false),
            ]
        );
        let flushed = TranscriptSession::new(&main_text).sources(FoldMode::FlushAll);
        assert!(flushed.iter().all(FoldSource::close_last_group));
        let held = TranscriptSession::new(&main_text).sources(FoldMode::Holdback);
        assert!(!held.iter().any(FoldSource::close_last_group));
    }

    #[test]
    fn a_transcript_without_siblings_is_just_main() {
        let sources = TranscriptSession::new("/nowhere/x.jsonl").sources(FoldMode::Holdback);
        assert_eq!(sources, vec![FoldSource::main("/nowhere/x.jsonl", false)]);
        assert_eq!(
            subagent_dir("/a/b/c.jsonl"),
            PathBuf::from("/a/b/c/subagents")
        );
        assert_eq!(subagent_dir("c.jsonl"), PathBuf::from("c/subagents"));
    }
}
