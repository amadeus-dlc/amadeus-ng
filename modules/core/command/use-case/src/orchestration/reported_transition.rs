//! `ReportedTransition` — `CommitVerdictUseCase` が受け取る正規化済みの入力。

use core_command_domain::orchestration::Verdict;

/// コンダクタが報告した結末（正規化済み）と、その結末だけが運ぶ材料。
///
/// ドメインの [`Verdict`] は「受理 10 語のどれだったか」の分類であり、材料（成果物列・
/// 人間入力・差し戻しフィードバック・読み飛ばし理由）を持たない。本型はその分類に材料を
/// 貼り付けたもので、変種ごとに必要な材料だけを持つ — `Forward` に差し戻しフィードバックを
/// 添えるような組合せは**そもそも表現できない**。
///
/// **綴りの揺れの受理は本型の仕事ではない。** `approved` / `completed` / `complete` / `done` が
/// 同義であることは [`Verdict::parse`] が畳み、生の文字列から本型を組むのは合成ルート（U7）の
/// 責務である。ユースケースは正規化済みの型しか受け取らない。
///
/// # `resume` / `resumed` はここに無い
///
/// 再開は**遷移をコミットしない**。コマンド名を提示するだけのルーティングであり、その分岐は
/// Controller（U7）が**ユースケースへ届く手前で**行う
/// （`coding-rules/use-case-rules.md` §3「resume 4 択はルーティング（コマンド名の提示）のみ」
/// 「`--single` は Report が呼ぶのではなく Controller が手前で分岐する」）。upstream も
/// `handleReport` が `RESUME_RESULTS` を早期に `handleResumeReport` へ振り分けており、同じ構造で
/// ある。したがって本型は**集約へ 1 コマンドを打つ 5 経路だけ**を持つ。
///
/// どの集約コマンドを打つかは**ここでは決まらない** — `Forward` がゲートの承認になるか
/// 非ゲートの完了になるかは、報告された語ではなくステージの性質で決まるので、集約の
/// `gated` クエリを見て [`super::CommitVerdictUseCase`] が選ぶ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReportedTransition {
    /// `awaiting-approval` — ゲートを開く。
    AwaitingApproval {
        /// レビュー対象の成果物パス列（集約は検証せずイベントに載せる）。
        artifacts: Vec<String>,
    },
    /// `approved` / `completed` / `complete` / `done` — 前進。
    Forward {
        /// 承認時の人間入力（逐語保持）。
        user_input: Option<String>,
    },
    /// `rejected` — ゲートでの差し戻し。
    Rejected {
        /// 差し戻しのフィードバック。
        feedback: Option<String>,
    },
    /// `revised` — 差し戻し後のゲート再入。
    Revised,
    /// `skipped` — ルーティングされたライフサイクル結末（完了ではない）。
    Skipped {
        /// 読み飛ばす理由。
        reason: String,
    },
}

impl ReportedTransition {
    /// 材料を落として、ドメイン語彙の分類へ射影する。
    ///
    /// `Verdict` は 6 分類あるが、本型が射影しうるのは遷移をコミットする 5 つだけである
    /// （`Verdict::Resume` は U7 のルーティングで消費され、ここまで来ない）。
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
