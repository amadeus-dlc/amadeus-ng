//! `ReportOutcome` — `ReportUseCase` が返す型付きの結果。

use core_command_domain::orchestration::{NextDecision, WorkflowExecutionEvent};
use core_command_domain::workflow_definition::StageSlug;

/// `report` が何をしたかの型付きの結果。
///
/// **文言を持たない。** 「Committed approve for "..." (scope: ...)」のような利用者向けの逐語は
/// 出す側 (合成ルート U7 の Presenter) が組む — 文言はそれを出す側の持ち物である
/// (`coding-rules/error-handling.md` §対象外、2026-08-29 の message-catalog 解体)。ここに
/// あるのは、その文言を組むのに要る**材料**だけである。
///
/// コミットの有無が変種で割れているのが本型の要点である — upstream の `report` は 3 経路で
/// 何も書かない (既に開いているゲートへの再報告・カーソル通過済みステージへの再報告・
/// 再開のルーティング) ので、それを `Committed` と同じ形で返すと呼出側が見分けられない。
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(
    clippy::large_enum_variant,
    reason = "`Committed` はドメインイベントを丸ごと運ぶので他の変種より大きい。ワンショット \
              CLI が 1 回の起動につき 1 個だけ作る値であり、`Box` 化して得られるのは呼出側の \
              デリファレンスと 1 回のヒープ確保だけである (材料をそのまま返すという本型の \
              目的も濁る)。"
)]
pub enum ReportOutcome {
    /// 遷移をコミットした — 集約が返した単一のドメインイベント。
    ///
    /// どのサブコマンドを打ったか (承認か完了か差し戻しか) は変種そのものが語るので、
    /// 別立ての識別子は持たない。
    Committed {
        /// コミットしたドメインイベント。
        event: WorkflowExecutionEvent,
    },
    /// 既に承認ゲートが開いていたので**何もコミットしなかった**。
    ///
    /// upstream の `cli/report/awaiting-approval-repeat` は監査行も状態差分も空である
    /// (ゲート根拠の再検証だけが起きる)。
    GateAlreadyOpen {
        /// 既に開いていたステージ。
        stage: StageSlug,
    },
    /// カーソルが通過済みの completed ステージへの再報告 — 冪等な done (BR1.9)。
    ///
    /// 集約の `stale_report` が下した判断をそのまま運ぶ。何もコミットしない。
    AlreadyDone {
        /// 再報告されたステージ。
        stage: StageSlug,
        /// 集約が返した停止の理由。
        decision: NextDecision,
    },
    /// `resume` / `resumed` — ルーティングのみ。集約を再構成すらしていない。
    ResumeRouting,
}
