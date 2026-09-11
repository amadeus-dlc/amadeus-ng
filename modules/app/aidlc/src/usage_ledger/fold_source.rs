//! 畳み込む 1 つの会話履歴ファイル — main か、sub-agent の sidecar 付きファイルか。
//!
//! cursor の鍵はファイルパスである。台帳は session を跨いで積み上がり、session ごとに
//! ファイルが違うので、`"main"` のような定数を鍵にすると別 session の位置を当ててしまう
//! （本家 `foldFileIntoLedger` の CURSOR KEY）。
use harness_claude::TranscriptUsageRow;
use std::path::{Path, PathBuf};

/// 1 ファイル分の畳み込み指示。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FoldSource {
    path: PathBuf,
    cursor_key: String,
    agent: Option<(String, Option<String>)>,
    close_last_group: bool,
}

impl FoldSource {
    const fn new(
        path: PathBuf,
        cursor_key: String,
        agent: Option<(String, Option<String>)>,
        close_last_group: bool,
    ) -> Self {
        Self {
            path,
            cursor_key,
            agent,
            close_last_group,
        }
    }

    /// main の会話履歴。cursor の鍵は封筒が運んだパスの逐語。
    pub(crate) fn main(transcript: &str, close_last_group: bool) -> Self {
        Self::new(
            PathBuf::from(transcript),
            transcript.to_string(),
            None,
            close_last_group,
        )
    }

    /// sub-agent の会話履歴。`agent_type` は sidecar `agent-<id>.meta.json` の `agentType`。
    pub(crate) fn subagent(
        path: PathBuf,
        agent_id: String,
        agent_type: Option<String>,
        close_last_group: bool,
    ) -> Self {
        let cursor_key = path.to_string_lossy().into_owned();
        Self::new(
            path,
            cursor_key,
            Some((agent_id, agent_type)),
            close_last_group,
        )
    }

    /// 読むファイル。
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    /// `cursors` の鍵。
    pub(crate) fn cursor_key(&self) -> &str {
        &self.cursor_key
    }

    /// 行が自分の `agentId` を持たないときの持ち主（sub-agent のファイル名から来る）。
    pub(crate) fn fallback_agent_id(&self) -> Option<&str> {
        self.agent.as_ref().map(|(agent_id, _)| agent_id.as_str())
    }

    /// この呼出しで最後の群を締めるか。
    pub(crate) const fn close_last_group(&self) -> bool {
        self.close_last_group
    }

    /// `byAgent` の鍵。main は `main`、sub-agent は sidecar の種別、無ければ `subagent`。
    pub(crate) fn agent_bucket(&self, row: &TranscriptUsageRow) -> String {
        match &self.agent {
            None => "main".to_string(),
            Some((agent_id, agent_type)) => {
                if row.agent_id() == Some(agent_id.as_str()) {
                    agent_type.clone().unwrap_or_else(|| "subagent".to_string())
                } else {
                    "subagent".to_string()
                }
            }
        }
    }
}
