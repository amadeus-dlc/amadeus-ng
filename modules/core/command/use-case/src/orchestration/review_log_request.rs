//! `ReviewLogRequest` — `RecordReviewUseCase` の入力（正規化済み）。

use core_command_domain::workflow_definition::StageSlug;

use super::review_log_kind::ReviewLogKind;

/// `aidlc-log review` 1 回分の入力。
///
/// 構文段（フラグの有無・値の必須・閉集合）は合成ルートが通し終えているので、ここに届く
/// のは**正規化済みの値**だけである（`coding-rules/use-case-rules.md` — 入力は型付きの値で
/// 受ける）。`--unit` / `--single` は本 build では未配線なので運ばない（設計 §1 の繰延）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewLogRequest {
    stage: StageSlug,
    reviewer: String,
    iteration: u32,
    kind: ReviewLogKind,
}

impl ReviewLogRequest {
    /// 4 材料を束ねる（**この型の唯一の構築経路**）。
    #[must_use]
    pub fn new(
        stage: StageSlug,
        reviewer: impl Into<String>,
        iteration: u32,
        kind: ReviewLogKind,
    ) -> ReviewLogRequest {
        ReviewLogRequest {
            stage,
            reviewer: reviewer.into(),
            iteration,
            kind,
        }
    }

    /// レビュー対象のステージ。
    #[must_use]
    pub const fn stage(&self) -> &StageSlug {
        &self.stage
    }

    /// 名指されたレビュアー。
    #[must_use]
    pub fn reviewer(&self) -> &str {
        &self.reviewer
    }

    /// 通し番号（正の整数であることは合成ルートが確かめている）。
    #[must_use]
    pub const fn iteration(&self) -> u32 {
        self.iteration
    }

    /// 書こうとしている行の種類。
    #[must_use]
    pub const fn kind(&self) -> ReviewLogKind {
        self.kind
    }
}
