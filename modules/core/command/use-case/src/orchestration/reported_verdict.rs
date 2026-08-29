//! `ReportedVerdict` / `ReportedTransition` — `report` が受け取る正規化済みの入力。

use core_command_domain::orchestration::Verdict;

/// コンダクタが報告した結末 (正規化済み)。
///
/// ドメインの [`Verdict`] は「受理 10 語のどれだったか」の 6 分類であり、材料 (成果物列・
/// 人間入力・差し戻しフィードバック・読み飛ばし理由) を持たない。本型はその 6 分類に材料を
/// 貼り付けたもので、変種ごとに必要な材料だけを持つ — `Forward` に差し戻しフィードバックを
/// 添えるような組合せは**そもそも表現できない**。
///
/// **綴りの揺れの受理は本型の仕事ではない。** `approved` / `completed` / `complete` / `done` が
/// 同義であることは [`Verdict::parse`] が畳み、生の文字列から本型を組むのは合成ルート (U7) の
/// 責務である。ユースケースは正規化済みの型しか受け取らない。
///
/// # なぜ `Resumed` だけが外側にいるのか
///
/// `resume` / `resumed` は**集約に触れない**からである — 再開はコマンド名の提示
/// (ルーティング) だけで、遷移をコミットしない (`coding-rules/use-case-rules.md` §3
/// 「upstream 自体がこの規律で出来ている: resume 4 択はルーティングのみ」)。集約へ 1 コマンドを
/// 打つ 5 経路を [`ReportedTransition`] に括り出すことで、「再開は集約へ届かない」が型の事実に
/// なり、ユースケース側に到達不能な分岐が残らない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReportedVerdict {
    /// 集約へ 1 コマンドを打つ 5 経路。
    Transition(ReportedTransition),
    /// `resume` / `resumed` — ルーティングのみ。集約を再構成すらしない。
    Resumed,
}

impl ReportedVerdict {
    /// 材料を落として、ドメイン語彙の 6 分類へ射影する。
    #[must_use]
    pub const fn verdict(&self) -> Verdict {
        match self {
            ReportedVerdict::Transition(transition) => transition.verdict(),
            ReportedVerdict::Resumed => Verdict::Resume,
        }
    }
}

/// 集約へ打つ 1 コマンドと、その経路だけが運ぶ材料。
///
/// どの集約コマンドを打つかは**ここでは決まらない** — `Forward` がゲートの承認になるか
/// 非ゲートの完了になるかは、報告された語ではなくステージの性質で決まるので、集約の
/// `gated` クエリを見て [`super::ReportUseCase`] が選ぶ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReportedTransition {
    /// `awaiting-approval` — ゲートを開く。
    AwaitingApproval {
        /// レビュー対象の成果物パス列 (集約は検証せずイベントに載せる)。
        artifacts: Vec<String>,
    },
    /// `approved` / `completed` / `complete` / `done` — 前進。
    Forward {
        /// 承認時の人間入力 (逐語保持)。
        user_input: Option<String>,
    },
    /// `rejected` — ゲートでの差し戻し。
    Rejected {
        /// 差し戻しのフィードバック。
        feedback: Option<String>,
    },
    /// `revised` — 差し戻し後のゲート再入。
    Revised,
    /// `skipped` — ルーティングされたライフサイクル結末 (完了ではない)。
    Skipped {
        /// 読み飛ばす理由。
        reason: String,
    },
}

impl ReportedTransition {
    /// 材料を落として、ドメイン語彙の分類へ射影する。
    #[must_use]
    pub const fn verdict(&self) -> Verdict {
        match self {
            ReportedTransition::AwaitingApproval { .. } => Verdict::AwaitingApproval,
            ReportedTransition::Forward { .. } => Verdict::Forward,
            ReportedTransition::Rejected { .. } => Verdict::Rejected,
            ReportedTransition::Revised => Verdict::Revised,
            ReportedTransition::Skipped { .. } => Verdict::Skipped,
        }
    }
}
