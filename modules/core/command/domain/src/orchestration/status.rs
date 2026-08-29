//! `Status` — 状態ファイル `Status` 行の 2 値 (park マーカーとは直交)。

/// ワークフロー全体の 2 値。
///
/// park マーカー (`parked_at`) とは**直交**するので、これだけでは「今コマンドを受け付けるか」
/// は決まらない (判定は `IntentExecution::accepts_commands` — BR1.0)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Status {
    /// 進行中 — スコープ内に未決着のステージが残っている。
    Running,
    /// スコープ内の最後のステージまで決着済み。以後、状態遷移コマンドは `NotRunning` で
    /// 拒否される。
    Completed,
}

impl Status {
    /// 進行中か (park マーカーは見ない — 受理判定は `accepts_commands`)。
    #[must_use]
    pub fn is_running(self) -> bool {
        self == Status::Running
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_two_values_are_distinct() {
        assert_ne!(Status::Running, Status::Completed);
        assert_eq!(Status::Running, Status::Running);
    }

    #[test]
    fn running_is_the_only_value_that_accepts_progress() {
        assert!(Status::Running.is_running());
        assert!(!Status::Completed.is_running());
    }
}
