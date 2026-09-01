//! `Directive` — `next` / `continue` が放出する判別共用体 (公開言語 B14)。
//!
//! upstream `validateDirective` 相当の検証は**型で**行う — kind ごとの typed variant なので
//! 未知キー・型違反・cross-field 違反は構成不能である (E1+E2)。ワイヤ JSON への直列化と
//! 28KiB 上限 (超過は emit 拒否 — half-emitted を出さない) は Presenter (U7) の責務で、
//! ここには持ち込まない。
//!
//! placeholder 2 種 (`dispatch-subagent` / `present-gate`) と slice 2 の `invoke-swarm` は
//! variant を**持たない** — 「エンジンは今日これを構築しない」を構成不能で表す
//! (投機実装禁止 — 02 §4.1)。[`DirectiveKind`] は 10 種の閉集合 (ワイヤ判別子のカタログ) の
//! ままで、この共用体は**構築できる部分集合**である。

use super::ask_directive::AskDirective;
use super::directive_schema::DirectiveKind;
use super::load_steering_directive::LoadSteeringDirective;
use super::run_stage_directive::RunStageDirective;
use crate::orchestration::StageSlugView;

/// 放出できる directive の判別共用体。
#[allow(
    clippy::large_enum_variant,
    reason = "run-stage が最大の payload を持つのは公開言語の形そのもの — Box で包むと \
              消費側のパターンが崩れる。directive は 1 ターン 1 個しか生成しない"
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Directive {
    /// ルール束の分割配信 1 部。
    LoadSteering(LoadSteeringDirective),
    /// ステージ本体の実行。
    RunStage(RunStageDirective),
    /// 構造化質問の提示。
    Ask(AskDirective),
    /// 逐語で印字して停止、または名指しのコマンド実行 (print の 3 形)。
    Print {
        /// 逐語メッセージ (コマンドの名指しを含む)。
        message: String,
    },
    /// エラーで停止。`message` はユーザへ逐語で見せる。
    Error {
        /// 逐語メッセージ。
        message: String,
    },
    /// ループの停止 (完了・エピローグ・冪等な終端)。
    Done {
        /// 理由 (省略可 — 逐語)。
        reason: Option<String>,
    },
    /// park 済みワークフローでの停止。
    Parked {
        /// park している位置。
        stage: StageSlugView,
        /// 逐語メッセージ (`Workflow parked at ...`)。
        message: String,
    },
}

impl Directive {
    /// ワイヤ判別子。
    #[must_use]
    pub const fn kind(&self) -> DirectiveKind {
        match self {
            Directive::LoadSteering(_) => DirectiveKind::LoadSteering,
            Directive::RunStage(_) => DirectiveKind::RunStage,
            Directive::Ask(_) => DirectiveKind::Ask,
            Directive::Print { .. } => DirectiveKind::Print,
            Directive::Error { .. } => DirectiveKind::Error,
            Directive::Done { .. } => DirectiveKind::Done,
            Directive::Parked { .. } => DirectiveKind::Parked,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::ask_kind::AskKind;
    use super::super::gate_field::GateField;
    use super::super::run_stage_directive::RunStageDirectiveBuilder;
    use super::*;
    use crate::orchestration::{PhaseView, StageModeView};

    fn slug() -> StageSlugView {
        StageSlugView::parse("requirements-analysis").unwrap()
    }

    #[test]
    fn every_constructible_variant_names_its_serialized_kind() {
        let run_stage = RunStageDirectiveBuilder::new(
            slug(),
            PhaseView::Inception,
            "aidlc-product-agent",
            StageModeView::Inline,
            GateField::Gated,
            "stages/inception/requirements-analysis.md",
            "record/inception/requirements-analysis/memory.md",
        )
        .build();
        assert_eq!(
            Directive::RunStage(run_stage).kind(),
            DirectiveKind::RunStage
        );
        assert_eq!(
            Directive::Ask(AskDirective::new(
                AskKind::ResumeMenu,
                "How would you like to proceed?".to_string()
            ))
            .kind(),
            DirectiveKind::Ask
        );
        assert_eq!(
            Directive::Print {
                message: "aidlc-utility status".to_string()
            }
            .kind(),
            DirectiveKind::Print
        );
        assert_eq!(
            Directive::Error {
                message: "boom".to_string()
            }
            .kind(),
            DirectiveKind::Error
        );
        assert_eq!(Directive::Done { reason: None }.kind(), DirectiveKind::Done);
        assert_eq!(
            Directive::Parked {
                stage: slug(),
                message: "Workflow parked".to_string()
            }
            .kind(),
            DirectiveKind::Parked
        );
    }
}
