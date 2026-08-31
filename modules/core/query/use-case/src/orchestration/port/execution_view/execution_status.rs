//! `ExecutionStatus` — リードモデルの `- **Status**:` 行の 2 値 (park マーカーとは直交)。

use crate::orchestration::UnknownValue;

/// ワークフロー全体の 2 値。
///
/// park マーカー ([`super::ExecutionStateView::parked_at`]) とは**直交**するので、これだけで
/// 「今コマンドを受け付けるか」は決まらない (判定は
/// [`super::ExecutionStateView::accepts_commands`] — BR1.0)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExecutionStatus {
    /// 進行中 — スコープ内に未決着のステージが残っている。
    Running,
    /// スコープ内の最後のステージまで決着済み。
    Completed,
}

impl ExecutionStatus {
    /// リードモデル上の逐語綴り (状態テンプレートの `[Running/Completed]`)。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            ExecutionStatus::Running => "Running",
            ExecutionStatus::Completed => "Completed",
        }
    }

    /// リードモデルの綴りから引き当てる。
    ///
    /// # Errors
    ///
    /// 閉集合 2 語以外は [`UnknownValue`] (`Unknown` 変種へ逃がさない — b25 の閉集合方針)。
    pub fn parse(s: &str) -> Result<ExecutionStatus, UnknownValue> {
        match s {
            "Running" => Ok(ExecutionStatus::Running),
            "Completed" => Ok(ExecutionStatus::Completed),
            other => Err(UnknownValue::new(other)),
        }
    }

    /// 進行中か (park マーカーは見ない — 受理判定は `accepts_commands`)。
    #[must_use]
    pub fn is_running(self) -> bool {
        self == ExecutionStatus::Running
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_two_values_round_trip() {
        for status in [ExecutionStatus::Running, ExecutionStatus::Completed] {
            assert_eq!(ExecutionStatus::parse(status.as_str()), Ok(status));
        }
    }

    #[test]
    fn an_unknown_spelling_is_rejected_with_its_material() {
        let error = ExecutionStatus::parse("Parked").unwrap_err();
        assert_eq!(error.as_str(), "Parked");
        assert_eq!(error.to_string(), "unknown value \"Parked\"");
    }

    #[test]
    fn only_running_is_running() {
        assert!(ExecutionStatus::Running.is_running());
        assert!(!ExecutionStatus::Completed.is_running());
    }
}
