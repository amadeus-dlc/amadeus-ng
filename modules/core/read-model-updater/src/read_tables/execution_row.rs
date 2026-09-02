//! `ExecutionRow` — `read_execution` の 1 行 (実行 1 本の現在状態)。

use chrono::SecondsFormat;
use core_command_domain::orchestration::{IntentExecution, StageIndex};

use super::spelling;
use super::stage_lookup::slug_at;

/// `read_execution` の 1 行。主キーは `execution_id`。
///
/// 値はすべて再生した [`IntentExecution`] のクエリの答えの写しである。`parked_active` と
/// `accepts_commands` は集約の**導出述語**であり、読取側が `status` と `parked_at` から
/// 組み直さなくてよいように列にしてある (裁定 §10-1 の非正規化)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionRow {
    execution_id: String,
    intent_id: String,
    status: String,
    cursor_index: Option<usize>,
    cursor_slug: Option<String>,
    parked_at_index: Option<usize>,
    parked_at_slug: Option<String>,
    parked_active: bool,
    accepts_commands: bool,
    autonomy: String,
    seq_nr: usize,
    last_updated_at: String,
    state_binding: String,
}

impl ExecutionRow {
    /// 実行の集約を 1 行へ写す (**この型の唯一の構築経路**)。
    #[must_use]
    pub fn of(execution: &IntentExecution) -> ExecutionRow {
        let cursor = execution.cursor();
        ExecutionRow {
            execution_id: execution.id().as_str().to_string(),
            intent_id: execution.intent_id().as_str().to_string(),
            status: spelling::status(execution.status()).to_string(),
            cursor_index: Some(cursor.to_usize()),
            cursor_slug: slug_at(execution, cursor),
            parked_at_index: execution.parked_at().map(StageIndex::to_usize),
            parked_at_slug: execution.parked_at().and_then(|at| slug_at(execution, at)),
            parked_active: execution.parked_active(),
            accepts_commands: execution.accepts_commands(),
            autonomy: execution.autonomy().as_state_field().to_string(),
            seq_nr: execution.seq_nr(),
            last_updated_at: execution
                .last_updated_at()
                .to_rfc3339_opts(SecondsFormat::Secs, true),
            state_binding: execution.state_binding().as_str().to_string(),
        }
    }

    /// 実行の識別子 (UUIDv7)。
    #[must_use]
    pub fn execution_id(&self) -> &str {
        &self.execution_id
    }

    /// この実行が対象にしている intent の識別子。
    #[must_use]
    pub fn intent_id(&self) -> &str {
        &self.intent_id
    }

    /// ワークフロー全体の 2 値 (`running` / `completed`)。
    #[must_use]
    pub fn status(&self) -> &str {
        &self.status
    }

    /// カーソル位置 (文書順の索引)。
    #[must_use]
    pub const fn cursor_index(&self) -> Option<usize> {
        self.cursor_index
    }

    /// カーソル位置の slug。
    #[must_use]
    pub fn cursor_slug(&self) -> Option<&str> {
        self.cursor_slug.as_deref()
    }

    /// park マーカーの位置 (無ければ NULL)。
    #[must_use]
    pub const fn parked_at_index(&self) -> Option<usize> {
        self.parked_at_index
    }

    /// park マーカー位置の slug (無ければ NULL)。
    #[must_use]
    pub fn parked_at_slug(&self) -> Option<&str> {
        self.parked_at_slug.as_deref()
    }

    /// park 分岐が発火する状態か (マーカー有 ∧ 位置一致)。
    #[must_use]
    pub const fn parked_active(&self) -> bool {
        self.parked_active
    }

    /// 状態遷移コマンドを受理する状態か。
    #[must_use]
    pub const fn accepts_commands(&self) -> bool {
        self.accepts_commands
    }

    /// 自律モードの綴り (`autonomous` / `gated`)。
    #[must_use]
    pub fn autonomy(&self) -> &str {
        &self.autonomy
    }

    /// 集約内の通番 (歴史がどこまで進んだか)。
    #[must_use]
    pub const fn seq_nr(&self) -> usize {
        self.seq_nr
    }

    /// 最終更新時刻 (RFC3339 / 秒精度 / `Z`)。
    #[must_use]
    pub fn last_updated_at(&self) -> &str {
        &self.last_updated_at
    }

    /// 実行状態の束縛ダイジェスト (`h`)。
    #[must_use]
    pub fn state_binding(&self) -> &str {
        &self.state_binding
    }
}
